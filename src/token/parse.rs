use crate::token::{Token, TokenOffsets, Tokens};

use super::TokenKind;

pub fn parse_identifer<'a>(source: &'a str, tokens: &TokenOffsets, token: Token) -> &'a str {
    let source_from_token = &source[tokens[token]..];

    assert!(
        source_from_token
            .chars()
            .next()
            .is_some_and(unicode_ident::is_xid_start)
    );

    &source_from_token[..source_from_token
        .char_indices()
        .skip(1)
        .find(|(_, ch)| !unicode_ident::is_xid_continue(*ch))
        .map(|(i, _)| i)
        .unwrap_or(source_from_token.len())]
}

pub fn parse_u64(source: &str, tokens: &TokenOffsets, token: Token) -> u64 {
    let source_from_token = &source[tokens[token]..];

    assert!(
        source_from_token
            .chars()
            .next()
            .is_some_and(|ch| matches!(ch, '0'..='9'))
    );

    source_from_token[..source_from_token
        .char_indices()
        .skip(1)
        .find(|(_, ch)| !matches!(ch, '0'..='9'))
        .map(|(i, _)| i)
        .unwrap_or(source_from_token.len())]
        .parse()
        .unwrap()
}

pub fn parse_string_segment<'a>(source: &'a str, tokens: &TokenOffsets, token: Token) -> &'a str {
    let source_from_token = &source[tokens[token]..];

    &source_from_token[..source_from_token
        .char_indices()
        .skip(1)
        .find(|(_, ch)| matches!(ch, '"' | '\\' | '{'))
        .map(|(i, _)| i)
        .unwrap_or(source_from_token.len())]
}

pub fn parse_string_escape(source: &str, tokens: &TokenOffsets, token: Token) -> char {
    let source_from_token = &source[tokens[token]..];

    assert_eq!(source_from_token.chars().next(), Some('\\'));

    match source_from_token.chars().nth(1).unwrap() {
        'n' => '\n',
        '\\' => '\\',
        '{' => '{',
        _ => panic!(),
    }
}

pub fn token_length(source: &str, tokens: &Tokens, token: Token) -> usize {
    match tokens.kinds[token] {
        TokenKind::EqualGreater | TokenKind::HyphenGreater => 2,
        TokenKind::Equal
        | TokenKind::Plus
        | TokenKind::Hyphen
        | TokenKind::Star
        | TokenKind::Slash
        | TokenKind::Comma
        | TokenKind::Semicolon
        | TokenKind::Colon
        | TokenKind::Dot
        | TokenKind::LeftParen
        | TokenKind::RightParen
        | TokenKind::LeftCurly
        | TokenKind::RightCurly => 1,
        TokenKind::Number => {
            let source_from_token = &source[tokens.offsets[token]..];
            source_from_token
                .char_indices()
                .skip(1)
                .find(|(_, ch)| !matches!(ch, '0'..='9'))
                .map(|(i, _)| i)
                .unwrap_or(source_from_token.len())
        }
        TokenKind::Ident
        | TokenKind::Let
        | TokenKind::Mut
        | TokenKind::Loop
        | TokenKind::Match
        | TokenKind::If
        | TokenKind::Then
        | TokenKind::Else
        | TokenKind::False
        | TokenKind::True => parse_identifer(source, &tokens.offsets, token).len(),
        TokenKind::StringStart | TokenKind::StringEnd => 1,
        TokenKind::StringSegment => parse_string_segment(source, &tokens.offsets, token).len(),
        TokenKind::StringEscape => 2,
        TokenKind::InterpolationStart | TokenKind::InterpolationEnd => 1,
    }
}
