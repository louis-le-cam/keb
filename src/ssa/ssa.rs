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
            Expr::BlockArg(block) => match &self.blocks[block] {
                BlockData::ExternFunction { arg, .. }
                | BlockData::Function { arg, .. }
                | BlockData::Block { arg, .. } => *arg,
            },
        }
    }

    pub fn instruction_type(&self, types: &Types, inst: Inst) -> Type {
        match &self.insts[inst] {
            InstData::Field(expr, field) => {
                let field = *field;

                let record_type = self.expression_type(types, *expr);

                match types.get(record_type) {
                    Val::Value(TypeData::Product { fields }) => fields[field as usize].1,
                    Val::None | Val::Sentinel(_) | Val::Value(_) => panic!(),
                }
            }
            InstData::Record(_, ty) => *ty,
            InstData::Equal(lhs, _)
            | InstData::Add(lhs, _)
            | InstData::Sub(lhs, _)
            | InstData::Mul(lhs, _)
            | InstData::Div(lhs, _) => self.expression_type(types, *lhs),
            InstData::Call {
                function: block, ..
            } => match &self.blocks[*block] {
                BlockData::ExternFunction { ret, .. } | BlockData::Function { ret, .. } => *ret,
                BlockData::Block { .. } => TypeSentinel::Unit.to_index(),
            },
            InstData::Jump { .. } | InstData::JumpCondition { .. } | InstData::Return(_) => {
                TypeSentinel::Unit.to_index()
            }
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

    pub fn inst(&mut self, block: Block, inst_data: InstData) -> Inst {
        let inst = self.insts.push(inst_data);

        match &mut self.blocks[block] {
            BlockData::ExternFunction { .. } => {
                panic!("Cannot add instruction to extern function")
            }
            BlockData::Function { insts, .. } | BlockData::Block { insts, .. } => insts.push(inst),
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

    pub fn inst_call(&mut self, block: Block, target_function: Block, argument: Expr) -> Inst {
        self.inst(
            block,
            InstData::Call {
                function: target_function,
                argument,
            },
        )
    }

    pub fn inst_jump(&mut self, block: Block, target_block: Block, argument: Expr) -> Inst {
        self.inst(
            block,
            InstData::Jump {
                block: target_block,
                argument,
            },
        )
    }

    pub fn inst_jump_condition(
        &mut self,
        block: Block,
        condition: Expr,
        then: Block,
        else_: Block,
    ) -> Inst {
        self.inst(
            block,
            InstData::JumpCondition {
                condition,
                then,
                else_,
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
#[derive(Sentinel, Clone, Copy, Debug, PartialEq, Eq)]
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
    Equal(Expr, Expr),
    Add(Expr, Expr),
    Sub(Expr, Expr),
    Mul(Expr, Expr),
    Div(Expr, Expr),
    Call {
        function: Block,
        argument: Expr,
    },
    Jump {
        block: Block,
        argument: Expr,
    },
    JumpCondition {
        condition: Expr,
        then: Block,
        else_: Block,
        // TODO: Block arguments
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
