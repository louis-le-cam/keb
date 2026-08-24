use crate::{
    key_vec::{
        Sentinels,
        Value::{Item, Sentinel},
    },
    semantic::{
        ROOT_SEM, Sem, SemKind, SemKinds, SemTypes, Semantic, Type, TypeData, TypeSentinel, Types,
        combine_types,
    },
    syntax::{ROOT_SYN, Syn, SynKind, SynSentinel, Syntax},
    token::{self, Token, TokenOffsets},
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
        let root = self.semantic.push(
            SemKind::Module { bindings: vec![] },
            TypeSentinel::Unknown.to_index(),
        );
        assert_eq!(root, ROOT_SEM);

        let mut bindings = Vec::new();

        let mut root_syn = ROOT_SYN;
        loop {
            assert_eq!(self.syntax.kinds[ROOT_SYN], SynKind::Root);

            let Item(syn) = self.syntax.lhs.get(root_syn) else {
                break;
            };
            let syn = Syn::from(*syn);

            assert_eq!(self.syntax.kinds[syn], SynKind::Binding);
            let pattern = Syn::from(self.syntax.lhs[syn]);
            let value = Syn::from(self.syntax.rhs[syn]);

            assert_eq!(self.syntax.kinds[pattern], SynKind::Ident);
            let token = Token::from(self.syntax.lhs[pattern]);
            let name = token::parse_identifer(self.source, self.tokens, token);

            bindings.push((name.to_string(), self.parse_expression(value)));

            match self.syntax.rhs.get(root_syn) {
                Sentinel(SynSentinel::None) => break,
                Item(rhs) => root_syn = Syn::from(*rhs),
            };
        }

        self.semantic.kinds[root] = SemKind::Module { bindings };
    }

    fn add_type(&mut self, sem: Sem, type_: Type) {
        self.semantic.types[sem] = combine_types(&mut self.types, self.semantic.types[sem], type_);
    }

    fn parse_expression(&mut self, i: Syn) -> Sem {
        match &self.syntax.kinds[i] {
            SynKind::Ident => {
                let token = Token::from(self.syntax.lhs[i]);
                let name = token::parse_identifer(self.source, self.tokens, token).to_string();
                self.push(SemKind::Reference { name })
            }
            SynKind::False => {
                let token = Token::from(self.syntax.lhs[i]);
                self.push(SemKind::False(token))
            }
            SynKind::True => {
                let token = Token::from(self.syntax.lhs[i]);
                self.push(SemKind::True(token))
            }
            SynKind::Number => {
                let token = Token::from(self.syntax.lhs[i]);
                self.push(SemKind::Number(token))
            }
            SynKind::Function => {
                let pattern = Syn::from(self.syntax.lhs[i]);
                let body = Syn::from(self.syntax.rhs[i]);

                let param = self.push(SemKind::Reference {
                    name: "__param".to_string(),
                });

                let (pattern, return_type) =
                    if self.syntax.kinds[pattern] == SynKind::ReturnAscription {
                        let ascription_pattern = Syn::from(self.syntax.lhs[pattern]);
                        let return_type = Syn::from(self.syntax.rhs[pattern]);
                        (ascription_pattern, self.parse_type(return_type))
                    } else {
                        (pattern, TypeSentinel::Unknown.to_index())
                    };

                let (pattern, argument_type) = if self.syntax.kinds[pattern] == SynKind::Ascription
                {
                    let ascription_pattern = Syn::from(self.syntax.lhs[pattern]);
                    let argument_type = Syn::from(self.syntax.rhs[pattern]);
                    (ascription_pattern, self.parse_type(argument_type))
                } else {
                    (pattern, TypeSentinel::Unknown.to_index())
                };

                let body = self.parse_expression(body);
                let (body, pattern_type) = self.sift_through_pattern(param, pattern, body);

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
            SynKind::Equal => {
                let lhs = Syn::from(self.syntax.lhs[i]);
                let rhs = Syn::from(self.syntax.rhs[i]);
                self.parse_binary_operator(lhs, rhs, "builtin_equal")
            }
            SynKind::Add => {
                let lhs = Syn::from(self.syntax.lhs[i]);
                let rhs = Syn::from(self.syntax.rhs[i]);
                self.parse_binary_operator(lhs, rhs, "builtin_add")
            }
            SynKind::Subtract => {
                let lhs = Syn::from(self.syntax.lhs[i]);
                let rhs = Syn::from(self.syntax.rhs[i]);
                self.parse_binary_operator(lhs, rhs, "builtin_sub")
            }
            SynKind::Multiply => {
                let lhs = Syn::from(self.syntax.lhs[i]);
                let rhs = Syn::from(self.syntax.rhs[i]);
                self.parse_binary_operator(lhs, rhs, "builtin_mul")
            }
            SynKind::Divide => {
                let lhs = Syn::from(self.syntax.lhs[i]);
                let rhs = Syn::from(self.syntax.rhs[i]);
                self.parse_binary_operator(lhs, rhs, "builtin_div")
            }
            SynKind::Assignment => {
                let pattern = Syn::from(self.syntax.lhs[i]);
                let value = self.parse_expression(Syn::from(self.syntax.rhs[i]));

                match self.syntax.kinds[pattern] {
                    SynKind::Ident => {
                        let token = Token::from(self.syntax.lhs[i]);
                        let binding =
                            token::parse_identifer(self.source, self.tokens, token).to_string();
                        self.push(SemKind::Assignment { binding, value })
                    }
                    _ => panic!(),
                }
            }
            SynKind::Application => {
                let function = self.parse_expression(Syn::from(self.syntax.lhs[i]));
                let argument = self.parse_expression(Syn::from(self.syntax.rhs[i]));
                self.push(SemKind::Application { function, argument })
            }
            SynKind::Loop => {
                let body = self.parse_expression(Syn::from(self.syntax.lhs[i]));
                self.push(SemKind::Loop(body))
            }
            SynKind::Match => {
                let function = self.push(SemKind::Function {
                    argument: "__param".to_string(),
                    body: Sem::from_u32_index(0),
                });

                let curly = Syn::from(self.syntax.lhs[i]);
                let reversed_arms = self.parse_match_body(curly);

                let param = self.push(SemKind::Reference {
                    name: "__param".to_string(),
                });

                let empty_chain = self.push(SemKind::ChainClosed {
                    statements: Vec::new(),
                });
                let mut rest = self.push(SemKind::Loop(empty_chain));

                for (pattern, body) in reversed_arms {
                    let body = self.parse_expression(body);
                    rest = self.sift_through_optional_pattern(param, pattern, body, rest);
                }

                if let SemKind::Function { body, .. } = &mut self.semantic.kinds[function] {
                    *body = rest;
                } else {
                    panic!()
                }

                let function_type = self.types.push(TypeData::Function {
                    argument_type: self.semantic.types[param],
                    return_type: TypeSentinel::Unknown.to_index(),
                });
                self.add_type(function, function_type);

                function
            }
            SynKind::If => {
                let condition = self.parse_expression(Syn::from(self.syntax.lhs[i]));
                let body = Syn::from(self.syntax.rhs[i]);

                if self.syntax.kinds[body] == SynKind::Else {
                    let then = self.parse_expression(Syn::from(self.syntax.lhs[body]));
                    let else_ = self.parse_expression(Syn::from(self.syntax.rhs[body]));

                    self.push(SemKind::IfElse {
                        condition,
                        then,
                        else_,
                    })
                } else {
                    let then = self.parse_expression(body);
                    self.push(SemKind::If { condition, then })
                }
            }
            SynKind::Else => panic!(),
            SynKind::Paren => {
                let syn = Syn::from(self.syntax.rhs[i]);
                match syn.sentinel() {
                    Some(SynSentinel::None) => self.semantic.push(
                        SemKind::BuildStruct { fields: Vec::new() },
                        TypeSentinel::Unit.to_index(),
                    ),
                    None => self.parse_expression(syn),
                }
            }
            SynKind::Tuple => {
                let mut fields = Vec::new();

                let mut tuple = i;
                for i in 0.. {
                    assert_eq!(self.syntax.kinds[tuple], SynKind::Tuple);
                    let lhs = Syn::from(self.syntax.lhs[tuple]);

                    fields.push(self.parse_tuple_field(lhs, i));

                    let rhs = Syn::from(self.syntax.rhs[tuple]);
                    match self.syntax.kinds.get(rhs) {
                        Sentinel(SynSentinel::None) => break,
                        Item(SynKind::Tuple) => {
                            tuple = rhs;
                            continue;
                        }
                        Item(_) => {
                            fields.push(self.parse_tuple_field(rhs, i));
                            break;
                        }
                    }
                }

                self.push(SemKind::BuildStruct { fields })
            }
            SynKind::Ascription => {
                let expression = self.parse_expression(Syn::from(self.syntax.lhs[i]));
                let ty = self.parse_type(Syn::from(self.syntax.rhs[i]));
                self.add_type(expression, ty);
                expression
            }
            SynKind::Chain => self.parse_chain(i),
            SynKind::String => todo!(
                "Implement string in the semantic phase, needs careful thought on interpolation"
            ),
            SynKind::StringSegment | SynKind::StringInterpolation => unreachable!(),
            kind => panic!("{kind:?}"),
        }
    }

    fn parse_match_body(&mut self, syn: Syn) -> Box<dyn Iterator<Item = (Syn, Syn)>> {
        match self.syntax.kinds[syn] {
            SynKind::Curly => {
                let content = Syn::from(self.syntax.rhs[syn]);

                match &self.syntax.kinds[content] {
                    SynKind::Function => {
                        let pattern = Syn::from(self.syntax.lhs[content]);
                        let body = Syn::from(self.syntax.rhs[content]);
                        Box::new(std::iter::once((pattern, body)))
                    }
                    SynKind::Tuple => {
                        let mut arms = Vec::new();
                        let mut tuple = content;
                        loop {
                            assert_eq!(self.syntax.kinds[tuple], SynKind::Tuple);
                            let lhs = Syn::from(self.syntax.lhs[tuple]);

                            arms.push(self.parse_match_arm(lhs));

                            let rhs = Syn::from(self.syntax.rhs[tuple]);
                            match self.syntax.kinds.get(rhs) {
                                Sentinel(SynSentinel::None) => break,
                                Item(SynKind::Tuple) => {
                                    tuple = rhs;
                                    continue;
                                }
                                Item(SynKind::Function) => arms.push(self.parse_match_arm(rhs)),
                                Item(_) => panic!(),
                            }
                        }

                        Box::new(arms.into_iter().rev())
                    }
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }

    fn parse_match_arm(&mut self, syn: Syn) -> (Syn, Syn) {
        assert_eq!(self.syntax.kinds[syn], SynKind::Function);
        let pattern = Syn::from(self.syntax.lhs[syn]);
        let body = Syn::from(self.syntax.rhs[syn]);
        (pattern, body)
    }

    fn parse_tuple_field(&mut self, syn: Syn, i: usize) -> (String, Sem) {
        if self.syntax.kinds[syn] != SynKind::Ascription {
            return (i.to_string(), self.parse_expression(syn));
        }

        let ident = Syn::from(self.syntax.lhs[syn]);
        assert_eq!(self.syntax.kinds[ident], SynKind::Ident);
        let token = Token::from(self.syntax.lhs[ident]);
        let name = token::parse_identifer(self.source, self.tokens, token).to_string();

        let field = self.parse_expression(Syn::from(self.syntax.rhs[syn]));
        (name.to_string(), field)
    }

    fn parse_chain(&mut self, syn: Syn) -> Sem {
        let mut statements = Vec::new();

        let mut chain = syn;
        loop {
            assert_eq!(self.syntax.kinds[chain], SynKind::Chain);
            let statement = Syn::from(self.syntax.lhs[chain]);

            if self.syntax.kinds[statement] == SynKind::Binding {
                let pattern = Syn::from(self.syntax.lhs[statement]);
                let value = self.parse_expression(Syn::from(self.syntax.rhs[statement]));

                let body_syn = Syn::from(self.syntax.rhs[chain]);
                let body = match self.syntax.kinds.get(body_syn) {
                    Sentinel(SynSentinel::None) => self.push(SemKind::ChainClosed {
                        statements: Vec::new(),
                    }),
                    Item(SynKind::Chain) => self.parse_chain(body_syn),
                    Item(_) => self.parse_expression(body_syn),
                };
                let expression = self.sift_through_pattern(value, pattern, body).0;
                break self.push(SemKind::ChainOpen {
                    statements,
                    expression,
                });
            }

            statements.push(self.parse_expression(statement));

            let rhs = Syn::from(self.syntax.rhs[chain]);
            match self.syntax.kinds.get(rhs) {
                Sentinel(SynSentinel::None) => {
                    break self.push(SemKind::ChainClosed { statements });
                }
                Item(SynKind::Chain) => chain = rhs,
                Item(_) => {
                    let expression = self.parse_expression(rhs);
                    break self.push(SemKind::ChainOpen {
                        statements,
                        expression,
                    });
                }
            }
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

    // fn parse_chain(&mut self, mut syns: impl Iterator<Item = Syn>, closed: bool) -> Sem {
    //     let mut expressions = Vec::new();

    //     while let Some(syn) = syns.next() {
    //         match &self.syntax[syn] {
    //             SynData::Binding { pattern, value } => {
    //                 let value = self.parse_expression(*value);
    //                 let body = self.parse_chain(syns, closed);
    //                 expressions.push(self.sift_through_pattern(value, *pattern, body).0);
    //                 break;
    //             }
    //             _ => {
    //                 expressions.push(self.parse_expression(syn));
    //             }
    //         }
    //     }

    //     if closed {
    //         self.push(SemKind::ChainClosed {
    //             statements: expressions,
    //         })
    //     } else {
    //         let Some((expression, statements)) = expressions.split_last() else {
    //             panic!();
    //         };

    //         self.push(SemKind::ChainOpen {
    //             statements: statements.to_vec(),
    //             expression: *expression,
    //         })
    //     }
    // }

    fn sift_through_pattern(&mut self, value: Sem, pattern: Syn, body: Sem) -> (Sem, Type) {
        match &self.syntax.kinds[pattern] {
            SynKind::Ident => {
                let token = Token::from(self.syntax.lhs[pattern]);
                let binding = self.push(SemKind::Binding {
                    name: token::parse_identifer(self.source, self.tokens, token).to_string(),
                    value,
                    body,
                });

                (binding, TypeSentinel::Unknown.to_index())
            }
            SynKind::Mut => {
                let pattern = Syn::from(self.syntax.lhs[pattern]);
                let SynKind::Ident = self.syntax.kinds[pattern] else {
                    panic!()
                };

                let token = Token::from(self.syntax.lhs[pattern]);
                let name = token::parse_identifer(self.source, self.tokens, token).to_string();

                (
                    self.push(SemKind::MutBinding { name, value, body }),
                    TypeSentinel::Unknown.to_index(),
                )
            }
            SynKind::Paren => {
                let syn = Syn::from(self.syntax.rhs[pattern]);
                match syn.sentinel() {
                    Some(SynSentinel::None) => (body, TypeSentinel::Unit.to_index()),
                    None => self.sift_through_pattern(value, syn, body),
                }
            }
            SynKind::Ascription => {
                let syn = Syn::from(self.syntax.lhs[pattern]);
                let type_ = self.parse_type(Syn::from(self.syntax.rhs[pattern]));
                self.add_type(value, type_);
                (self.sift_through_pattern(value, syn, body).0, type_)
            }
            SynKind::Tuple => {
                let mut syns = Vec::new();
                let mut tuple = pattern;

                loop {
                    assert_eq!(self.syntax.kinds[tuple], SynKind::Tuple);

                    syns.push(Syn::from(self.syntax.lhs[tuple]));

                    let rhs = Syn::from(self.syntax.rhs[tuple]);
                    match self.syntax.kinds.get(rhs) {
                        Sentinel(SynSentinel::None) => break,
                        Item(SynKind::Tuple) => {
                            tuple = rhs;
                            continue;
                        }
                        Item(_) => {
                            syns.push(rhs);
                            break;
                        }
                    }
                }

                let mut body = body;
                let mut fields_types = Vec::with_capacity(syns.len());

                for (i, sem) in syns.iter().enumerate().rev() {
                    let field = self.push(SemKind::Access {
                        field: i.to_string(),
                        expr: value,
                    });

                    let (field, field_type) = self.sift_through_pattern(field, *sem, body);

                    fields_types.push((i.to_string(), field_type));

                    body = field;
                }

                let type_ = self.types.push(TypeData::Product {
                    fields: fields_types,
                });

                (body, type_)
            }
            pattern => panic!("{:#?}", pattern),
        }
    }

    fn sift_through_optional_pattern(
        &mut self,
        value: Sem,
        pattern: Syn,
        then: Sem,
        else_: Sem,
    ) -> Sem {
        match &self.syntax.kinds[pattern] {
            SynKind::Ident => {
                let token = Token::from(self.syntax.lhs[pattern]);
                self.push(SemKind::Binding {
                    name: token::parse_identifer(self.source, self.tokens, token).to_string(),
                    value,
                    body: then,
                })
            }
            SynKind::False => {
                let token = Token::from(self.syntax.lhs[pattern]);

                self.add_type(value, TypeSentinel::False.to_index());
                let false_ = self.push(SemKind::False(token));
                let structure = self.push(SemKind::BuildStruct {
                    fields: vec![("0".to_string(), value), ("1".to_string(), false_)],
                });

                let add_function = self.push(SemKind::Reference {
                    name: "builtin_equal".to_string(),
                });

                let condition = self.push(SemKind::Application {
                    function: add_function,
                    argument: structure,
                });

                self.push(SemKind::IfElse {
                    condition,
                    then,
                    else_,
                })
            }
            SynKind::True => {
                self.add_type(value, TypeSentinel::True.to_index());

                self.push(SemKind::IfElse {
                    condition: value,
                    then,
                    else_,
                })
            }
            SynKind::Number => {
                let token = Token::from(self.syntax.lhs[pattern]);

                self.add_type(value, TypeSentinel::Uint32.to_index());

                let number = self.push(SemKind::Number(token));

                let structure = self.push(SemKind::BuildStruct {
                    fields: vec![("0".to_string(), value), ("1".to_string(), number)],
                });

                let add_function = self.push(SemKind::Reference {
                    name: "builtin_equal".to_string(),
                });

                let condition = self.push(SemKind::Application {
                    function: add_function,
                    argument: structure,
                });

                self.push(SemKind::IfElse {
                    condition,
                    then,
                    else_,
                })
            }

            SynKind::Mut => todo!(),
            SynKind::Ascription => todo!(),
            SynKind::Paren => todo!(),

            SynKind::Curly => todo!(),

            SynKind::Tuple => todo!(),

            SynKind::Application => todo!(),

            SynKind::String => unreachable!(),
            SynKind::StringSegment | SynKind::StringInterpolation => unreachable!(),

            _ => panic!(),
        }
    }

    fn parse_type(&mut self, i: Syn) -> Type {
        match &self.syntax.kinds[i] {
            SynKind::Ident => {
                let token = Token::from(self.syntax.lhs[i]);
                match token::parse_identifer(self.source, self.tokens, token) {
                    "u32" => TypeSentinel::Uint32.to_index(),
                    _ => panic!("unknown type"),
                }
            }
            SynKind::ReturnAscription => {
                let argument_type = self.parse_type(Syn::from(self.syntax.lhs[i]));
                let return_type = self.parse_type(Syn::from(self.syntax.rhs[i]));
                self.types.push(TypeData::Function {
                    argument_type,
                    return_type,
                })
            }
            SynKind::Paren => {
                let inner = Syn::from(self.syntax.rhs[i]);
                match self.syntax.kinds.get(inner) {
                    Sentinel(SynSentinel::None) => TypeSentinel::Unit.to_index(),
                    Item(_) => self.parse_type(inner),
                }
            }
            SynKind::Tuple => {
                let mut syns = Vec::new();
                let mut tuple = i;

                loop {
                    assert_eq!(self.syntax.kinds[tuple], SynKind::Tuple);

                    syns.push(Syn::from(self.syntax.lhs[tuple]));

                    let rhs = Syn::from(self.syntax.rhs[tuple]);
                    match self.syntax.kinds.get(rhs) {
                        Sentinel(SynSentinel::None) => break,
                        Item(SynKind::Tuple) => {
                            tuple = rhs;
                            continue;
                        }
                        Item(_) => {
                            syns.push(rhs);
                            break;
                        }
                    }
                }

                let fields = syns
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
