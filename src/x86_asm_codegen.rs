use std::borrow::Cow;

use crate::{
    key_vec::{KeyVec, Sentinel, Val},
    semantic::{Type, TypeData, TypeSentinel, Types},
    ssa::{
        Block, BlockData, BlockSentinel, ConstData, ConstSentinel, Expr, InstData, InstSentinel,
        Ssa,
    },
};

pub fn generate(types: &Types, ssa: &Ssa) -> String {
    let mut generator = Generator {
        types,
        ssa,
        blocks: KeyVec::from_vec((0..ssa.blocks.len()).map(|_| String::new()).collect()),
        args_allocations: KeyVec::from_vec((0..ssa.blocks.len()).map(|_| None).collect()),
        insts_allocations: KeyVec::from_vec((0..ssa.insts.len()).map(|_| None).collect()),
        next_allocations: [
            Allocation::Eax,
            Allocation::Ebx,
            Allocation::Ecx,
            Allocation::Edx,
        ],
    };

    generator.generate();

    generator.result()
}

#[derive(Clone, Copy, Debug)]
enum Allocation {
    Stack { offset: u64, size: u64 },
    StackArgument { offset: u64, size: u64 },
    Eax,
    Ebx,
    Ecx,
    Edx,
    Esi,
    Edi,
    Immediate(u32),
}

struct Generator<'a> {
    types: &'a Types,
    ssa: &'a Ssa,
    blocks: KeyVec<BlockSentinel, String>,
    args_allocations: KeyVec<BlockSentinel, Option<Allocation>>,
    insts_allocations: KeyVec<InstSentinel, Option<Allocation>>,
    next_allocations: [Allocation; 4],
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

    fn generate_function(&mut self, function: Block) {
        let mut stack_size = 0;

        let BlockData::Function {
            name,
            arg,
            ret,
            insts,
        } = &self.ssa.blocks[function]
        else {
            panic!()
        };

        let mut asm = String::new();

        if name == "main" {
            asm.push_str(&format!(".set main, f{}_{name}\n", function.as_u32()));
        }

        asm.push_str(&format!("f{}_{name}:\n", function.as_u32()));

        asm.push_str("  push %rbp\n");
        asm.push_str("  mov %rsp, %rbp\n\n");

        let argument_size = self.type_size(*arg);
        self.args_allocations[function] = match argument_size {
            0 => None,
            4 => Some(Allocation::Esi),
            size => {
                let allocation = Allocation::StackArgument {
                    offset: 0,
                    size: size,
                };
                // stack_size += size;
                Some(allocation)
            }
        };

        let return_size = self.type_size(*ret);
        let return_allocation = match return_size {
            0 => None,
            4 => Some(Allocation::Edi),
            size => {
                let allocation = Allocation::Stack {
                    offset: stack_size,
                    size,
                };
                stack_size += size;
                Some(allocation)
            }
        };

        for inst in insts {
            let inst_asm = match &self.ssa.insts[*inst] {
                InstData::Field(expr, field) => {
                    let allocation = self.expr_allocation(*expr);

                    let expr_type = self.expr_type(*expr);

                    let field_offset = match self.types.get(expr_type) {
                        Val::None => panic!(),
                        Val::Sentinel(_) => panic!(),
                        Val::Value(type_data) => match type_data {
                            TypeData::Function { .. } => panic!(),
                            TypeData::Product { fields } => fields[0..*field as usize]
                                .iter()
                                .fold(0, |acc, (_, field_type)| acc + self.type_size(*field_type)),
                        },
                    };

                    let field_type = match self.types.get(expr_type) {
                        Val::None => panic!(),
                        Val::Sentinel(_) => panic!(),
                        Val::Value(type_data) => match type_data {
                            TypeData::Function { .. } => panic!(),
                            TypeData::Product { fields } => fields[*field as usize].1,
                        },
                    };

                    let field_size = self.type_size(field_type);
                    let field_allocation = self.reserve_allocation(field_size, &mut stack_size);

                    let source_allocation =
                        self.offset_allocation(allocation, field_offset, field_size);

                    self.insts_allocations[*inst] = Some(field_allocation);

                    self.move_(&source_allocation, &field_allocation)
                }
                InstData::Record(fields, type_) => {
                    let record_size = self.type_size(*type_);

                    let allocation = self.reserve_allocation(record_size, &mut stack_size);
                    self.insts_allocations[*inst] = Some(allocation);

                    let mut inst_asm = String::new();

                    let mut offset = 0;
                    for field in fields {
                        let field_allocation = self.expr_allocation(*field);
                        let field_size = self.type_size(self.expr_type(*field));
                        inst_asm.push_str(&self.move_(
                            &field_allocation,
                            &self.offset_allocation(allocation, offset, field_size),
                        ));

                        offset += field_size;
                    }

                    inst_asm
                }
                InstData::Equal(_lhs, _rhs) => todo!(),
                InstData::Add(lhs, rhs) => {
                    let lhs_allocation = self.expr_allocation(*lhs);
                    let rhs_allocation = self.expr_allocation(*rhs);

                    self.insts_allocations[*inst] = Some(lhs_allocation);

                    format!(
                        "  add {}, {}\n",
                        allocation_asm(&rhs_allocation),
                        allocation_asm(&lhs_allocation),
                    )
                }
                InstData::Sub(_lhs, _rhs) => todo!(),
                InstData::Mul(_lhs, _rhs) => todo!(),
                InstData::Div(_lhs, _rhs) => todo!(),
                InstData::Call { function, argument } => {
                    let mut inst_asm = "\n".to_string();
                    let (argument_type, return_type) = match self.ssa.blocks[*function] {
                        BlockData::ExternFunction { arg, ret, .. }
                        | BlockData::Function { arg, ret, .. } => (arg, ret),
                        BlockData::Block { .. } => panic!(),
                    };

                    let (argument_allocation, return_allocation) =
                        self.other_function_allocations(argument_type, return_type, stack_size);

                    if let Some(argument_allocation) = argument_allocation {
                        let allocation = self.expr_allocation(*argument);
                        inst_asm.push_str(&self.move_(&allocation, &argument_allocation));
                    };

                    let argument_size = argument_allocation
                        .map(|allocation| allocation_size(&allocation))
                        .unwrap_or(0);

                    let function_name = match &self.ssa.blocks[*function] {
                        BlockData::ExternFunction { name, .. }
                        | BlockData::Function { name, .. } => name,
                        BlockData::Block { .. } => panic!(),
                    };

                    inst_asm.push_str(&format!("  sub ${}, %rsp\n", stack_size + argument_size));
                    inst_asm.push_str(&format!("  call f{}_{function_name}\n", function.as_u32()));
                    inst_asm.push_str(&format!("  add ${}, %rsp\n", stack_size + argument_size));

                    if let Some(return_allocation) = return_allocation {
                        let allocation =
                            self.reserve_allocation(self.type_size(return_type), &mut stack_size);

                        self.insts_allocations[*inst] = Some(allocation);

                        inst_asm.push_str(&self.move_(&return_allocation, &allocation));
                    }

                    inst_asm.push_str("\n");

                    inst_asm
                }
                InstData::Jump { .. } => todo!(),
                InstData::JumpCondition { .. } => todo!(),
                InstData::Return(expr) => {
                    let mut inst_asm = String::new();
                    if let Some(return_allocation) = return_allocation {
                        let allocation = self.expr_allocation(*expr);
                        inst_asm.push_str(&self.move_(&allocation, &return_allocation));
                    }

                    if name == "main" {
                        inst_asm.push_str("\n  mov %edi, %eax");
                    }

                    inst_asm.push_str("\n  mov %rbp, %rsp\n  pop %rbp\n  ret\n");

                    inst_asm
                }
            };

            asm.push_str(&inst_asm);
        }

        self.blocks[function] = asm;
    }

    fn move_(&self, source: &Allocation, destination: &Allocation) -> String {
        match allocation_size(destination) {
            4 => format!(
                "  movl {}, {}\n",
                allocation_asm(source),
                allocation_asm(destination)
            ),
            8 => format!(
                "  movl {}, {}\n  movl {}, {}\n  movl {}, {}\n  movl {}, {}\n",
                allocation_asm(&self.offset_allocation(*source, 0, 4)),
                // FIXME: Took a random register to move memory to memory, not
                // the greatest idea
                allocation_asm(&Allocation::Esi),
                allocation_asm(&Allocation::Esi),
                allocation_asm(&self.offset_allocation(*destination, 0, 4)),
                allocation_asm(&self.offset_allocation(*source, 4, 4)),
                allocation_asm(&Allocation::Esi),
                allocation_asm(&Allocation::Esi),
                allocation_asm(&self.offset_allocation(*destination, 4, 4)),
            ),
            _ => todo!(),
        }
    }

    fn other_function_allocations(
        &self,
        argument_type: Type,
        return_type: Type,
        stack_size: u64,
    ) -> (Option<Allocation>, Option<Allocation>) {
        let argument_size = self.type_size(argument_type);
        let return_size = self.type_size(return_type);

        (
            match argument_size {
                0 => None,
                4 => Some(Allocation::Esi),
                size => {
                    let allocation = Allocation::Stack {
                        offset: stack_size + size,
                        size,
                    };
                    Some(allocation)
                }
            },
            match return_size {
                0 => None,
                4 => Some(Allocation::Edi),
                size => {
                    let allocation = Allocation::Stack {
                        offset: stack_size + argument_size,
                        size,
                    };
                    Some(allocation)
                }
            },
        )
    }

    #[track_caller]
    fn expr_allocation(&self, expr: Expr) -> Allocation {
        match expr {
            Expr::Inst(inst) => self.insts_allocations[inst].unwrap(),
            Expr::BlockArg(block) => self.args_allocations[block].unwrap(),
            Expr::Const(const_) => match self.ssa.consts.get(const_) {
                Val::None => panic!(),
                Val::Sentinel(_) => panic!(),
                Val::Value(const_data) => match const_data {
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

    fn generate_block(&mut self, block: Block) {
        let BlockData::Block { arg, insts } = &self.ssa.blocks[block] else {
            panic!()
        };

        let Some(TypeSentinel::Unit) = arg.sentinel() else {
            todo!("implemented block arg");
        };

        let mut asm = String::new();

        asm.push_str(&format!("b{}:\n", block.as_u32()));

        self.blocks[block] = asm;
    }

    fn reserve_allocation(&mut self, size: u64, stack_size: &mut u64) -> Allocation {
        match size {
            // TODO: Use an unused allocation
            4 => {
                let allocation = self.next_allocations[0];
                self.next_allocations.rotate_left(1);
                allocation
            }
            _ => {
                let allocation = Allocation::Stack {
                    offset: size + *stack_size,
                    size,
                };
                *stack_size += size;
                allocation
            }
        }
    }

    fn offset_allocation(&self, allocation: Allocation, offset: u64, size: u64) -> Allocation {
        match allocation {
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

    fn type_size(&self, type_: Type) -> u64 {
        match self.types.get(type_) {
            Val::None => panic!(),
            Val::Sentinel(sentinel) => match sentinel {
                TypeSentinel::Unknown => panic!(),
                TypeSentinel::Bool | TypeSentinel::False | TypeSentinel::True => 1,
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
}

fn allocation_asm(allocation: &Allocation) -> Cow<'static, str> {
    match allocation {
        Allocation::Stack { offset, .. } => Cow::Owned(format!("-{}(%rbp)", offset)),
        Allocation::StackArgument { offset, size: _ } => {
            Cow::Owned(format!("{}(%rbp)", offset + 16))
        }
        Allocation::Eax => Cow::Borrowed("%eax"),
        Allocation::Ebx => Cow::Borrowed("%ebx"),
        Allocation::Ecx => Cow::Borrowed("%ecx"),
        Allocation::Edx => Cow::Borrowed("%edx"),
        Allocation::Esi => Cow::Borrowed("%esi"),
        Allocation::Edi => Cow::Borrowed("%edi"),
        Allocation::Immediate(value) => Cow::Owned(format!("${value}")),
    }
}

fn allocation_size(allocation: &Allocation) -> u64 {
    match allocation {
        Allocation::Stack { size, .. } | Allocation::StackArgument { size, .. } => *size,
        Allocation::Eax
        | Allocation::Ebx
        | Allocation::Ecx
        | Allocation::Edx
        | Allocation::Esi
        | Allocation::Edi
        | Allocation::Immediate(_) => 4,
    }
}
