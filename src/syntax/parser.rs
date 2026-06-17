use std::iter::Peekable;

use crate::{
    key_vec::Sentinels,
    syntax::{ROOT_SYN, Syn, SynKind, SynSentinel, Syntax},
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
        let mut root = self.syntax.push(
            SynKind::Root,
            SynSentinel::None.to_index(),
            SynSentinel::None.to_index(),
        );
        assert_eq!(root, ROOT_SYN);

        loop {
            while self
                .tokens
                .next_if(|(_, token)| matches!(token, TokenKind::Semicolon))
                .is_some()
            {}

            let Some(statement) = self.parse_tuple() else {
                break;
            };

            match Syn::from(self.syntax.lhs[root]).sentinel() {
                Some(SynSentinel::None) => self.syntax.lhs[root] = statement.into(),
                None => {
                    let new_root =
                        self.syntax
                            .push(SynKind::Root, statement, SynSentinel::None.to_index());
                    self.syntax.rhs[root] = new_root.into();
                    root = new_root;
                }
            }
        }
    }

    fn parse_chain(&mut self) -> Option<Syn> {
        let syn = self.parse_assignment()?;

        let Some((_, TokenKind::Semicolon)) = self.tokens.peek() else {
            return Some(syn);
        };

        self.tokens.next();

        let chain = self
            .syntax
            .push(SynKind::Chain, syn, SynSentinel::None.to_index());
        let mut chain_current = chain;

        while let Some(syn) = self.parse_assignment() {
            match self.tokens.peek() {
                Some((_, TokenKind::Semicolon)) => {
                    self.tokens.next();
                    let new_chain =
                        self.syntax
                            .push(SynKind::Chain, syn, SynSentinel::None.to_index());
                    self.syntax.rhs[chain_current] = new_chain.into();
                    chain_current = new_chain;
                }
                _ => {
                    self.syntax.rhs[chain_current] = syn.into();
                    break;
                }
            };
        }

        Some(chain)
    }

    fn parse_assignment(&mut self) -> Option<Syn> {
        let pattern = self.parse_tuple()?;

        let Some((_, TokenKind::Equal)) = self.tokens.peek() else {
            return Some(pattern);
        };

        self.tokens.next();

        let value = self.parse_tuple().unwrap();

        Some(self.syntax.push(SynKind::Assignment, pattern, value))
    }

    fn parse_tuple(&mut self) -> Option<Syn> {
        let syn = self.parse_function()?;

        let Some((_, TokenKind::Comma)) = self.tokens.peek() else {
            return Some(syn);
        };

        self.tokens.next();

        let tuple = self
            .syntax
            .push(SynKind::Tuple, syn, SynSentinel::None.to_index());
        let mut tuple_current = tuple;

        while let Some(syn) = self.parse_function() {
            match self.tokens.peek() {
                Some((_, TokenKind::Comma)) => {
                    self.tokens.next();
                    let new_tuple =
                        self.syntax
                            .push(SynKind::Tuple, syn, SynSentinel::None.to_index());
                    self.syntax.rhs[tuple_current] = new_tuple.into();
                    tuple_current = new_tuple;
                }
                _ => {
                    self.syntax.rhs[tuple_current] = syn.into();
                    break;
                }
            };
        }

        Some(tuple)
    }

    fn parse_function(&mut self) -> Option<Syn> {
        let syn = self.parse_return_ascription()?;

        Some(match self.tokens.peek() {
            Some((_, TokenKind::EqualGreater)) => {
                self.tokens.next();
                let body = self.parse_function().unwrap();
                self.syntax.push(SynKind::Function, syn, body)
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
                self.syntax.push(SynKind::ReturnAscription, syn, type_)
            }
            _ => syn,
        })
    }

    fn parse_application(&mut self) -> Option<Syn> {
        let syn = self.parse_comparative()?;

        Some(match self.parse_application() {
            Some(argument) => self.syntax.push(SynKind::Application, syn, argument),
            None => syn,
        })
    }

    fn parse_comparative(&mut self) -> Option<Syn> {
        let mut syn = self.parse_additive()?;

        while let Some((_, TokenKind::DoubleEqual)) = self.tokens.peek() {
            self.tokens.next();
            let rhs = self.parse_comparative().unwrap();
            syn = self.syntax.push(SynKind::Equal, syn, rhs)
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
                    syn = self.syntax.push(SynKind::Add, syn, rhs)
                }
                Some((_, TokenKind::Hyphen)) => {
                    self.tokens.next();
                    let rhs = self.parse_multiplicative().unwrap();
                    syn = self.syntax.push(SynKind::Subtract, syn, rhs)
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
                    syn = self.syntax.push(SynKind::Multiply, syn, rhs)
                }
                Some((_, TokenKind::Slash)) => {
                    self.tokens.next();
                    let rhs = self.parse_ascription().unwrap();
                    syn = self.syntax.push(SynKind::Divide, syn, rhs)
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
                self.syntax.push(SynKind::Ascription, syn, type_)
            }
            _ => syn,
        })
    }

    fn parse_access(&mut self) -> Option<Syn> {
        let mut syn = self.parse_terminal()?;

        while let Some((_, TokenKind::Dot)) = self.tokens.peek() {
            self.tokens.next();
            let key = self.parse_terminal().unwrap();
            syn = self.syntax.push(SynKind::Access, syn, key);
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
                self.syntax
                    .push(SynKind::Number, token, SynSentinel::None.to_index())
            }
            TokenKind::Ident => {
                self.tokens.next();
                self.syntax
                    .push(SynKind::Ident, token, SynSentinel::None.to_index())
            }
            TokenKind::Let => self.parse_let(),
            TokenKind::Mut => {
                self.tokens.next();
                let pattern = self.parse_return_ascription().unwrap();
                self.syntax
                    .push(SynKind::Mut, pattern, SynSentinel::None.to_index())
            }
            TokenKind::Loop => {
                self.tokens.next();
                let body = self.parse_application().unwrap();
                self.syntax
                    .push(SynKind::Loop, body, SynSentinel::None.to_index())
            }
            TokenKind::Match => self.parse_match(),
            TokenKind::If => self.parse_if(),
            TokenKind::False => {
                self.tokens.next();
                self.syntax
                    .push(SynKind::False, token, SynSentinel::None.to_index())
            }
            TokenKind::True => {
                self.tokens.next();
                self.syntax
                    .push(SynKind::True, token, SynSentinel::None.to_index())
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
            Some(expr) => self.syntax.push(SynKind::Paren, token, expr),
            None => self
                .syntax
                .push(SynKind::Paren, token, SynSentinel::None.to_index()),
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
            Some(expr) => self.syntax.push(SynKind::Curly, token, expr),
            None => self
                .syntax
                .push(SynKind::Curly, token, SynSentinel::None.to_index()),
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

        self.syntax.push(SynKind::Binding, pattern, value)
    }

    fn parse_match(&mut self) -> Syn {
        let Some((_, TokenKind::Match)) = self.tokens.next() else {
            panic!()
        };

        let content = self.parse_curly();

        self.syntax
            .push(SynKind::Match, content, SynSentinel::None.to_index())
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
            self.syntax.push(SynKind::If, condition, then)
        } else {
            let else_ = self.parse_application().unwrap();
            let else_syn = self.syntax.push(SynKind::Else, then, else_);
            self.syntax.push(SynKind::If, condition, else_syn)
        }
    }

    fn parse_string(&mut self) -> Syn {
        let Some((token, TokenKind::StringStart)) = self.tokens.next() else {
            panic!()
        };

        let string = self
            .syntax
            .push(SynKind::String, token, SynSentinel::None.to_index());

        let mut string_current = string;
        loop {
            let (token, token_kind) = self.tokens.next().unwrap();
            match token_kind {
                TokenKind::StringSegment | TokenKind::StringEscape => {
                    let segment = self.syntax.push(
                        SynKind::StringSegment,
                        token,
                        SynSentinel::None.to_index(),
                    );
                    self.syntax.rhs[string_current] = segment.into();
                    string_current = segment;
                }
                TokenKind::InterpolationStart => {
                    let expression = self.parse_chain().unwrap();

                    let interpolation = self.syntax.push(
                        SynKind::StringInterpolation,
                        expression,
                        SynSentinel::None.to_index(),
                    );
                    self.syntax.rhs[string_current] = interpolation.into();
                    string_current = interpolation;

                    let Some((_, TokenKind::InterpolationEnd)) = self.tokens.next() else {
                        panic!();
                    };
                }
                TokenKind::StringEnd => break,
                _ => panic!(),
            }
        }

        string
    }
}
