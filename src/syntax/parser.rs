use std::iter::Peekable;

use crate::{
    syntax::{self, StringSegment, Syn, SynData, Syntax},
    token::{Token, TokenKind, TokenKinds},
};

pub fn parse(tokens: &TokenKinds) -> Syntax {
    let mut parser = Parser {
        tokens: tokens
            .entries()
            .map(|(token, kind)| (token, *kind))
            .peekable(),
        syntax: Syntax::default(),
    };

    parser.parse_root();

    parser.syntax
}

struct Parser<I: Iterator<Item = (Token, TokenKind)>> {
    tokens: Peekable<I>,
    syntax: Syntax,
}

impl<I: Iterator<Item = (Token, TokenKind)>> Parser<I> {
    fn parse_root(&mut self) {
        let root = self.syntax.push(SynData::Root(Vec::new()));
        assert_eq!(root, syntax::ROOT_SYN);

        let syns = std::iter::from_fn(|| {
            while let Some(_) = self
                .tokens
                .next_if(|(_, token)| matches!(token, TokenKind::Semicolon))
            {}

            self.parse_tuple()
        })
        .collect();

        self.syntax[root] = SynData::Root(syns);
    }

    fn parse_chain(&mut self) -> Option<Syn> {
        let syn = self.parse_assignment()?;

        let Some((_, TokenKind::Semicolon)) = self.tokens.peek() else {
            return Some(syn);
        };

        self.tokens.next();

        let mut syns = vec![syn];

        let closed = loop {
            match self.parse_assignment() {
                Some(syn) => syns.push(syn),
                None => break true,
            };

            match self.tokens.peek() {
                Some((_, TokenKind::Semicolon)) => self.tokens.next(),
                _ => break false,
            };
        };

        Some(self.syntax.push(if closed {
            SynData::ChainClosed(syns)
        } else {
            SynData::ChainOpen(syns)
        }))
    }

    fn parse_assignment(&mut self) -> Option<Syn> {
        let pattern = self.parse_tuple()?;

        let Some((_, TokenKind::Equal)) = self.tokens.peek() else {
            return Some(pattern);
        };

        self.tokens.next();

        let value = self.parse_tuple()?;

        Some(self.syntax.push(SynData::Assignment { pattern, value }))
    }

    fn parse_tuple(&mut self) -> Option<Syn> {
        let syn = self.parse_function()?;

        let Some((_, TokenKind::Comma)) = self.tokens.peek() else {
            return Some(syn);
        };

        self.tokens.next();

        let mut syns = vec![syn];

        loop {
            let Some(syn) = self.parse_function() else {
                break;
            };

            syns.push(syn);

            match self.tokens.peek() {
                Some((_, TokenKind::Comma)) => {
                    self.tokens.next();
                }
                _ => break,
            };
        }

        Some(self.syntax.push(SynData::Tuple(syns)))
    }

    fn parse_function(&mut self) -> Option<Syn> {
        let syn = self.parse_return_ascription()?;

        Some(match self.tokens.peek() {
            Some((_, TokenKind::EqualGreater)) => {
                self.tokens.next();
                let body = self.parse_function().unwrap();
                self.syntax.push(SynData::Function { pattern: syn, body })
            }
            _ => syn,
        })
    }

    fn parse_return_ascription(&mut self) -> Option<Syn> {
        let syn = self.parse_application()?;

        Some(match self.tokens.peek() {
            Some((_, TokenKind::HyphenGreater)) => {
                self.tokens.next();
                let type_ = self.parse_return_ascription().unwrap();
                self.syntax.push(SynData::ReturnAscription { syn, type_ })
            }
            _ => syn,
        })
    }

    fn parse_application(&mut self) -> Option<Syn> {
        let syn = self.parse_comparative()?;

        Some(match self.parse_application() {
            Some(argument) => self.syntax.push(SynData::Application {
                function: syn,
                argument,
            }),
            None => syn,
        })
    }

    fn parse_comparative(&mut self) -> Option<Syn> {
        let mut syn = self.parse_additive()?;

        loop {
            match self.tokens.peek() {
                Some((_, TokenKind::DoubleEqual)) => {
                    self.tokens.next();
                    let rhs = self.parse_additive().unwrap();
                    syn = self.syntax.push(SynData::Equal(syn, rhs))
                }
                _ => break,
            }
        }

        Some(syn)
    }

    fn parse_additive(&mut self) -> Option<Syn> {
        let mut syn = self.parse_multiplicative()?;

        loop {
            match self.tokens.peek() {
                Some((_, TokenKind::Plus)) => {
                    self.tokens.next();
                    let rhs = self.parse_multiplicative().unwrap();
                    syn = self.syntax.push(SynData::Add(syn, rhs))
                }
                Some((_, TokenKind::Hyphen)) => {
                    self.tokens.next();
                    let rhs = self.parse_multiplicative().unwrap();
                    syn = self.syntax.push(SynData::Subtract(syn, rhs))
                }
                _ => break,
            }
        }

        Some(syn)
    }

    fn parse_multiplicative(&mut self) -> Option<Syn> {
        let mut syn = self.parse_ascription()?;

        loop {
            match self.tokens.peek() {
                Some((_, TokenKind::Star)) => {
                    self.tokens.next();
                    let rhs = self.parse_ascription().unwrap();
                    syn = self.syntax.push(SynData::Multiply(syn, rhs))
                }
                Some((_, TokenKind::Slash)) => {
                    self.tokens.next();
                    let rhs = self.parse_ascription().unwrap();
                    syn = self.syntax.push(SynData::Divide(syn, rhs))
                }
                _ => break,
            }
        }

        Some(syn)
    }

    fn parse_ascription(&mut self) -> Option<Syn> {
        let syn = self.parse_access()?;

        Some(match self.tokens.peek() {
            Some((_, TokenKind::Colon)) => {
                self.tokens.next();
                let type_ = self.parse_ascription().unwrap();
                self.syntax.push(SynData::Ascription { syn, type_ })
            }
            _ => syn,
        })
    }

    fn parse_access(&mut self) -> Option<Syn> {
        let mut syn = self.parse_terminal()?;

        while let Some((_, TokenKind::Dot)) = self.tokens.peek() {
            self.tokens.next();
            let key = self.parse_terminal().unwrap();
            syn = self.syntax.push(SynData::Access { syn, key });
        }

        Some(syn)
    }

    fn parse_terminal(&mut self) -> Option<Syn> {
        let &(token, kind) = self.tokens.peek()?;
        Some(match kind {
            TokenKind::LeftParen => self.parse_paren(),
            TokenKind::LeftCurly => self.parse_curly(),

            TokenKind::Number => {
                self.tokens.next();
                self.syntax.push(SynData::Number(token))
            }
            TokenKind::Ident => {
                self.tokens.next();
                self.syntax.push(SynData::Ident(token))
            }
            TokenKind::Let => self.parse_let(),
            TokenKind::Mut => {
                self.tokens.next();
                let pattern = self.parse_return_ascription().unwrap();
                self.syntax.push(SynData::Mut { pattern })
            }
            TokenKind::Loop => {
                self.tokens.next();
                let body = self.parse_application().unwrap();
                self.syntax.push(SynData::Loop(body))
            }
            TokenKind::Match => self.parse_match(),
            TokenKind::If => self.parse_if(),
            TokenKind::False => {
                self.tokens.next();
                self.syntax.push(SynData::False(token))
            }
            TokenKind::True => {
                self.tokens.next();
                self.syntax.push(SynData::True(token))
            }

            TokenKind::StringStart => self.parse_string(),

            TokenKind::EqualGreater
            | TokenKind::HyphenGreater
            | TokenKind::DoubleEqual
            | TokenKind::Equal
            | TokenKind::Plus
            | TokenKind::Hyphen
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Comma
            | TokenKind::Semicolon
            | TokenKind::Colon
            | TokenKind::Dot
            | TokenKind::RightParen
            | TokenKind::RightCurly
            | TokenKind::Then
            | TokenKind::Else
            | TokenKind::StringEnd
            | TokenKind::StringSegment
            | TokenKind::StringEscape
            | TokenKind::InterpolationStart
            | TokenKind::InterpolationEnd => return None,
        })
    }

    fn parse_paren(&mut self) -> Syn {
        let Some((token, TokenKind::LeftParen)) = self.tokens.next() else {
            panic!()
        };

        let expr = self.parse_chain();

        let Some((_, TokenKind::RightParen)) = self.tokens.next() else {
            panic!();
        };

        match expr {
            Some(expr) => self.syntax.push(SynData::Paren(expr)),
            None => self.syntax.push(SynData::EmptyParen(token)),
        }
    }

    fn parse_curly(&mut self) -> Syn {
        let Some((token, TokenKind::LeftCurly)) = self.tokens.next() else {
            panic!()
        };

        let expr = self.parse_chain();

        let Some((_, TokenKind::RightCurly)) = self.tokens.next() else {
            panic!();
        };

        match expr {
            Some(expr) => self.syntax.push(SynData::Curly(expr)),
            None => self.syntax.push(SynData::EmptyCurly(token)),
        }
    }

    fn parse_let(&mut self) -> Syn {
        let Some((_, TokenKind::Let)) = self.tokens.next() else {
            panic!()
        };

        let pattern = self.parse_tuple().unwrap();

        let Some((_, TokenKind::Equal)) = self.tokens.next() else {
            panic!();
        };

        let value = self.parse_tuple().unwrap();

        self.syntax.push(SynData::Binding {
            pattern: pattern,
            value: value,
        })
    }

    fn parse_match(&mut self) -> Syn {
        let Some((_, TokenKind::Match)) = self.tokens.next() else {
            panic!()
        };

        let content = self.parse_curly();

        self.syntax.push(SynData::Match(content))
    }

    fn parse_if(&mut self) -> Syn {
        let Some((_, TokenKind::If)) = self.tokens.next() else {
            panic!()
        };

        let condition = self.parse_application().unwrap();

        let Some((_, TokenKind::Then)) = self.tokens.next() else {
            panic!()
        };

        let then = self.parse_application().unwrap();

        if self
            .tokens
            .next_if(|(_, token)| *token == TokenKind::Else)
            .is_none()
        {
            self.syntax.push(SynData::If { condition, then })
        } else {
            let else_ = self.parse_application().unwrap();

            self.syntax.push(SynData::IfElse {
                condition,
                then,
                else_,
            })
        }
    }

    fn parse_string(&mut self) -> Syn {
        let Some((_, TokenKind::StringStart)) = self.tokens.next() else {
            panic!()
        };

        let mut segments = Vec::new();

        loop {
            let (token, token_kind) = self.tokens.next().unwrap();
            match token_kind {
                TokenKind::StringSegment | TokenKind::StringEscape => {
                    segments.push(StringSegment::Token(token))
                }
                TokenKind::InterpolationStart => {
                    segments.push(StringSegment::Interpolation(self.parse_chain().unwrap()));

                    let Some((_, TokenKind::InterpolationEnd)) = self.tokens.next() else {
                        panic!();
                    };
                }
                TokenKind::StringEnd => break self.syntax.push(SynData::String(segments)),
                _ => panic!(),
            }
        }
    }
}
