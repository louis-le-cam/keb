use crate::{
    key_vec::{Index, KeyVec, Sentinel},
    token::Token,
};

#[derive(Debug)]
pub enum SynData {
    // TODO: Find a solution to avoid nested allocation from the [`Vec<Syn>`],
    // maybe reference a range of syns (with a start and a length) in the main
    // vector.
    Root(Vec<Syn>),
    Ident(Token),
    False(Token),
    True(Token),
    Number(Token),
    Add(Syn, Syn),
    Subtract(Syn, Syn),
    Multiply(Syn, Syn),
    Divide(Syn, Syn),
    // TODO: Maybe binding should only hold one syn that can be either a
    // pattern or an assignement?
    Binding {
        pattern: Syn,
        value: Syn,
    },
    Mut {
        pattern: Syn,
    },
    Assignment {
        pattern: Syn,
        value: Syn,
    },
    Function {
        pattern: Syn,
        body: Syn,
    },
    ReturnAscription {
        syn: Syn,
        type_: Syn,
    },
    Ascription {
        syn: Syn,
        type_: Syn,
    },
    Access {
        syn: Syn,
        key: Syn,
    },
    EmptyParen(Token),
    Paren(Syn),
    EmptyCurly(Token),
    Curly(Syn),
    // TODO: Find a solution to avoid nested allocation from the [`Vec<Syn>`],
    // maybe reference a range of syns (with a start and a length) in the main
    // vector.
    Tuple(Vec<Syn>),
    Application {
        function: Syn,
        argument: Syn,
    },
    Loop(Syn),
    Match(Syn),
    If {
        condition: Syn,
        then: Syn,
    },
    IfElse {
        condition: Syn,
        then: Syn,
        else_: Syn,
    },
    // TODO: Find a solution to avoid nested allocation from the [`Vec<Syn>`],
    // maybe reference a range of syns (with a start and a length) in the main
    // vector.
    ChainOpen(Vec<Syn>),
    // TODO: Find a solution to avoid nested allocation from the [`Vec<Syn>`],
    // maybe reference a range of syns (with a start and a length) in the main
    // vector.
    ChainClosed(Vec<Syn>),
    // TODO: Find a solution to avoid nested allocation.
    String(Vec<StringSegment>),
}

#[derive(Debug)]
pub enum StringSegment {
    Token(Token),
    Interpolation(Syn),
}

#[derive(Sentinel, Clone, Copy, Debug)]
pub enum SynSentinel {}

pub type Syn = Index<SynSentinel>;
pub type Syntax = KeyVec<SynSentinel, SynData>;

pub const ROOT_SYN: Syn = Syn::from_u32_index(0);
