use crate::key_vec::{Index, KeyVec, Sentinel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Ident,
    Number,

    StringStart,
    StringEnd,
    StringSegment,
    StringEscape,
    InterpolationStart,
    InterpolationEnd,

    Let,
    Loop,
    If,
    Then,
    Else,
    False,
    True,

    EqualGreater,
    Equal,
    Plus,
    HyphenGreater,
    Hyphen,
    Comma,
    Semicolon,
    Colon,
    Dot,

    LeftParen,
    RightParen,
    LeftCurly,
    RightCurly,
}

#[derive(Sentinel, Clone, Copy, Debug)]
pub enum TokenSentinel {}

pub type Token = Index<TokenSentinel>;
pub type Tokens = KeyVec<TokenSentinel, (usize, TokenKind)>;
