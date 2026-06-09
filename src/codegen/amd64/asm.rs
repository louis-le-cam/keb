use std::collections::HashSet;

use crate::{
    codegen::amd64::allocation::{Allocation, Allocations},
    key_vec::{
        KeyVec, Sentinels,
        Value::{Item, Sentinel},
    },
    semantic::{Type, TypeData, TypeSentinel, Types},
    ssa::{Block, BlockData, BlockSentinel, ConstData, ConstSentinel, Expr, Inst, InstData, Ssa},
};

pub fn generate(types: &Types, ssa: &Ssa, allocations: &Allocations) -> String {
    let mut generator = Generator {
        types,
        ssa,
        allocations,
        blocks: KeyVec::from_vec((0..ssa.blocks.len()).map(|_| String::new()).collect()),
    };

    generator.generate();

    generator.result()
}

struct Generator<'a> {
    types: &'a Types,
    ssa: &'a Ssa,
    allocations: &'a Allocations,
    blocks: KeyVec<BlockSentinel, String>,
}

impl Generator<'_> {
    fn result(self) -> String {
        let mut asm = String::new();

        asm.push_str(".code64\n");
        asm.push_str(".global main\n\n");
        asm.extend(self.blocks.entries().flat_map(|(_, asm)| [asm, "\n"]));

        asm
    }

    fn generate(&mut self) {
        for (block, block_data) in self.ssa.blocks.entries() {
            match block_data {
                BlockData::ExternFunction { name, .. } => {
                    self.blocks[block] = format!(".set f{}_{name}, {name}\n", block.as_u32());
                }
                BlockData::Function { .. } => self.generate_function(block),
                BlockData::Block { .. } => self.generate_block(block),
            }
        }
    }

    fn inst_block(&self, inst: Inst) -> Block {
        self.ssa
            .blocks
            .entries()
            .find(|(_, block_data)| match block_data {
                BlockData::ExternFunction { .. } => false,
                BlockData::Function { insts, .. } => insts.contains(&inst),
                BlockData::Block { insts, .. } => insts.contains(&inst),
            })
            .unwrap()
            .0
    }

    fn block_children(&self, block: Block) -> impl Iterator<Item = Block> {
        let children = match &self.ssa.blocks[block] {
            BlockData::ExternFunction { .. } => [None, None],
            BlockData::Function { insts, .. } | BlockData::Block { insts, .. } => {
                match insts.last().map(|inst| &self.ssa.insts[*inst]) {
                    Some(InstData::Jump {
                        block: destination, ..
                    }) => [Some(*destination), None],
                    Some(InstData::JumpCondition { then, else_, .. }) => {
                        [Some(*then), Some(*else_)]
                    }
                    _ => [None, None],
                }
            }
        };

        children.into_iter().filter_map(|block| block)
    }

    fn block_descendants(&self, block: Block) -> impl Iterator<Item = Block> {
        let mut blocks = Vec::from([block]);
        let mut visited = HashSet::new();

        while let Some(child) = blocks.pop() {
            visited.insert(child);

            blocks.extend(
                self.block_children(child)
                    .filter(|descendant| visited.contains(descendant)),
            );
        }

        visited.remove(&block);
        visited.into_iter()
    }

    fn block_parent_instructions(&self, block: Block) -> impl Iterator<Item = Inst> {
        self.ssa
            .insts
            .entries()
            .filter(move |(_, inst_data)| match inst_data {
                InstData::Jump {
                    block: destination, ..
                } => *destination == block,
                InstData::JumpCondition { then, else_, .. } => *then == block || *else_ == block,
                _ => false,
            })
            .map(|(inst, _)| inst)
    }

    fn stack_bottom(&self, allocations: &[Allocation]) -> u64 {
        allocations
            .iter()
            .filter_map(|allocation| match allocation {
                Allocation::Stack { offset, size: _ } => Some(*offset),
                _ => None,
            })
            .max()
            .unwrap_or(0)
    }

    fn used_allocations(&self, inst: Inst) -> Vec<Allocation> {
        let block = self.inst_block(inst);
        let function = self.block_function(block);

        let mut allocations = self.used_allocations_inner(inst);

        allocations.push(self.allocations.arguments[function]);
        for block in self.block_descendants(function) {
            allocations.push(self.allocations.arguments[block]);
        }

        allocations
    }

    /// This function does not take block argument into account, see [`Self::used_allocations`]
    fn used_allocations_inner(&self, inst: Inst) -> Vec<Allocation> {
        let block = self.inst_block(inst);

        let insts = match &self.ssa.blocks[block] {
            BlockData::Function { insts, .. } | BlockData::Block { insts, .. } => insts,
            BlockData::ExternFunction { .. } => unreachable!(),
        };

        let mut allocations = self
            .block_parent_instructions(block)
            .fold(HashSet::new(), |mut set, parent_inst| {
                set.extend(self.used_allocations(parent_inst));
                set
            })
            .into_iter()
            .collect::<Vec<_>>();

        for block_inst in insts {
            if *block_inst == inst {
                break;
            }

            let inst_allocations = &self.allocations.instructions[*block_inst];

            for inst in &inst_allocations.deallocations {
                let allocation = self.allocations.instructions[*inst].allocation;
                // TODO: Maybe identify allocation by id instead?
                // That would make de-allocation simpler and more correct?
                allocations.retain(|alloc| &allocation != alloc);
            }

            for (previous, new) in &inst_allocations.reallocations {
                // TODO: Maybe identify allocation by id instead?
                // That would make de-allocation simpler and more correct?
                allocations.retain(|alloc| previous != alloc);
                allocations.push(*new);
            }

            if inst_allocations.allocation != Allocation::Unit {
                allocations.push(inst_allocations.allocation);
            }
        }

        allocations
    }

    fn generate_function(&mut self, function: Block) {
        let BlockData::Function { name, insts, .. } = &self.ssa.blocks[function] else {
            panic!()
        };

        let mut asm = String::new();

        if name == "main" {
            asm.push_str(&format!(".set main, f{}_{name}\n", function.as_u32()));
        }

        asm.push_str(&format!("f{}_{name}:\n", function.as_u32()));

        asm.push_str("  push %rbp\n");
        asm.push_str("  mov %rsp, %rbp\n\n");

        for inst in insts {
            asm.push_str(&self.inst_asm(*inst));
        }

        self.blocks[function] = asm;
    }

    fn inst_asm(&mut self, inst: Inst) -> String {
        match &self.ssa.insts[inst] {
            InstData::Field(expr, field) => {
                let allocation = self.expr_allocation(*expr);

                let expr_type = self.expr_type(*expr);

                let field_offset = match self.types.get(expr_type) {
                    Sentinel(_) => panic!(),
                    Item(type_data) => match type_data {
                        TypeData::Function { .. } => panic!(),
                        TypeData::Product { fields } => fields[0..*field as usize]
                            .iter()
                            .fold(0, |acc, (_, field_type)| acc + self.type_size(*field_type)),
                    },
                };

                let field_type = match self.types.get(expr_type) {
                    Sentinel(_) => panic!(),
                    Item(type_data) => match type_data {
                        TypeData::Function { .. } => panic!(),
                        TypeData::Product { fields } => fields[*field as usize].1,
                    },
                };

                let field_size = self.type_size(field_type);
                let source_allocation = allocation.offset(field_offset, field_size);

                source_allocation.move_to(&self.allocations.instructions[inst].allocation)
            }
            InstData::Record(fields, _) => {
                let allocation = self.allocations.instructions[inst].allocation;

                let mut inst_asm = String::new();

                let mut offset = 0;
                for field in fields {
                    let field_allocation = self.expr_allocation(*field);
                    let field_size = self.type_size(self.expr_type(*field));
                    inst_asm.push_str(
                        &field_allocation.move_to(&allocation.offset(offset, field_size)),
                    );

                    offset += field_size;
                }

                inst_asm
            }
            InstData::Equal(lhs, rhs) => {
                let lhs_allocation = self.expr_allocation(*lhs);
                let rhs_allocation = self.expr_allocation(*rhs);

                let allocation = self.allocations.instructions[inst].allocation;

                let inst_number = inst.as_u32();

                format!(
                    "{}  cmp {}, {}\n  je i{inst_number}_equal\n{}  jmp i{inst_number}_end\ni{inst_number}_equal:{}\ni{inst_number}_end:\n\n",
                    lhs_allocation.move_to(&allocation),
                    allocation.asm().unwrap(),
                    rhs_allocation.asm().unwrap(),
                    Allocation::Immediate(0).move_to(&allocation),
                    Allocation::Immediate(1).move_to(&allocation),
                )
            }
            InstData::Add(lhs, rhs) => {
                let lhs_allocation = self.expr_allocation(*lhs);
                let rhs_allocation = self.expr_allocation(*rhs);

                let allocation = self.allocations.instructions[inst].allocation;

                format!(
                    "{}  add {}, {}\n",
                    lhs_allocation.move_to(&allocation),
                    rhs_allocation.asm().unwrap(),
                    allocation.asm().unwrap(),
                )
            }
            InstData::Sub(lhs, rhs) => {
                let lhs_allocation = self.expr_allocation(*lhs);
                let rhs_allocation = self.expr_allocation(*rhs);

                let allocation = self.allocations.instructions[inst].allocation;

                format!(
                    "{}  sub {}, {}\n",
                    lhs_allocation.move_to(&allocation),
                    rhs_allocation.asm().unwrap(),
                    allocation.asm().unwrap(),
                )
            }
            InstData::Mul(lhs, rhs) => {
                let lhs_allocation = self.expr_allocation(*lhs);
                let rhs_allocation = self.expr_allocation(*rhs);

                assert_eq!(
                    self.allocations.instructions[inst].allocation,
                    Allocation::Eax
                );

                format!(
                    "{}  mul {}\n",
                    lhs_allocation.move_to(&Allocation::Eax),
                    rhs_allocation.asm().unwrap(),
                )
            }
            InstData::Div(lhs, rhs) => {
                let lhs_allocation = self.expr_allocation(*lhs);
                let rhs_allocation = self.expr_allocation(*rhs);

                assert_eq!(
                    self.allocations.instructions[inst].allocation,
                    Allocation::Eax
                );

                format!(
                    "{}{}{}  div {}\n",
                    rhs_allocation.move_to(&Allocation::Ebx),
                    lhs_allocation.move_to(&Allocation::Eax),
                    Allocation::Immediate(0).move_to(&Allocation::Edx),
                    Allocation::Ebx.asm().unwrap(),
                )
            }
            InstData::Call { function, argument } => {
                let mut inst_asm = "\n".to_string();

                let used_allocations = self.used_allocations(inst);
                let stack_size = self.stack_bottom(&used_allocations);

                inst_asm.push_str(&format!("  sub ${}, %rsp\n", stack_size));
                inst_asm.push_str("  push %rax\n");
                inst_asm.push_str("  push %rbx\n");
                inst_asm.push_str("  push %rcx\n");
                inst_asm.push_str("  push %rdx\n");
                inst_asm.push_str("  push %rsi\n");

                let (argument_type, return_type) = match self.ssa.blocks[*function] {
                    BlockData::ExternFunction { arg, ret, .. }
                    | BlockData::Function { arg, ret, .. } => (arg, ret),
                    BlockData::Block { .. } => panic!(),
                };

                let (argument_allocation, return_allocation) =
                    self.other_function_allocations(argument_type, return_type, stack_size);

                let allocation = self.expr_allocation(*argument);
                inst_asm.push_str(&allocation.move_to(&argument_allocation));

                let argument_size = argument_allocation.size();

                let function_name = match &self.ssa.blocks[*function] {
                    BlockData::ExternFunction { name, .. } | BlockData::Function { name, .. } => {
                        name
                    }
                    BlockData::Block { .. } => panic!(),
                };

                inst_asm.push_str(&format!("  sub ${argument_size}, %rsp\n"));
                inst_asm.push_str(&format!("  call f{}_{function_name}\n", function.as_u32()));
                inst_asm.push_str(&format!("  add ${argument_size}, %rsp\n"));

                inst_asm.push_str("  pop %rsi\n");
                inst_asm.push_str("  pop %rdx\n");
                inst_asm.push_str("  pop %rcx\n");
                inst_asm.push_str("  pop %rbx\n");
                inst_asm.push_str("  pop %rax\n");
                inst_asm.push_str(&format!("  add ${}, %rsp\n", stack_size));

                inst_asm.push_str(
                    &return_allocation.move_to(&self.allocations.instructions[inst].allocation),
                );

                inst_asm.push_str("\n");

                inst_asm
            }
            InstData::Jump { block, argument } => {
                let argument_allocation = &self.allocations.arguments[*block];

                format!(
                    "{}  jmp b{}\n",
                    self.expr_allocation(*argument).move_to(argument_allocation),
                    block.as_u32()
                )
            }
            InstData::JumpCondition {
                condition,
                then,
                else_,
            } => {
                let condition_asm = self.expr_allocation(*condition).asm().unwrap();

                format!(
                    "  cmp $0, {condition_asm}\n  jne b{}\n  jmp b{}",
                    then.as_u32(),
                    else_.as_u32(),
                )
            }
            InstData::Return(expr) => {
                let function = self.block_function(self.inst_block(inst));
                let return_allocation = &self.allocations.returns[function];

                let mut inst_asm = String::new();
                let allocation = self.expr_allocation(*expr);
                inst_asm.push_str(&allocation.move_to(&return_allocation));

                inst_asm.push_str("\n  mov %rbp, %rsp\n  pop %rbp\n  ret\n");

                inst_asm
            }
        }
    }

    fn block_function(&self, block: Block) -> Block {
        if let BlockData::Function { .. } = &self.ssa.blocks[block] {
            return block;
        }

        let mut visited_blocks = HashSet::new();

        let mut blocks = Vec::from([block]);

        while let Some(block) = blocks.pop() {
            visited_blocks.insert(block);

            match self.ssa.blocks[block] {
                BlockData::ExternFunction { .. } => unreachable!(),
                BlockData::Function { .. } => return block,
                BlockData::Block { .. } => {}
            }

            blocks.extend(
                self.block_parents(block)
                    .filter(|block| !visited_blocks.contains(block)),
            );
        }

        panic!()
    }

    fn block_parents(&self, block: Block) -> impl Iterator<Item = Block> {
        self.ssa
            .blocks
            .entries()
            .filter(move |(parent, _)| self.block_children(*parent).any(|child| child == block))
            .map(|(block, _)| block)
    }

    fn other_function_allocations(
        &self,
        argument_type: Type,
        return_type: Type,
        stack_size: u64,
    ) -> (Allocation, Allocation) {
        let argument_size = self.type_size(argument_type);
        let return_size = self.type_size(return_type);

        (
            match argument_size {
                0 => Allocation::Unit,
                4 => Allocation::Esi,
                size => Allocation::Stack {
                    // TEMP: This magic `40` corresponds to the size of the
                    // registers we push on the stack before calling a function.
                    offset: stack_size + size + 40,
                    size,
                },
            },
            match return_size {
                0 => Allocation::Unit,
                4 => Allocation::Edi,
                size => Allocation::Stack {
                    offset: stack_size + argument_size,
                    size,
                },
            },
        )
    }

    #[track_caller]
    fn expr_allocation(&self, expr: Expr) -> Allocation {
        match expr {
            Expr::Inst(inst) => self.allocations.instructions[inst].allocation,
            Expr::BlockArg(block) => self.allocations.arguments[block],
            Expr::Const(const_) => match self.ssa.consts.get(const_) {
                Sentinel(_) => panic!(),
                Item(const_data) => match const_data {
                    ConstData::Uint32(value) => Allocation::Immediate(*value),
                    ConstData::Product(_, _) => panic!(),
                },
            },
        }
    }

    #[track_caller]
    fn expr_type(&self, expr: Expr) -> Type {
        match expr {
            Expr::Const(const_) => match self.ssa.consts.get(const_) {
                Sentinel(sentinel) => match sentinel {
                    ConstSentinel::Unit => TypeSentinel::Unit.to_index(),
                    ConstSentinel::False => TypeSentinel::False.to_index(),
                    ConstSentinel::True => TypeSentinel::True.to_index(),
                },
                Item(const_data) => match const_data {
                    ConstData::Uint32(_) => TypeSentinel::Uint32.to_index(),
                    ConstData::Product(_, _) => todo!(),
                },
            },
            Expr::Inst(inst) => match self.ssa.insts[inst] {
                InstData::Field(_, _) => todo!(),
                InstData::Record(_, ty) => ty,
                InstData::Equal(_, _) => TypeSentinel::Bool.to_index(),
                InstData::Add(_, _)
                | InstData::Sub(_, _)
                | InstData::Mul(_, _)
                | InstData::Div(_, _) => TypeSentinel::Uint32.to_index(),
                InstData::Call { function, .. } => match self.ssa.blocks[function] {
                    BlockData::ExternFunction { ret, .. } | BlockData::Function { ret, .. } => ret,
                    _ => panic!(),
                },
                InstData::Jump { .. } | InstData::JumpCondition { .. } | InstData::Return(_) => {
                    TypeSentinel::Unit.to_index()
                }
            },
            Expr::BlockArg(block) => match self.ssa.blocks[block] {
                BlockData::ExternFunction { arg, .. }
                | BlockData::Function { arg, .. }
                | BlockData::Block { arg, .. } => arg,
            },
        }
    }

    fn generate_block(&mut self, block: Block) {
        let BlockData::Block { insts, .. } = &self.ssa.blocks[block] else {
            panic!()
        };

        let mut asm = String::new();

        asm.push_str(&format!("b{}:\n", block.as_u32()));

        for inst in insts {
            asm.push_str(&self.inst_asm(*inst));
        }

        self.blocks[block] = asm;
    }

    fn type_size(&self, type_: Type) -> u64 {
        match self.types.get(type_) {
            Sentinel(sentinel) => match sentinel {
                TypeSentinel::Unknown => panic!(),
                TypeSentinel::Bool | TypeSentinel::False | TypeSentinel::True => 4,
                TypeSentinel::Unit => 0,
                TypeSentinel::Uint32 => 4,
            },
            Item(type_data) => match type_data {
                TypeData::Function { .. } => panic!(),
                TypeData::Product { fields } => {
                    let size = fields
                        .iter()
                        .fold(0, |acc, (_, field_type)| acc + self.type_size(*field_type));

                    size
                }
            },
        }
    }
}
