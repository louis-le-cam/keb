use crate::token::{TokenKind, TokenKinds, TokenOffsets, Tokens};

pub fn lex(source: &str) -> Tokens {
    let mut chars = source.char_indices().peekable();

    let mut interpolations_curly_nesting = Vec::<u32>::new();
    let mut in_string = false;

    let tokens = std::iter::from_fn(move || {
        'outer: loop {
            let (start, char) = chars.next()?;

            let token = match char {
                '"' if in_string => {
                    in_string = false;
                    TokenKind::StringEnd
                }
                '\\' if in_string => match chars.next().unwrap().1 {
                    'n' | '\\' | '{' => TokenKind::StringEscape,
                    // TODO: Support more escape sequence (unicode, hexadecimal, ...)
                    _ => panic!(),
                },
                '{' if in_string => {
                    interpolations_curly_nesting.push(0);
                    in_string = false;
                    TokenKind::InterpolationStart
                }
                _ if in_string => {
                    while chars
                        .next_if(|(_, ch)| !matches!(ch, '"' | '\\' | '{'))
                        .is_some()
                    {}

                    TokenKind::StringSegment
                }

                '"' => {
                    in_string = true;
                    TokenKind::StringStart
                }

                '#' => {
                    while let Some(_) = chars.next_if(|(_, ch)| *ch != '\n') {}
                    continue;
                }
                // TODO: Should multiline comments be nesteable?
                '(' if chars.next_if(|(_, ch)| *ch == '#').is_some() => loop {
                    match chars.next() {
                        Some((_, '#')) if chars.next_if(|(_, ch)| *ch == ')').is_some() => {
                            continue 'outer;
                        }
                        Some(_) => {}
                        None => panic!(),
                    }
                },

                '=' if chars.next_if(|(_, ch)| *ch == '>').is_some() => TokenKind::EqualGreater,
                '-' if chars.next_if(|(_, ch)| *ch == '>').is_some() => TokenKind::HyphenGreater,
                '=' if chars.next_if(|(_, ch)| *ch == '=').is_some() => TokenKind::DoubleEqual,
                '=' => TokenKind::Equal,
                '+' => TokenKind::Plus,
                '-' => TokenKind::Hyphen,
                '*' => TokenKind::Star,
                '/' => TokenKind::Slash,

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
                '}' => 'token: {
                    let Some(curly_count) = interpolations_curly_nesting.last_mut() else {
                        break 'token TokenKind::RightCurly;
                    };

                    if let Some(new_curly_count) = curly_count.checked_sub(1) {
                        *curly_count = new_curly_count;
                        TokenKind::RightCurly
                    } else {
                        interpolations_curly_nesting.pop();
                        in_string = true;
                        TokenKind::InterpolationEnd
                    }
                }

                _ if char.is_ascii_digit() => {
                    while let Some(_) = chars.next_if(|(_, ch)| ch.is_ascii_digit()) {}
                    TokenKind::Number
                }
                _ if unicode_ident::is_xid_start(char) => {
                    while let Some(_) = chars.next_if(|(_, ch)| unicode_ident::is_xid_continue(*ch))
                    {
                    }

                    let end = chars.peek().map(|(i, _)| *i).unwrap_or(source.len());

                    match &source[start..end] {
                        "let" => TokenKind::Let,
                        "mut" => TokenKind::Mut,
                        "loop" => TokenKind::Loop,
                        "match" => TokenKind::Match,
                        "if" => TokenKind::If,
                        "then" => TokenKind::Then,
                        "else" => TokenKind::Else,
                        "false" => TokenKind::False,
                        "true" => TokenKind::True,
                        _ => TokenKind::Ident,
                    }
                }
                _ if char.is_whitespace() => {
                    while let Some(_) = chars.next_if(|(_, ch)| ch.is_whitespace()) {}
                    continue;
                }
                _ => panic!("Unexpected character: {char:?}"),
            };

            break Some((start, token));
        }
    });

    let (offsets, kinds) = tokens.unzip();

    Tokens {
        offsets: TokenOffsets::from_vec(offsets),
        kinds: TokenKinds::from_vec(kinds),
    }
}
