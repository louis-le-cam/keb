use std::fmt::Display;

use colored::Colorize as _;

use crate::token::{
    Token, TokenKind, Tokens, parse_identifer, parse_string_escape, parse_string_segment,
    parse_u64, token_length,
};

pub fn debug(source: &str, tokens: &Tokens) {
    let mut offset = 0;
    for (token, (i, _)) in tokens.entries() {
        print!("{}", source[offset..i].white().italic());
        offset = i + token_length(source, tokens, token);

        print!("{}", token.debug(source, tokens));
    }

    println!();
}

impl Token {
    pub fn debug(self, source: &str, tokens: &Tokens) -> impl Display {
        std::fmt::from_fn(move |f| match tokens.kinds[self] {
            TokenKind::EqualGreater => write!(f, "{}", "=>".bright_yellow()),
            TokenKind::HyphenGreater => write!(f, "{}", "->".bright_yellow()),
            TokenKind::DoubleEqual => write!(f, "{}", "==".bright_yellow()),

            TokenKind::Equal => write!(f, "{}", "=".bright_yellow()),
            TokenKind::Plus => write!(f, "{}", "+".bright_yellow()),
            TokenKind::Hyphen => write!(f, "{}", "-".bright_yellow()),
            TokenKind::Star => write!(f, "{}", "*".bright_yellow()),
            TokenKind::Slash => write!(f, "{}", "/".bright_yellow()),

            TokenKind::Comma => write!(f, "{}", ",".white()),
            TokenKind::Semicolon => write!(f, "{}", ";".white()),
            TokenKind::Colon => write!(f, "{}", ":".white()),
            TokenKind::Dot => write!(f, "{}", ".".white()),

            TokenKind::LeftParen => write!(f, "{}", "(".bright_white()),
            TokenKind::RightParen => write!(f, "{}", ")".bright_white()),
            TokenKind::LeftCurly => write!(f, "{}", "{".bright_white()),
            TokenKind::RightCurly => write!(f, "{}", "}".bright_white()),

            TokenKind::Number => write!(
                f,
                "{}",
                parse_u64(source, &tokens.offsets, self)
                    .to_string()
                    .bright_purple()
            ),
            TokenKind::Ident => write!(
                f,
                "{}",
                parse_identifer(source, &tokens.offsets, self).bright_cyan()
            ),
            TokenKind::Let => write!(f, "{}", "let".bright_red()),
            TokenKind::Mut => write!(f, "{}", "mut".bright_red()),
            TokenKind::Loop => write!(f, "{}", "loop".bright_red()),
            TokenKind::Match => write!(f, "{}", "match".bright_red()),
            TokenKind::If => write!(f, "{}", "if".bright_red()),
            TokenKind::Then => write!(f, "{}", "then".bright_red()),
            TokenKind::Else => write!(f, "{}", "else".bright_red()),
            TokenKind::False => write!(f, "{}", "false".bright_purple()),
            TokenKind::True => write!(f, "{}", "true".bright_purple()),

            TokenKind::StringStart => write!(f, "{}", "\"".bright_yellow().bold()),
            TokenKind::StringEnd => write!(f, "{}", "\"".bright_yellow().bold()),
            TokenKind::StringSegment => write!(
                f,
                "{}",
                parse_string_segment(source, &tokens.offsets, self)
                    .bright_yellow()
                    .underline()
            ),
            TokenKind::StringEscape => write!(
                f,
                "{}",
                parse_string_escape(source, &tokens.offsets, self)
                    .escape_default()
                    .to_string()
                    .bright_yellow()
                    .bold()
            ),
            TokenKind::InterpolationStart => write!(f, "{}", "{".bright_yellow().bold()),
            TokenKind::InterpolationEnd => write!(f, "{}", "}".bright_yellow().bold()),
        })
    }
}
