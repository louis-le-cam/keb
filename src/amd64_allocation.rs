use std::{borrow::Cow, collections::HashSet};

use crate::{
    key_vec::{KeyVec, Sentinel as _, Val},
    semantic::{Type, TypeData, TypeSentinel, Types},
    ssa::{
        Block, BlockData, BlockSentinel, ConstData, ConstSentinel, Expr, Inst, InstData,
        InstSentinel, Ssa,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Allocation {
    Unit,
    Stack { offset: u64, size: u64 },
    StackArgument { offset: u64, size: u64 },
    Eax,
    Ebx,
    Ecx,
    Edx,
    Esi,
    Edi,
    R8d,
    Immediate(u32),
}

pub struct InstAllocations {
    pub reallocations: Vec<(Allocation, Allocation)>,
    pub allocation: Allocation,
    pub deallocations: Vec<Inst>,
}

struct Allocator<'a> {
    types: &'a Types,
    ssa: &'a Ssa,
    allocations: Allocations,
}

pub struct Allocations {
    /// The allocation of the argument of a function from the function's
    /// perspective (i.e. stack allocation are relative to the callee stack).
    ///
    /// Block arguments are also specified here.
    pub arguments: KeyVec<BlockSentinel, Allocation>,
    /// The allocation of the return value of a function from the function's
    /// perspective (i.e. stack allocation are relative to the callee stack).
    ///
    /// Block don't have any return value so their return allocation should be
    /// [`Allocation::Unit`].
    pub returns: KeyVec<BlockSentinel, Allocation>,
    /// The allocation of the result of an instruction.
    ///
    /// If an instruction doesn't result in any value, the allocation should be
    /// [`Allocation::Unit`].
    pub instructions: KeyVec<InstSentinel, InstAllocations>,
}

pub fn allocate(types: &Types, ssa: &Ssa) -> Allocations {
    let mut allocator = Allocator {
        types,
        ssa,
        allocations: Allocations {
            arguments: KeyVec::from_vec((0..ssa.blocks.len()).map(|_| Allocation::Unit).collect()),
            returns: KeyVec::from_vec((0..ssa.blocks.len()).map(|_| Allocation::Unit).collect()),
            instructions: KeyVec::from_vec(
                (0..ssa.insts.len())
                    .map(|_| InstAllocations {
                        reallocations: Vec::new(),
                        allocation: Allocation::Unit,
                        deallocations: Vec::new(),
                    })
                    .collect(),
            ),
        },
    };

    allocator.allocate_all();

    allocator.allocations
}

impl Allocator<'_> {
    fn allocate_all(&mut self) {
        for (block, block_data) in self.ssa.blocks.entries() {
            self.allocate_function(block, block_data);
        }
    }

    fn allocate_function(&mut self, function: Block, block_data: &BlockData) {
        let BlockData::Function {
            name,
            arg,
            ret,
            insts,
        } = block_data
        else {
            return;
        };

        let mut stack_bottom = 0;
        self.allocations.arguments[function] = match self.type_size(*arg) {
            0 => Allocation::Unit,
            4 => Allocation::Esi,
            size => {
                stack_bottom += size;
                Allocation::StackArgument { offset: 0, size }
            }
        };

        let return_size = self.type_size(*ret);
        self.allocations.returns[function] = if name == "main" {
            assert_eq!(return_size, 4);
            Allocation::Eax
        } else {
            match return_size {
                0 => Allocation::Unit,
                4 => Allocation::Edi,
                size => {
                    let allocation = Allocation::Stack {
                        offset: stack_bottom,
                        size,
                    };
                    stack_bottom += size;
                    allocation
                }
            }
        };

        for block in self.block_descendants(function) {
            self.allocate_block_argument(block, &mut stack_bottom);
        }

        for inst in insts {
            self.allocate_instruction(*inst);
        }

        for block in self.block_descendants(function) {
            let BlockData::Block { arg: _, insts } = &self.ssa.blocks[block] else {
                panic!();
            };

            for inst in insts {
                self.allocate_instruction(*inst);
            }
        }
    }

    fn allocate_block_argument(&mut self, block: Block, stack_bottom: &mut u64) {
        let BlockData::Block { arg, .. } = &self.ssa.blocks[block] else {
            unreachable!();
        };

        let argument_size = self.type_size(*arg);
        let allocation = if argument_size == 0 {
            Allocation::Unit
        } else {
            *stack_bottom += argument_size;
            Allocation::Stack {
                offset: *stack_bottom + argument_size,
                size: argument_size,
            }
        };

        self.allocations.arguments[block] = allocation;
    }

    fn allocate_instruction(&mut self, inst: Inst) {
        self.allocations.instructions[inst].allocation = match &self.ssa.insts[inst] {
            InstData::Field(expr, field) => {
                self.dealloc_if_unused(inst, *expr);

                let expr_type = self.expr_type(*expr);

                let field_type = match self.types.get(expr_type) {
                    Val::None => panic!(),
                    Val::Sentinel(_) => panic!(),
                    Val::Value(type_data) => match type_data {
                        TypeData::Function { .. } => panic!(),
                        TypeData::Product { fields } => fields[*field as usize].1,
                    },
                };
                self.allocate(inst, self.type_size(field_type))
            }
            InstData::Record(fields, type_) => {
                for field in fields {
                    self.dealloc_if_unused(inst, *field);
                }

                self.allocate(inst, self.type_size(*type_))
            }
            InstData::Equal(lhs, rhs) | InstData::Add(lhs, rhs) | InstData::Sub(lhs, rhs) => {
                self.dealloc_if_unused(inst, *lhs);
                self.dealloc_if_unused(inst, *rhs);
                self.allocate(inst, 4)
            }
            InstData::Mul(lhs, rhs) | InstData::Div(lhs, rhs) => {
                self.dealloc_if_unused(inst, *lhs);
                self.dealloc_if_unused(inst, *rhs);
                // TODO: Do proper allocation for multiplication and division,
                // they only work on some registers.
                Allocation::Eax
            }
            InstData::Call { function, argument } => {
                self.dealloc_if_unused(inst, *argument);
                let return_type = match self.ssa.blocks[*function] {
                    BlockData::ExternFunction { ret, .. } | BlockData::Function { ret, .. } => ret,
                    BlockData::Block { .. } => panic!(),
                };

                self.allocate(inst, self.type_size(return_type))
            }
            InstData::Jump { block: _, argument } => {
                self.dealloc_if_unused(inst, *argument);
                Allocation::Unit
            }
            InstData::JumpCondition {
                condition,
                then: _,
                else_: _,
            } => {
                self.dealloc_if_unused(inst, *condition);
                Allocation::Unit
            }
            InstData::Return(expr) => {
                self.dealloc_if_unused(inst, *expr);
                Allocation::Unit
            }
        };
    }

    fn allocate(&self, inst: Inst, size: u64) -> Allocation {
        let used_allocations = self.used_allocations(inst);

        if size == 4 {
            let available_registers = [
                Allocation::Eax,
                Allocation::Ebx,
                Allocation::Ecx,
                Allocation::Edx,
            ];

            if let Some(allocation) = available_registers.into_iter().find(|allocation| {
                !used_allocations
                    .iter()
                    .any(|used_alloc| used_alloc == allocation)
            }) {
                return allocation;
            }
        }

        let stack_bottom = self.stack_bottom(&used_allocations);

        Allocation::Stack {
            offset: stack_bottom + size,
            size,
        }
    }

    fn dealloc_if_unused(&mut self, at: Inst, expr: Expr) {
        if let Expr::Inst(inst) = expr
            && self.inst_used_after(inst, at)
        {
            self.allocations.instructions[at].deallocations.push(inst);
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

        for inst in insts {
            let inst_allocations = &self.allocations.instructions[*inst];

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

    fn inst_used_after(&self, inst: Inst, after: Inst) -> bool {
        let block = self.inst_block(inst);

        let insts = match &self.ssa.blocks[block] {
            BlockData::ExternFunction { .. } => panic!(),
            BlockData::Function { insts, .. } | BlockData::Block { insts, .. } => insts,
        };

        !insts
            .iter()
            .skip_while(|fn_inst| **fn_inst != after)
            .skip(1)
            .chain(
                self.block_descendants(block)
                    .flat_map(|block| match &self.ssa.blocks[block] {
                        BlockData::ExternFunction { .. } => panic!(),
                        BlockData::Function { insts, .. } | BlockData::Block { insts, .. } => insts,
                    }),
            )
            .any(|other_inst| {
                self.instruction_usages(*other_inst)
                    .any(|usage| usage == inst)
            })
    }

    fn block_descendants<'a, 'b>(&'a self, block: Block) -> impl Iterator<Item = Block> + 'b {
        let mut blocks = Vec::from([block]);
        let mut visited = HashSet::new();

        while let Some(child) = blocks.pop() {
            visited.insert(child);

            blocks.extend(
                self.block_children(child)
                    .filter(|descendant| !visited.contains(descendant)),
            );
        }

        visited.remove(&block);
        visited.into_iter()
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

    fn instruction_usages(&self, inst: Inst) -> impl Iterator<Item = Inst> {
        let expr_as_inst = |expr| match expr {
            Expr::Inst(inst) => Some(inst),
            Expr::Const(_) | Expr::BlockArg(_) => None,
        };

        match &self.ssa.insts[inst] {
            InstData::Field(Expr::Inst(record), _) => Vec::from([*record]),
            InstData::Field(_, _) => Vec::new(),
            InstData::Record(exprs, _) => exprs
                .iter()
                .filter_map(|expr| expr_as_inst(*expr))
                .collect(),
            InstData::Equal(lhs, rhs) => [lhs, rhs]
                .into_iter()
                .filter_map(|expr| expr_as_inst(*expr))
                .collect(),
            InstData::Add(lhs, rhs) => [lhs, rhs]
                .into_iter()
                .filter_map(|expr| expr_as_inst(*expr))
                .collect(),
            InstData::Sub(lhs, rhs) => [lhs, rhs]
                .into_iter()
                .filter_map(|expr| expr_as_inst(*expr))
                .collect(),
            InstData::Mul(lhs, rhs) => [lhs, rhs]
                .into_iter()
                .filter_map(|expr| expr_as_inst(*expr))
                .collect(),
            InstData::Div(lhs, rhs) => [lhs, rhs]
                .into_iter()
                .filter_map(|expr| expr_as_inst(*expr))
                .collect(),
            InstData::Call {
                function: _,
                argument: Expr::Inst(argument),
            } => Vec::from([*argument]),
            InstData::Call {
                function: _,
                argument: _,
            } => Vec::new(),
            InstData::Jump {
                block: _,
                argument: Expr::Inst(argument),
            } => Vec::from([*argument]),
            InstData::Jump {
                block: _,
                argument: _,
            } => Vec::new(),
            InstData::JumpCondition {
                condition: Expr::Inst(condition),
                then: _,
                else_: _,
            } => Vec::from([*condition]),
            InstData::JumpCondition {
                condition: _,
                then: _,
                else_: _,
            } => Vec::new(),
            InstData::Return(Expr::Inst(inst)) => Vec::from([*inst]),
            InstData::Return(_) => Vec::new(),
        }
        .into_iter()
    }

    fn type_size(&self, type_: Type) -> u64 {
        match self.types.get(type_) {
            Val::None => panic!(),
            Val::Sentinel(sentinel) => match sentinel {
                TypeSentinel::Unknown => panic!(),
                TypeSentinel::Bool | TypeSentinel::False | TypeSentinel::True => 4,
                TypeSentinel::Unit => 0,
                TypeSentinel::Uint32 => 4,
            },
            Val::Value(type_data) => match type_data {
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

    #[track_caller]
    fn expr_type(&self, expr: Expr) -> Type {
        match expr {
            Expr::Const(const_) => match self.ssa.consts.get(const_) {
                Val::None => panic!(),
                Val::Sentinel(sentinel) => match sentinel {
                    ConstSentinel::Unit => TypeSentinel::Unit.to_index(),
                    ConstSentinel::False => TypeSentinel::False.to_index(),
                    ConstSentinel::True => TypeSentinel::True.to_index(),
                },
                Val::Value(const_data) => match const_data {
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
}

impl Allocation {
    pub fn asm(&self) -> Option<Cow<'static, str>> {
        match self {
            Allocation::Unit => None,
            Allocation::Stack { offset, .. } => Some(Cow::Owned(format!("-{}(%rbp)", offset))),
            Allocation::StackArgument { offset, size: _ } => {
                Some(Cow::Owned(format!("{}(%rbp)", offset + 16)))
            }
            Allocation::Eax => Some(Cow::Borrowed("%eax")),
            Allocation::Ebx => Some(Cow::Borrowed("%ebx")),
            Allocation::Ecx => Some(Cow::Borrowed("%ecx")),
            Allocation::Edx => Some(Cow::Borrowed("%edx")),
            Allocation::Esi => Some(Cow::Borrowed("%esi")),
            Allocation::Edi => Some(Cow::Borrowed("%edi")),
            Allocation::R8d => Some(Cow::Borrowed("%r8d")),
            Allocation::Immediate(value) => Some(Cow::Owned(format!("${value}"))),
        }
    }

    pub fn move_to(&self, destination: &Allocation) -> String {
        if self == destination {
            return String::new();
        }

        match destination.size() {
            0 => String::new(),
            4 => format!(
                "  movl {}, {}\n",
                self.asm().unwrap(),
                destination.asm().unwrap()
            ),
            8 => format!(
                "  movl {}, {}\n  movl {}, {}\n  movl {}, {}\n  movl {}, {}\n",
                self.offset(0, 4).asm().unwrap(),
                // FIXME: Took a random register to move memory to memory, not
                // the greatest idea.
                Allocation::R8d.asm().unwrap(),
                Allocation::R8d.asm().unwrap(),
                destination.offset(0, 4).asm().unwrap(),
                self.offset(4, 4).asm().unwrap(),
                Allocation::R8d.asm().unwrap(),
                Allocation::R8d.asm().unwrap(),
                destination.offset(4, 4).asm().unwrap(),
            ),
            _ => todo!(),
        }
    }

    pub fn size(&self) -> u64 {
        match self {
            Allocation::Unit => 0,
            Allocation::Stack { size, .. } | Allocation::StackArgument { size, .. } => *size,
            Allocation::Eax
            | Allocation::Ebx
            | Allocation::Ecx
            | Allocation::Edx
            | Allocation::Esi
            | Allocation::Edi
            | Allocation::R8d
            | Allocation::Immediate(_) => 4,
        }
    }

    pub fn offset(&self, offset: u64, size: u64) -> Allocation {
        match self {
            Allocation::Stack {
                offset: base_offset,
                ..
            } => Allocation::Stack {
                offset: base_offset - offset,
                size,
            },
            Allocation::StackArgument {
                offset: base_offset,
                ..
            } => Allocation::StackArgument {
                offset: base_offset + offset,
                size,
            },
            _ => panic!(),
        }
    }
}
