use crate::token::{TokenKind, Tokens};

pub fn lex(source: &str) -> Tokens {
    let mut chars = source.char_indices().peekable();

    let mut interpolations_curly_nesting = Vec::<u32>::new();
    let mut in_string = false;

    let tokens = std::iter::from_fn(move || {
        loop {
            if in_string {
                let (start, char) = chars.next()?;

                match char {
                    '"' => {
                        in_string = false;
                        return Some((start, TokenKind::StringEnd));
                    }
                    '\\' => {
                        match chars.next() {
                            // TODO: advance multiple characters for certain escape sequences
                            _ => {}
                        };
                        return Some((start, TokenKind::StringEscape));
                    }
                    '{' => {
                        interpolations_curly_nesting.push(0);
                        in_string = false;
                        return Some((start, TokenKind::InterpolationStart));
                    }
                    _ => {
                        while chars
                            .next_if(|(_, ch)| !matches!(ch, '"' | '\\' | '{'))
                            .is_some()
                        {}

                        return Some((start, TokenKind::StringSegment));
                    }
                }
            }

            while let Some(_) = chars.next_if(|(_, ch)| ch.is_whitespace()) {}

            let (start, char) = chars.next()?;

            let token = match char {
                '"' => {
                    in_string = true;
                    TokenKind::StringStart
                }

                '=' if chars.next_if(|(_, ch)| *ch == '>').is_some() => TokenKind::EqualGreater,
                '=' => TokenKind::Equal,
                '+' => TokenKind::Plus,
                '-' if chars.next_if(|(_, ch)| *ch == '>').is_some() => TokenKind::HyphenGreater,
                '-' => TokenKind::Hyphen,
                ',' => TokenKind::Comma,
                ';' => TokenKind::Semicolon,
                ':' => TokenKind::Colon,
                '.' => TokenKind::Dot,

                '(' => TokenKind::LeftParen,
                ')' => TokenKind::RightParen,
                '{' => {
                    if let Some(curly_count) = interpolations_curly_nesting.last_mut() {
                        *curly_count += 1;
                    }

                    TokenKind::LeftCurly
                }
                '}' => {
                    if let Some(curly_count) = interpolations_curly_nesting.last_mut() {
                        if let Some(new_curly_count) = curly_count.checked_sub(1) {
                            *curly_count = new_curly_count;
                            TokenKind::RightCurly
                        } else {
                            interpolations_curly_nesting.pop();
                            in_string = true;
                            TokenKind::InterpolationEnd
                        }
                    } else {
                        TokenKind::RightCurly
                    }
                }

                // TODO: allow multi-line comments (with `/**/` ?)
                // QUESTION: should we use `#` or `//` for single line comments
                // - `#` is the unix standard, if we use it, we don't have to
                //   care about shebangs, it is also concise
                // - `//` is the c-like language standard, it will be more
                //   familiar to most developpers (python uses `#` so it's at
                //   least a BIG language using it)
                //
                // There is also the question of doc comments, if we use a
                // distinct syntax like rust, the `///` is more established and
                // something like `##` seems a bit weird.
                //
                // Also crate comments, `//!` in rust would look like shebang
                // `#!` which is really not good since they crate comments
                // usually start at the first line.
                //
                // There is also `--` as an honorable mention...
                //
                // `#` could also be used as attribute syntax `#[attr]` or
                // `#attr`, which would conflict with the comment syntax.
                '/' if chars.next_if(|(_, ch)| *ch == '/').is_some() => {
                    while let Some(_) = chars.next_if(|(_, ch)| *ch != '\n') {}
                    continue;
                }
                '#' => {
                    while let Some(_) = chars.next_if(|(_, ch)| *ch != '\n') {}
                    continue;
                }

                '0'..='9' => {
                    while let Some(_) = chars.next_if(|(_, ch)| matches!(ch, '0'..='9')) {}
                    TokenKind::Number
                }
                _ if unicode_ident::is_xid_start(char) => {
                    while let Some(_) = chars.next_if(|(_, ch)| unicode_ident::is_xid_continue(*ch))
                    {
                    }

                    match &source[start..chars.peek().map(|(i, _)| *i).unwrap_or(source.len())] {
                        "let" => TokenKind::Let,
                        "loop" => TokenKind::Loop,
                        "if" => TokenKind::If,
                        "then" => TokenKind::Then,
                        "else" => TokenKind::Else,
                        "false" => TokenKind::False,
                        "true" => TokenKind::True,
                        _ => TokenKind::Ident,
                    }
                }
                _ => panic!("{char}"),
            };

            break Some((start, token));
        }
    });

    Tokens::from_vec(tokens.collect())
}
