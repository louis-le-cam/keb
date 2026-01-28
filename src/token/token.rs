use crate::key_vec::{Index, KeyVec, Sentinel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    EqualGreater,
    HyphenGreater,
    DoubleEqual,

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
    Mut,
    Loop,
    Match,
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

pub type TokenOffsets = KeyVec<TokenSentinel, usize>;
pub type TokenKinds = KeyVec<TokenSentinel, TokenKind>;

pub struct Tokens {
    pub offsets: TokenOffsets,
    pub kinds: TokenKinds,
}

impl Tokens {
    pub fn entries(&self) -> impl Iterator<Item = (Token, (usize, TokenKind))> {
        self.offsets
            .entries()
            .zip(self.kinds.entries())
            .map(|((token, offset), (_, kind))| (token, (*offset, *kind)))
    }
}
