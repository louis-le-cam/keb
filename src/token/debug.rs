use colored::Colorize as _;

use crate::token::{
    TokenKind, Tokens, parse_identifer, parse_string_escape, parse_string_segment, parse_u64,
    token_length,
};

pub fn debug(source: &str, tokens: &Tokens) {
    let mut offset = 0;
    for (token, (i, kind)) in tokens.entries() {
        print!("{}", source[offset..i].white().italic());
        offset = i + token_length(source, tokens, token);

        let text = match kind {
            TokenKind::EqualGreater => "=>".bright_yellow(),
            TokenKind::HyphenGreater => "->".bright_yellow(),
            TokenKind::Equal => "=".bright_yellow(),
            TokenKind::Plus => "+".bright_yellow(),
            TokenKind::Hyphen => "-".bright_yellow(),
            TokenKind::Star => "*".bright_yellow(),
            TokenKind::Slash => "/".bright_yellow(),

            TokenKind::Comma => ",".white(),
            TokenKind::Semicolon => ";".white(),
            TokenKind::Colon => ":".white(),
            TokenKind::Dot => ".".white(),

            TokenKind::LeftParen => "(".bright_white(),
            TokenKind::RightParen => ")".bright_white(),
            TokenKind::LeftCurly => "{".bright_white(),
            TokenKind::RightCurly => "}".bright_white(),

            TokenKind::Number => parse_u64(source, &tokens.offsets, token)
                .to_string()
                .bright_purple(),
            TokenKind::Ident => parse_identifer(source, &tokens.offsets, token).bright_cyan(),
            TokenKind::Let => "let".bright_red(),
            TokenKind::Mut => "mut".bright_red(),
            TokenKind::Loop => "loop".bright_red(),
            TokenKind::Match => "match".bright_red(),
            TokenKind::If => "if".bright_red(),
            TokenKind::Then => "then".bright_red(),
            TokenKind::Else => "else".bright_red(),
            TokenKind::False => "false".bright_purple(),
            TokenKind::True => "true".bright_purple(),

            TokenKind::StringStart => "\"".bright_yellow().bold(),
            TokenKind::StringEnd => "\"".bright_yellow().bold(),
            TokenKind::StringSegment => parse_string_segment(source, &tokens.offsets, token)
                .bright_yellow()
                .underline(),
            TokenKind::StringEscape => parse_string_escape(source, &tokens.offsets, token)
                .escape_default()
                .to_string()
                .bright_yellow()
                .bold(),
            TokenKind::InterpolationStart => "{".bright_yellow().bold(),
            TokenKind::InterpolationEnd => "}".bright_yellow().bold(),
        };

        print!("{text}");
    }

    println!();
}
