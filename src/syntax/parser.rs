use std::iter::Peekable;

use crate::{
    key_vec::Val,
    syntax::{self, Syn, SynData, Syns},
    token::{Token, TokenKind, Tokens},
};

pub fn parse(tokens: &Tokens) -> Syns {
    let mut parser = Parser {
        tokens: tokens
            .entries()
            .map(|(token, (_, kind))| (token, *kind))
            .peekable(),
        syns: Syns::default(),
    };

    parser.parse_root();

    parser.syns
}

struct Parser<I: Iterator<Item = (Token, TokenKind)>> {
    tokens: Peekable<I>,
    syns: Syns,
}

impl<I: Iterator<Item = (Token, TokenKind)>> Parser<I> {
    fn parse_root(&mut self) {
        let root = self.syns.push(SynData::Root(Vec::new()));
        assert_eq!(root, syntax::ROOT_SYN);

        let syns = std::iter::from_fn(|| {
            while let Some(_) = self
                .tokens
                .next_if(|(_, token)| matches!(token, TokenKind::Semicolon))
            {}

            self.parse_tuple()
        })
        .collect();

        match self.syns.get_mut(root) {
            Val::None => panic!(),
            Val::Value(syn_data) => *syn_data = SynData::Root(syns),
        }
    }

    fn parse_chain(&mut self) -> Option<Syn> {
        let syn = self.parse_tuple()?;

        let Some((_, TokenKind::Semicolon)) = self.tokens.peek() else {
            return Some(syn);
        };

        self.tokens.next();

        let mut syns = vec![syn];

        let closed = loop {
            match self.parse_tuple() {
                Some(syn) => syns.push(syn),
                None => break true,
            };

            match self.tokens.peek() {
                Some((_, TokenKind::Semicolon)) => {}
                _ => break false,
            };
        };

        Some(self.syns.push(if closed {
            SynData::ChainClosed(syns)
        } else {
            SynData::ChainOpen(syns)
        }))
    }

    fn parse_tuple(&mut self) -> Option<Syn> {
        let syn = self.parse_function()?;

        let Some((_, TokenKind::Comma)) = self.tokens.peek() else {
            return Some(syn);
        };

        self.tokens.next();

        let mut nodes = vec![syn];

        loop {
            nodes.push(self.parse_function().unwrap());

            match self.tokens.peek() {
                Some((_, TokenKind::Comma)) => {}
                _ => break,
            };
        }

        Some(self.syns.push(SynData::Tuple(nodes)))
    }

    fn parse_function(&mut self) -> Option<Syn> {
        let syn = self.parse_return_ascription()?;

        Some(match self.tokens.peek() {
            Some((_, TokenKind::EqualGreater)) => {
                self.tokens.next();
                let body = self.parse_function().unwrap();
                self.syns.push(SynData::Function { pattern: syn, body })
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
                self.syns.push(SynData::ReturnAscription { syn, type_ })
            }
            _ => syn,
        })
    }

    fn parse_application(&mut self) -> Option<Syn> {
        let syn = self.parse_additive()?;

        Some(match self.parse_application() {
            Some(argument) => self.syns.push(SynData::Application {
                function: syn,
                argument,
            }),
            None => syn,
        })
    }

    fn parse_additive(&mut self) -> Option<Syn> {
        let mut syn = self.parse_ascription()?;

        loop {
            match self.tokens.peek() {
                Some((_, TokenKind::Plus)) => {
                    self.tokens.next();
                    let rhs = self.parse_ascription().unwrap();
                    syn = self.syns.push(SynData::Add(syn, rhs))
                }
                Some((_, TokenKind::Hyphen)) => {
                    self.tokens.next();
                    let rhs = self.parse_ascription().unwrap();
                    syn = self.syns.push(SynData::Subtract(syn, rhs))
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
                self.syns.push(SynData::Ascription { syn, type_ })
            }
            _ => syn,
        })
    }

    fn parse_access(&mut self) -> Option<Syn> {
        let mut syn = self.parse_terminal()?;

        while let Some((_, TokenKind::Dot)) = self.tokens.peek() {
            self.tokens.next();
            let key = self.parse_terminal().unwrap();
            syn = self.syns.push(SynData::Access { syn, key });
        }

        Some(syn)
    }

    fn parse_terminal(&mut self) -> Option<Syn> {
        let &(token, kind) = self.tokens.peek()?;
        Some(match kind {
            TokenKind::Let => {
                self.tokens.next();

                let pattern = self.parse_chain().unwrap();

                let Some((_, TokenKind::Equal)) = self.tokens.next() else {
                    panic!();
                };

                let value = self.parse_tuple().unwrap();

                self.syns.push(SynData::Binding {
                    pattern: pattern,
                    value: value,
                })
            }
            TokenKind::False => {
                self.tokens.next();
                self.syns.push(SynData::False(token))
            }
            TokenKind::True => {
                self.tokens.next();
                self.syns.push(SynData::True(token))
            }
            TokenKind::Ident => {
                self.tokens.next();
                self.syns.push(SynData::Ident(token))
            }
            TokenKind::Number => {
                self.tokens.next();
                self.syns.push(SynData::Number(token))
            }
            TokenKind::LeftParen => {
                self.tokens.next();

                let expr = self.parse_chain();

                let Some((_, TokenKind::RightParen)) = self.tokens.next() else {
                    panic!();
                };

                match expr {
                    Some(expr) => self.syns.push(SynData::Paren(expr)),
                    None => self.syns.push(SynData::EmptyParen(token)),
                }
            }
            TokenKind::LeftCurly => {
                self.tokens.next();

                let expr = self.parse_chain();

                let Some((_, TokenKind::RightCurly)) = self.tokens.next() else {
                    panic!();
                };

                match expr {
                    Some(expr) => self.syns.push(SynData::Curly(expr)),
                    None => self.syns.push(SynData::EmptyCurly(token)),
                }
            }
            TokenKind::EqualGreater
            | TokenKind::Equal
            | TokenKind::Plus
            | TokenKind::HyphenGreater
            | TokenKind::Hyphen
            | TokenKind::Comma
            | TokenKind::Semicolon
            | TokenKind::Colon
            | TokenKind::Dot
            | TokenKind::RightParen
            | TokenKind::RightCurly => return None,
        })
    }
}
