use crate::{
    key_vec::{Index, KeyVec, Sentinel},
    token::Token,
};

#[derive(Debug)]
pub enum SynData {
    Root(Vec<Syn>),
    Ident(Token),
    False(Token),
    True(Token),
    Number(Token),
    Add(Syn, Syn),
    Subtract(Syn, Syn),
    Binding { pattern: Syn, value: Syn },
    Function { pattern: Syn, body: Syn },
    ReturnAscription { syn: Syn, type_: Syn },
    Ascription { syn: Syn, type_: Syn },
    Access { syn: Syn, key: Syn },
    EmptyParen(Token),
    Paren(Syn),
    EmptyCurly(Token),
    Curly(Syn),
    Tuple(Vec<Syn>),
    Application { function: Syn, argument: Syn },
    Loop(Syn),
    ChainOpen(Vec<Syn>),
    ChainClosed(Vec<Syn>),
}

#[derive(Sentinel, Clone, Copy, Debug)]
pub enum SynSentinel {}

pub type Syn = Index<SynSentinel>;
pub type Syns = KeyVec<SynSentinel, SynData>;

pub const ROOT_SYN: Syn = Syn::from_u32_index(0);
