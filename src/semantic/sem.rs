use crate::{
    key_vec::{Index, KeyVec, Sentinel},
    semantic::Type,
    token::Token,
};

#[derive(Clone, Debug)]
pub struct SemData {
    pub kind: SemKind,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub enum SemKind {
    False(Token),
    True(Token),
    Number(Token),
    Module {
        bindings: Vec<(String, Sem)>,
    },
    Function {
        argument: String,
        body: Sem,
    },
    Binding {
        name: String,
        value: Sem,
        body: Sem,
    },
    Reference {
        name: String,
    },
    Access {
        field: String,
        expr: Sem,
    },
    Application {
        function: Sem,
        argument: Sem,
    },
    Loop(Sem),
    If {
        condition: Sem,
        then: Sem,
    },
    IfElse {
        condition: Sem,
        then: Sem,
        else_: Sem,
    },
    BuildStruct {
        fields: Vec<(String, Sem)>,
    },
    ChainOpen {
        statements: Vec<Sem>,
        expression: Sem,
    },
    ChainClosed {
        statements: Vec<Sem>,
    },
}

#[derive(Sentinel, Clone, Copy, Debug)]
pub enum SemSentinel {}

pub type Sem = Index<SemSentinel>;
pub type Semantic = KeyVec<SemSentinel, SemData>;

pub const ROOT_SEM: Sem = Sem::from_u32_index(0);
