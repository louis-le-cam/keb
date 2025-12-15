mod lexer;
mod token;

use crate::key_vec::Val;

pub use self::{
    lexer::lex,
    token::{Token, TokenKind, Tokens},
};

pub fn parse_identifer<'a>(source: &'a str, tokens: &Tokens, token: Token) -> &'a str {
    let (offset, kind) = match tokens.get(token) {
        Val::None => panic!(),
        Val::Value(token) => *token,
    };

    let source_from_token = &source[offset..];

    assert!(
        source_from_token
            .chars()
            .next()
            .is_some_and(unicode_ident::is_xid_start)
    );
    assert_eq!(kind, TokenKind::Ident);

    &source_from_token[..source_from_token
        .char_indices()
        .skip(1)
        .find(|(_, char)| !unicode_ident::is_xid_continue(*char))
        .map(|(i, _)| i)
        .unwrap_or(source_from_token.len())]
}

pub fn parse_u64(source: &str, tokens: &Tokens, token: Token) -> u64 {
    let (offset, kind) = match tokens.get(token) {
        Val::None => panic!(),
        Val::Value(token) => *token,
    };

    let source_from_token = &source[offset..];

    assert!(
        source_from_token
            .chars()
            .next()
            .is_some_and(|char| matches!(char, '0'..='9'))
    );
    assert_eq!(kind, TokenKind::Number);

    source_from_token[..source_from_token
        .char_indices()
        .skip(1)
        .find(|(_, char)| !matches!(char, '0'..='9'))
        .map(|(i, _)| i)
        .unwrap_or(source_from_token.len())]
        .parse()
        .unwrap()
}
