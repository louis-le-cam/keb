use crate::{
    key_vec::{Sentinel, Val},
    semantic::{
        self, Node, NodeData, NodeKind, Nodes, Type, TypeData, TypeSentinel, Types, combine_types,
    },
    syntax::{self, Syn, SynData, Syns},
    token::{self, Tokens},
};

pub fn parse(source: &str, tokens: &Tokens, syntax: &Syns) -> (Nodes, Types) {
    let mut parser = Parser {
        source,
        tokens,
        syntax,
        nodes: Nodes::default(),
        types: Types::default(),
    };
    parser.parse_root();
    (parser.nodes, parser.types)
}

struct Parser<'a> {
    source: &'a str,
    tokens: &'a Tokens,
    syntax: &'a Syns,

    nodes: Nodes,
    types: Types,
}

impl Parser<'_> {
    fn push(&mut self, kind: NodeKind) -> Node {
        self.nodes.push(NodeData {
            kind,
            ty: TypeSentinel::Unknown.to_index(),
        })
    }

    fn parse_root(&mut self) {
        let Val::Value(SynData::Root(nodes)) = &self.syntax.get(syntax::ROOT_SYN) else {
            panic!();
        };

        let root = self.nodes.push(NodeData {
            kind: NodeKind::Module { bindings: vec![] },
            ty: TypeSentinel::Unknown.to_index(),
        });
        assert_eq!(root, semantic::ROOT_NODE);

        let bindings = nodes
            .iter()
            .map(|&node| {
                let Val::Value(SynData::Binding { pattern, value }) = self.syntax.get(node) else {
                    panic!()
                };

                let Val::Value(SynData::Ident(token)) = self.syntax.get(*pattern) else {
                    panic!()
                };

                let name = token::parse_identifer(self.source, self.tokens, *token);

                (name.to_string(), self.parse_expression(*value))
            })
            .collect();

        match self.nodes.get_mut(root) {
            Val::Value(node_data) => {
                *node_data = NodeData {
                    kind: NodeKind::Module { bindings },
                    ty: TypeSentinel::Unknown.to_index(),
                }
            }
            _ => panic!(),
        };
    }

    fn add_type(&mut self, node: Node, type_: Type) {
        match self.nodes.get_mut(node) {
            Val::None => panic!(),
            Val::Value(node_data) => {
                node_data.ty = combine_types(&mut self.types, node_data.ty, type_)
            }
        }
    }

    fn parse_expression(&mut self, i: Syn) -> Node {
        match self.syntax.get(i) {
            Val::None => panic!(),
            Val::Value(syn_data) => match syn_data {
                SynData::Ident(token) => self.push(NodeKind::Reference {
                    name: token::parse_identifer(self.source, self.tokens, *token).to_string(),
                }),
                SynData::False(token) => self.push(NodeKind::False(*token)),
                SynData::True(token) => self.push(NodeKind::True(*token)),
                SynData::Number(token) => self.push(NodeKind::Number(*token)),
                SynData::Function { pattern, body } => {
                    let param = self.push(NodeKind::Reference {
                        name: "__param".to_string(),
                    });

                    let (pattern, return_type) = if let Val::Value(SynData::ReturnAscription {
                        syn: pattern,
                        type_: return_type,
                    }) = &self.syntax.get(*pattern)
                    {
                        (pattern, self.parse_type(*return_type))
                    } else {
                        (pattern, TypeSentinel::Unknown.to_index())
                    };

                    let (pattern, argument_type) = if let Val::Value(SynData::Ascription {
                        type_: argument_type,
                        ..
                    }) = &self.syntax.get(*pattern)
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

                    let node = self.push(NodeKind::Function {
                        argument: "__param".to_string(),
                        body,
                    });

                    self.add_type(node, ty);

                    node
                }
                SynData::Add(lhs, rhs) => {
                    let lhs = self.parse_expression(*lhs);
                    let rhs = self.parse_expression(*rhs);

                    let structure = self.push(NodeKind::BuildStruct {
                        fields: vec![("0".to_string(), lhs), ("1".to_string(), rhs)],
                    });

                    let add_function = self.push(NodeKind::Reference {
                        name: "builtin_add".to_string(),
                    });

                    self.push(NodeKind::Application {
                        function: add_function,
                        argument: structure,
                    })
                }
                SynData::Application { function, argument } => {
                    let function = self.parse_expression(*function);
                    let argument = self.parse_expression(*argument);
                    self.push(NodeKind::Application { function, argument })
                }
                SynData::Paren(expr) => self.parse_expression(*expr),
                SynData::Tuple(nodes) => {
                    let fields = nodes
                        .iter()
                        .enumerate()
                        .map(|(i, node)| (i.to_string(), self.parse_expression(*node)))
                        .collect();
                    self.push(NodeKind::BuildStruct { fields })
                }
                SynData::Ascription { syn, type_ } => {
                    let expression = self.parse_expression(*syn);
                    let ty = self.parse_type(*type_);
                    self.add_type(expression, ty);
                    expression
                }
                SynData::ChainOpen(syns) => self.parse_chain(syns.iter().copied(), false),
                SynData::ChainClosed(syns) => self.parse_chain(syns.iter().copied(), true),
                expr => panic!("{expr:?}"),
            },
        }
    }

    fn parse_chain(&mut self, mut syns: impl Iterator<Item = Syn>, closed: bool) -> Node {
        let mut expressions = Vec::new();

        while let Some(syn) = syns.next() {
            match self.syntax.get(syn) {
                Val::None => panic!(),
                Val::Value(SynData::Binding { pattern, value }) => {
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
            let Some((expression, statements)) = expressions.split_last() else {
                panic!();
            };

            self.push(NodeKind::ChainOpen {
                statements: statements.to_vec(),
                expression: *expression,
            })
        } else {
            self.push(NodeKind::ChainClosed {
                statements: expressions,
            })
        }
    }

    fn sift_through_pattern(&mut self, value: Node, pattern: Syn, body: Node) -> (Node, Type) {
        match self.syntax.get(pattern) {
            Val::None => panic!(),
            Val::Value(syn_data) => match syn_data {
                SynData::Ident(token) => (
                    self.push(NodeKind::Binding {
                        name: token::parse_identifer(self.source, self.tokens, *token).to_string(),
                        value,
                        body,
                    }),
                    TypeSentinel::Unknown.to_index(),
                ),
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
                SynData::Tuple(nodes) => {
                    let mut body = body;
                    let mut fields_types = Vec::with_capacity(nodes.len());

                    for (i, node) in nodes.iter().enumerate().rev() {
                        let field = self.push(NodeKind::Access {
                            field: i.to_string(),
                            expr: value,
                        });

                        let (field, field_type) = self.sift_through_pattern(field, *node, body);

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
            },
        }
    }

    fn parse_type(&mut self, i: Syn) -> Type {
        match self.syntax.get(i) {
            Val::None => panic!(),
            Val::Value(syn_data) => match syn_data {
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
                SynData::Tuple(nodes) => {
                    let fields = nodes
                        .iter()
                        .enumerate()
                        .map(|(i, node)| (i.to_string(), self.parse_type(*node)))
                        .collect();
                    self.types.push(TypeData::Product { fields })
                }
                _ => panic!(),
            },
        }
    }
}
