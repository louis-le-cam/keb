use crate::key_vec::{Index, KeyVec, Sentinel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    EqualGreater,
    HyphenGreater,

    Equal,
    Plus,
    Hyphen,
    Star,
    Slash,

    Comma,
    Semicolon,
    Colon,
    Dot,

    LeftParen,
    RightParen,
    LeftCurly,
    RightCurly,

    Number,
    Ident,
    Let,
    Loop,
    If,
    Then,
    Else,
    False,
    True,

    StringStart,
    StringEnd,
    StringSegment,
    StringEscape,
    InterpolationStart,
    InterpolationEnd,
}

#[derive(Sentinel, Clone, Copy, Debug)]
pub enum TokenSentinel {}

pub type Token = Index<TokenSentinel>;
pub type Tokens = KeyVec<TokenSentinel, (usize, TokenKind)>;
