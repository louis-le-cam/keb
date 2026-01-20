use colored::Colorize as _;

use crate::token::{TokenKind, Tokens, parse_identifer, parse_u64};

pub fn debug(source: &str, tokens: &Tokens) {
    for (token, (i, kind)) in tokens.entries() {
        for whitespace in source[..*i]
            .chars()
            .rev()
            .take_while(|char| char.is_whitespace())
            .collect::<Vec<char>>()
            .into_iter()
            .rev()
        {
            print!("{whitespace}");
        }

        let text = match kind {
            TokenKind::Ident => parse_identifer(source, tokens, token).bright_white(),
            TokenKind::Number => parse_u64(source, tokens, token).to_string().bright_purple(),
            TokenKind::Let => "let".bright_red(),
            TokenKind::Loop => "loop".bright_red(),
            TokenKind::If => "if".bright_red(),
            TokenKind::Then => "then".bright_red(),
            TokenKind::Else => "else".bright_red(),
            TokenKind::False => "false".bright_purple(),
            TokenKind::True => "true".bright_purple(),
            TokenKind::EqualGreater => "=>".bright_yellow(),
            TokenKind::Equal => "=".bright_yellow(),
            TokenKind::Plus => "+".bright_yellow(),
            TokenKind::HyphenGreater => "->".bright_yellow(),
            TokenKind::Hyphen => "-".bright_yellow(),
            TokenKind::Comma => ",".white(),
            TokenKind::Semicolon => ";".white(),
            TokenKind::Colon => ":".white(),
            TokenKind::Dot => ".".white(),
            TokenKind::LeftParen => "(".bright_white(),
            TokenKind::RightParen => ")".bright_white(),
            TokenKind::LeftCurly => "{".bright_white(),
            TokenKind::RightCurly => "}".bright_white(),
        };

        print!("{text}");
    }

    println!();
}
