use crate::{
    key_vec::{Index, KeyVec, Sentinel, Val},
    semantic::{Type, TypeData, TypeSentinel, Types},
};

#[derive(Default, Debug)]
pub struct Ssa {
    pub blocks: Blocks,
    pub insts: Insts,
    pub consts: Consts,
}

impl Ssa {
    pub fn expression_type(&self, types: &Types, expr: Expr) -> Type {
        match expr {
            Expr::Const(const_) => self.const_type(const_),
            Expr::Inst(inst) => self.instruction_type(types, inst),
            Expr::BlockArg(block) => match self.blocks.get(block) {
                Val::None => panic!(),
                Val::Value(
                    BlockData::ExternFunction { arg, .. }
                    | BlockData::Function { arg, .. }
                    | BlockData::Block { arg, .. },
                ) => *arg,
            },
        }
    }

    pub fn instruction_type(&self, types: &Types, inst: Inst) -> Type {
        match self.insts.get(inst) {
            Val::None => panic!(),
            Val::Value(inst_data) => match inst_data {
                InstData::Field(expr, field) => {
                    let field = *field;

                    let record_type = self.expression_type(types, *expr);

                    match types.get(record_type) {
                        Val::Value(TypeData::Product { fields }) => fields[field as usize].1,
                        Val::None | Val::Sentinel(_) | Val::Value(_) => panic!(),
                    }
                }
                InstData::Record(_, ty) => *ty,
                InstData::Add(lhs, _) => self.expression_type(types, *lhs),
                InstData::Call {
                    function: block, ..
                } => match self.blocks.get(*block) {
                    Val::None => panic!(),
                    Val::Value(
                        BlockData::ExternFunction { ret, .. } | BlockData::Function { ret, .. },
                    ) => *ret,
                    Val::Value(BlockData::Block { .. }) => TypeSentinel::Unit.to_index(),
                },
                InstData::Jump { .. } | InstData::Return(_) => TypeSentinel::Unit.to_index(),
            },
        }
    }

    pub fn const_type(&self, const_: Const) -> Type {
        match self.consts.get(const_) {
            Val::None => panic!(),
            Val::Sentinel(sentinel) => match sentinel {
                ConstSentinel::Unit => TypeSentinel::Unit.to_index(),
                ConstSentinel::False => TypeSentinel::False.to_index(),
                ConstSentinel::True => TypeSentinel::True.to_index(),
            },
            Val::Value(value) => match value {
                ConstData::Uint32(_) => TypeSentinel::Uint32.to_index(),
                ConstData::Product(_, ty) => *ty,
            },
        }
    }

    fn block(&mut self, block_data: BlockData) -> Block {
        self.blocks.push(block_data)
    }

    pub fn extern_function(&mut self, name: String, arg: Type, ret: Type) -> Block {
        self.block(BlockData::ExternFunction {
            name: name,
            arg,
            ret,
        })
    }

    pub fn function(&mut self, name: String, arg: Type, ret: Type) -> Block {
        self.block(BlockData::Function {
            name,
            arg,
            ret,
            insts: vec![],
        })
    }

    pub fn basic_block(&mut self, arg: Type) -> Block {
        self.block(BlockData::Block { arg, insts: vec![] })
    }

    fn const_(&mut self, const_data: ConstData) -> Const {
        self.consts.push(const_data)
    }

    pub fn const_product(&mut self, types: &mut Types, fields: Vec<Const>) -> Const {
        if fields.len() == 0 {
            return ConstSentinel::Unit.to_index();
        }

        let type_fields = fields
            .iter()
            .map(|const_| (String::new(), self.const_type(*const_)))
            .collect();

        let type_ = types.push(TypeData::Product {
            fields: type_fields,
        });

        self.const_(ConstData::Product(fields, type_))
    }

    pub fn const_u32(&mut self, value: u32) -> Const {
        self.const_(ConstData::Uint32(value))
    }

    fn inst(&mut self, block: Block, inst_data: InstData) -> Inst {
        let inst = self.insts.push(inst_data);

        match self.blocks.get_mut(block) {
            Val::None => panic!(),
            Val::Value(BlockData::ExternFunction { .. }) => {
                panic!("Cannot add instruction to extern function")
            }
            Val::Value(BlockData::Function { insts, .. } | BlockData::Block { insts, .. }) => {
                insts.push(inst)
            }
        }

        inst
    }

    pub fn inst_field(&mut self, block: Block, expr: Expr, field: u32) -> Inst {
        self.inst(block, InstData::Field(expr, field))
    }

    pub fn inst_product(&mut self, types: &mut Types, block: Block, fields: Vec<Expr>) -> Inst {
        let type_ = if fields.len() == 0 {
            TypeSentinel::Unit.to_index()
        } else {
            let type_fields = fields
                .iter()
                .map(|expr| (String::new(), self.expression_type(types, *expr)))
                .collect();

            types.push(TypeData::Product {
                fields: type_fields,
            })
        };

        self.inst(block, InstData::Record(fields, type_))
    }

    pub fn inst_add(&mut self, block: Block, lhs: Expr, rhs: Expr) -> Inst {
        self.inst(block, InstData::Add(lhs, rhs))
    }

    pub fn inst_call(&mut self, block: Block, target_function: Block, argument: Expr) -> Inst {
        self.inst(
            block,
            InstData::Call {
                function: target_function,
                argument,
            },
        )
    }

    pub fn inst_jump(
        &mut self,
        block: Block,
        target_block: Block,
        condition: Expr,
        argument: Expr,
    ) -> Inst {
        self.inst(
            block,
            InstData::Jump {
                block: target_block,
                condition,
                argument,
            },
        )
    }

    pub fn inst_return(&mut self, block: Block, expr: Expr) -> Inst {
        self.inst(block, InstData::Return(expr))
    }
}

#[derive(Sentinel, Clone, Copy, Debug)]
pub enum BlockSentinel {}

#[derive(Sentinel, Clone, Copy, Debug)]
pub enum InstSentinel {}

#[repr(u32)]
#[derive(Sentinel, Clone, Copy, Debug)]
pub enum ConstSentinel {
    Unit = u32::MAX - 2,
    False,
    True,
}

pub type Block = Index<BlockSentinel>;
pub type Inst = Index<InstSentinel>;
pub type Const = Index<ConstSentinel>;

pub type Blocks = KeyVec<BlockSentinel, BlockData>;
pub type Insts = KeyVec<InstSentinel, InstData>;
pub type Consts = KeyVec<ConstSentinel, ConstData>;

#[derive(Debug)]
pub enum BlockData {
    ExternFunction {
        name: String,
        arg: Type,
        ret: Type,
    },
    Function {
        name: String,
        arg: Type,
        ret: Type,
        insts: Vec<Inst>,
    },
    Block {
        arg: Type,
        insts: Vec<Inst>,
    },
}

#[derive(Debug)]
pub enum InstData {
    Field(Expr, u32),
    Record(Vec<Expr>, Type),
    Add(Expr, Expr),
    Call {
        function: Block,
        argument: Expr,
    },
    Jump {
        block: Block,
        // Can be `Expr::Const(Const::TRUE)` for unconditional jumps
        condition: Expr,
        argument: Expr,
    },
    Return(Expr),
}

#[derive(Debug, Clone, Copy)]
pub enum Expr {
    Const(Const),
    Inst(Inst),
    BlockArg(Block),
}

#[derive(Debug)]
pub enum ConstData {
    Uint32(u32),
    Product(Vec<Const>, Type),
}
