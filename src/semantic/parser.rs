use crate::{
    key_vec::Sentinel,
    semantic::{
        self, Sem, SemKind, SemKinds, SemTypes, Semantic, Type, TypeData, TypeSentinel, Types,
        combine_types,
    },
    syntax::{self, Syn, SynData, Syntax},
    token::{self, TokenOffsets, parse_identifer},
};

pub fn parse(source: &str, tokens: &TokenOffsets, syntax: &Syntax) -> (Semantic, Types) {
    let mut parser = Parser {
        source,
        tokens,
        syntax,
        semantic: Semantic {
            kinds: SemKinds::default(),
            types: SemTypes::default(),
        },
        types: Types::default(),
    };
    parser.parse_root();
    (parser.semantic, parser.types)
}

struct Parser<'a> {
    source: &'a str,
    tokens: &'a TokenOffsets,
    syntax: &'a Syntax,

    semantic: Semantic,
    types: Types,
}

impl Parser<'_> {
    fn push(&mut self, kind: SemKind) -> Sem {
        self.semantic.push(kind, TypeSentinel::Unknown.to_index())
    }

    fn parse_root(&mut self) {
        let SynData::Root(sems) = &self.syntax[syntax::ROOT_SYN] else {
            panic!();
        };

        let root = self.semantic.push(
            SemKind::Module { bindings: vec![] },
            TypeSentinel::Unknown.to_index(),
        );
        assert_eq!(root, semantic::ROOT_SEM);

        let bindings = sems
            .iter()
            .map(|&sem| {
                let SynData::Binding { pattern, value } = &self.syntax[sem] else {
                    panic!()
                };

                let SynData::Ident(token) = &self.syntax[*pattern] else {
                    panic!()
                };

                let name = token::parse_identifer(self.source, self.tokens, *token);

                (name.to_string(), self.parse_expression(*value))
            })
            .collect();

        self.semantic.kinds[root] = SemKind::Module { bindings };
    }

    fn add_type(&mut self, sem: Sem, type_: Type) {
        self.semantic.types[sem] = combine_types(&mut self.types, self.semantic.types[sem], type_);
    }

    fn parse_expression(&mut self, i: Syn) -> Sem {
        match &self.syntax[i] {
            SynData::Ident(token) => self.push(SemKind::Reference {
                name: token::parse_identifer(self.source, self.tokens, *token).to_string(),
            }),
            SynData::False(token) => self.push(SemKind::False(*token)),
            SynData::True(token) => self.push(SemKind::True(*token)),
            SynData::Number(token) => self.push(SemKind::Number(*token)),
            SynData::Function { pattern, body } => {
                let param = self.push(SemKind::Reference {
                    name: "__param".to_string(),
                });

                let (pattern, return_type) = if let SynData::ReturnAscription {
                    syn: pattern,
                    type_: return_type,
                } = &self.syntax[*pattern]
                {
                    (pattern, self.parse_type(*return_type))
                } else {
                    (pattern, TypeSentinel::Unknown.to_index())
                };

                let (pattern, argument_type) = if let SynData::Ascription {
                    type_: argument_type,
                    ..
                } = &self.syntax[*pattern]
                {
                    (pattern, self.parse_type(*argument_type))
                } else {
                    (pattern, TypeSentinel::Unknown.to_index())
                };

                let body = self.parse_expression(*body);
                let (body, pattern_type) = self.sift_through_pattern(param, *pattern, body);

                let argument_type = combine_types(&mut self.types, argument_type, pattern_type);

                let ty = self.types.push(TypeData::Function {
                    argument_type,
                    return_type,
                });

                let sem = self.push(SemKind::Function {
                    argument: "__param".to_string(),
                    body,
                });

                self.add_type(sem, ty);

                sem
            }
            SynData::Add(lhs, rhs) => self.parse_binary_operator(*lhs, *rhs, "builtin_add"),
            SynData::Subtract(lhs, rhs) => self.parse_binary_operator(*lhs, *rhs, "builtin_sub"),
            SynData::Multiply(lhs, rhs) => self.parse_binary_operator(*lhs, *rhs, "builtin_mul"),
            SynData::Divide(lhs, rhs) => self.parse_binary_operator(*lhs, *rhs, "builtin_div"),
            SynData::Assignment { pattern, value } => {
                let value = self.parse_expression(*value);
                match self.syntax[*pattern] {
                    SynData::Ident(token) => self.push(SemKind::Assignment {
                        binding: parse_identifer(self.source, self.tokens, token).to_string(),
                        value,
                    }),
                    _ => panic!(),
                }
            }
            SynData::Application { function, argument } => {
                let function = self.parse_expression(*function);
                let argument = self.parse_expression(*argument);
                self.push(SemKind::Application { function, argument })
            }
            SynData::Loop(body) => {
                let body = self.parse_expression(*body);
                self.push(SemKind::Loop(body))
            }
            SynData::If { condition, then } => {
                let condition = self.parse_expression(*condition);
                let then = self.parse_expression(*then);

                self.push(SemKind::If { condition, then })
            }
            SynData::IfElse {
                condition,
                then,
                else_,
            } => {
                let condition = self.parse_expression(*condition);
                let then = self.parse_expression(*then);
                let else_ = self.parse_expression(*else_);

                self.push(SemKind::IfElse {
                    condition,
                    then,
                    else_,
                })
            }
            SynData::Paren(expr) => self.parse_expression(*expr),
            SynData::Tuple(sems) => {
                let fields = sems
                    .iter()
                    .enumerate()
                    .map(|(i, sem)| (i.to_string(), self.parse_expression(*sem)))
                    .collect();
                self.push(SemKind::BuildStruct { fields })
            }
            SynData::Ascription { syn, type_ } => {
                let expression = self.parse_expression(*syn);
                let ty = self.parse_type(*type_);
                self.add_type(expression, ty);
                expression
            }
            SynData::ChainOpen(syns) => self.parse_chain(syns.iter().copied(), false),
            SynData::ChainClosed(syns) => self.parse_chain(syns.iter().copied(), true),
            SynData::String(_segments) => todo!(
                "Implement string in the semantic phase, needs careful thought on interpolation"
            ),
            expr => panic!("{expr:?}"),
        }
    }

    fn parse_binary_operator(&mut self, lhs: Syn, rhs: Syn, function: &str) -> Sem {
        let lhs = self.parse_expression(lhs);
        let rhs = self.parse_expression(rhs);

        let structure = self.push(SemKind::BuildStruct {
            fields: vec![("0".to_string(), lhs), ("1".to_string(), rhs)],
        });

        let add_function = self.push(SemKind::Reference {
            name: function.to_string(),
        });

        self.push(SemKind::Application {
            function: add_function,
            argument: structure,
        })
    }

    fn parse_chain(&mut self, mut syns: impl Iterator<Item = Syn>, closed: bool) -> Sem {
        let mut expressions = Vec::new();

        while let Some(syn) = syns.next() {
            match &self.syntax[syn] {
                SynData::Binding { pattern, value } => {
                    let value = self.parse_expression(*value);
                    let body = self.parse_chain(syns, closed);
                    expressions.push(self.sift_through_pattern(value, *pattern, body).0);
                    break;
                }
                _ => {
                    expressions.push(self.parse_expression(syn));
                }
            }
        }

        if closed {
            self.push(SemKind::ChainClosed {
                statements: expressions,
            })
        } else {
            let Some((expression, statements)) = expressions.split_last() else {
                panic!();
            };

            self.push(SemKind::ChainOpen {
                statements: statements.to_vec(),
                expression: *expression,
            })
        }
    }

    fn sift_through_pattern(&mut self, value: Sem, pattern: Syn, body: Sem) -> (Sem, Type) {
        match &self.syntax[pattern] {
            SynData::Ident(token) => (
                self.push(SemKind::Binding {
                    name: token::parse_identifer(self.source, self.tokens, *token).to_string(),
                    value,
                    body,
                }),
                TypeSentinel::Unknown.to_index(),
            ),
            SynData::Mut { pattern } => {
                let SynData::Ident(token) = self.syntax[*pattern] else {
                    panic!()
                };

                let name = token::parse_identifer(self.source, self.tokens, token).to_string();

                (
                    self.push(SemKind::MutBinding { name, value, body }),
                    TypeSentinel::Unknown.to_index(),
                )
            }
            SynData::EmptyParen(_) => (body, TypeSentinel::Unit.to_index()),
            SynData::Paren(expr) => self.sift_through_pattern(value, *expr, body),
            SynData::Ascription {
                syn,
                type_: type_syn,
            } => {
                let type_ = self.parse_type(*type_syn);
                self.add_type(value, type_);
                (self.sift_through_pattern(value, *syn, body).0, type_)
            }
            // TODO: handle named fields
            SynData::Tuple(sems) => {
                let mut body = body;
                let mut fields_types = Vec::with_capacity(sems.len());

                for (i, sem) in sems.iter().enumerate().rev() {
                    let field = self.push(SemKind::Access {
                        field: i.to_string(),
                        expr: value,
                    });

                    let (field, field_type) = self.sift_through_pattern(field, *sem, body);

                    fields_types.push((i.to_string(), field_type));

                    body = field;
                }

                (
                    body,
                    self.types.push(TypeData::Product {
                        fields: fields_types,
                    }),
                )
            }
            pattern => panic!("{:#?}", pattern),
        }
    }

    fn parse_type(&mut self, i: Syn) -> Type {
        match &self.syntax[i] {
            SynData::Ident(token) => {
                match token::parse_identifer(self.source, self.tokens, *token) {
                    "u32" => TypeSentinel::Uint32.to_index(),
                    _ => panic!("unknown type"),
                }
            }
            SynData::ReturnAscription {
                syn: pattern,
                type_,
            } => {
                let argument_type = self.parse_type(*pattern);
                let return_type = self.parse_type(*type_);
                self.types.push(TypeData::Function {
                    argument_type: argument_type,
                    return_type: return_type,
                })
            }
            SynData::EmptyParen(_) => self.types.push(TypeData::Product { fields: Vec::new() }),
            SynData::Paren(expr) => self.parse_type(*expr),
            SynData::Tuple(sems) => {
                let fields = sems
                    .iter()
                    .enumerate()
                    .map(|(i, sem)| (i.to_string(), self.parse_type(*sem)))
                    .collect();
                self.types.push(TypeData::Product { fields })
            }
            _ => panic!(),
        }
    }
}
