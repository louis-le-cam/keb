use std::collections::HashMap;

use crate::{
    key_vec::{Sentinel, Val},
    semantic::{self, Node, NodeKind, Nodes, Type, TypeData, TypeSentinel, Types, combine_types},
};

pub fn infer_types(nodes: &mut Nodes, types: &mut Types) {
    let mut inferrer = Inferrer { nodes, types };

    inferrer.infer_root();
}

type Scope = HashMap<String, ScopeItem>;

#[derive(Clone, Debug)]
enum ScopeItem {
    Node(Node),
    Argument(Node),
    Type(Type),
}

struct Inferrer<'a> {
    nodes: &'a mut Nodes,
    types: &'a mut Types,
}

impl Inferrer<'_> {
    fn infer_root(&mut self) {
        let uint32_tuple = self.types.push(TypeData::Product {
            fields: vec![
                ("0".to_string(), TypeSentinel::Uint32.to_index()),
                ("1".to_string(), TypeSentinel::Uint32.to_index()),
            ],
        });
        let builtin_add = self.types.push(TypeData::Function {
            argument_type: uint32_tuple,
            return_type: TypeSentinel::Uint32.to_index(),
        });

        let unit = self.types.push(TypeData::Product { fields: Vec::new() });
        let print = self.types.push(TypeData::Function {
            argument_type: TypeSentinel::Uint32.to_index(),
            return_type: unit,
        });

        let scope = Scope::from([
            ("builtin_add".to_string(), ScopeItem::Type(builtin_add)),
            ("print".to_string(), ScopeItem::Type(print)),
        ]);
        self.infer_expression(&scope, semantic::ROOT_NODE);
    }

    fn add_type(&mut self, node: Node, type_: Type) {
        match self.nodes.get_mut(node) {
            Val::None => panic!(),
            Val::Value(node_data) => node_data.ty = combine_types(self.types, node_data.ty, type_),
        }
    }

    fn infer_expression(&mut self, scope: &Scope, i: Node) {
        match self.nodes.get_mut(i) {
            Val::Value(node_data) => match &node_data.kind {
                NodeKind::Number { .. } => self.add_type(i, TypeSentinel::Uint32.to_index()),
                NodeKind::False { .. } => self.add_type(i, TypeSentinel::False.to_index()),
                NodeKind::True { .. } => self.add_type(i, TypeSentinel::True.to_index()),
                NodeKind::Module { bindings } => {
                    let bindings = bindings.clone();

                    let mut scope = scope.clone();
                    for (name, node) in &bindings {
                        scope.insert(name.clone(), ScopeItem::Node(*node));
                    }

                    for (_, node) in &bindings {
                        self.infer_expression(&scope, *node);
                    }

                    let fields = bindings
                        .iter()
                        .map(|(name, value)| {
                            (name.clone(), {
                                match self.nodes.get(*value) {
                                    Val::None => panic!(),
                                    Val::Value(node_data) => node_data.ty,
                                }
                            })
                        })
                        .collect::<Vec<(String, Type)>>();

                    let type_ = self.types.push(TypeData::Product { fields });
                    self.add_type(i, type_);
                }
                NodeKind::Function { argument, body } => {
                    let body = *body;

                    let mut scope = scope.clone();
                    scope.insert(argument.clone(), ScopeItem::Argument(i));

                    {
                        let (argument_type, return_type) = match self.nodes.get(i) {
                            Val::None => panic!(),
                            Val::Value(node_data) => match self.types.get(node_data.ty) {
                                Val::None => panic!(),
                                Val::Sentinel(_) => panic!(),
                                Val::Value(type_data) => match type_data {
                                    TypeData::Function {
                                        argument_type,
                                        return_type,
                                    } => (*argument_type, *return_type),
                                    _ => panic!(),
                                },
                            },
                        };

                        let body_type = match self.nodes.get(body) {
                            Val::None => panic!(),
                            Val::Value(node_data) => node_data.ty,
                        };

                        let return_type = combine_types(&mut self.types, body_type, return_type);

                        let type_ = self.types.push(TypeData::Function {
                            argument_type,
                            return_type,
                        });
                        self.add_type(i, type_);
                    }

                    self.infer_expression(&scope, body);

                    {
                        let node_type = match self.nodes.get(i) {
                            Val::None => panic!(),
                            Val::Value(node_data) => node_data.ty,
                        };

                        let (argument_type, return_type) = match &self.types.get(node_type) {
                            Val::Value(TypeData::Function {
                                argument_type,
                                return_type,
                            }) => (*argument_type, *return_type),
                            _ => (
                                TypeSentinel::Unknown.to_index(),
                                TypeSentinel::Unknown.to_index(),
                            ),
                        };

                        let body_type = match self.nodes.get(body) {
                            Val::None => panic!(),
                            Val::Value(node_data) => node_data.ty,
                        };

                        let return_type = combine_types(self.types, body_type, return_type);

                        let type_ = self.types.push(TypeData::Function {
                            argument_type,
                            return_type,
                        });
                        self.add_type(i, type_);
                    }
                }
                NodeKind::Binding { name, value, body } => {
                    let name = name.clone();
                    let value = *value;
                    let body = *body;

                    self.infer_expression(scope, value);

                    let mut scope = scope.clone();
                    scope.insert(name, ScopeItem::Node(value));

                    self.infer_expression(&scope, body);

                    let body_type = match self.nodes.get(body) {
                        Val::None => panic!(),
                        Val::Value(node_data) => node_data.ty,
                    };

                    self.add_type(i, body_type);
                }
                NodeKind::Reference { name } => {
                    let type_ = match scope[name] {
                        ScopeItem::Node(node) => match self.nodes.get(node) {
                            Val::Value(node_data) => node_data.ty,
                            Val::None => panic!(),
                        },
                        ScopeItem::Argument(node) => match self.nodes.get(node) {
                            Val::Value(node_data) => match self.types.get(node_data.ty) {
                                Val::Value(TypeData::Function { argument_type, .. }) => {
                                    *argument_type
                                }
                                Val::None | Val::Sentinel(_) | Val::Value(_) => panic!(),
                            },
                            Val::None => panic!(),
                        },
                        ScopeItem::Type(ty) => ty,
                    };

                    self.add_type(i, type_);
                }
                NodeKind::Access { field, expr } => {
                    let field = field.clone();
                    let expr = *expr;

                    self.infer_expression(scope, expr);

                    let expr_data = match self.nodes.get_mut(expr) {
                        Val::Value(node_data) => node_data,
                        Val::None => panic!(),
                    };

                    match self.types.get(expr_data.ty) {
                        Val::None => panic!(),
                        Val::Sentinel(sentinel) => match sentinel {
                            TypeSentinel::Unknown => {}
                            _ => {}
                        },
                        Val::Value(type_data) => match type_data {
                            TypeData::Product { fields } => {
                                if let Some((_, field_type)) =
                                    fields.iter().find(|(name, _)| name == &field)
                                {
                                    self.add_type(i, *field_type);
                                }
                            }
                            _ => {}
                        },
                    }
                }
                NodeKind::Application { function, argument } => {
                    let function = *function;
                    let argument = *argument;

                    self.infer_expression(scope, function);
                    self.infer_expression(scope, argument);

                    let function_node_data = match self.nodes.get_mut(function) {
                        Val::None => panic!(),
                        Val::Value(node_data) => node_data,
                    };

                    match self.types.get(function_node_data.ty) {
                        Val::Value(TypeData::Function { return_type, .. }) => {
                            self.add_type(i, *return_type);
                        }
                        Val::None | Val::Sentinel(_) | Val::Value(_) => panic!(),
                    }
                }
                NodeKind::Loop(body) => {
                    let body = *body;
                    self.infer_expression(scope, body);
                    // TODO: When we add `break`, the type inference should infer based on them
                    self.add_type(i, TypeSentinel::Unit.to_index());
                }
                NodeKind::BuildStruct { fields } => {
                    let fields = fields.clone();
                    for (_, value) in &fields {
                        self.infer_expression(scope, *value);
                    }

                    let type_ = self.types.push(TypeData::Product {
                        fields: fields
                            .iter()
                            .map(|(name, value)| {
                                (
                                    name.clone(),
                                    match self.nodes.get(*value) {
                                        Val::None => panic!(),
                                        Val::Value(node_data) => node_data.ty,
                                    },
                                )
                            })
                            .collect::<Vec<(String, Type)>>(),
                    });

                    self.add_type(i, type_);
                }
                NodeKind::ChainOpen {
                    statements,
                    expression,
                } => {
                    let expression = *expression;

                    for statement in statements.clone() {
                        self.infer_expression(scope, statement);
                    }

                    self.infer_expression(scope, expression);

                    if let Val::Value(node_data) = self.nodes.get(expression) {
                        self.add_type(i, node_data.ty);
                    }
                }
                NodeKind::ChainClosed { statements } => {
                    for statement in statements.clone() {
                        self.infer_expression(scope, statement);
                    }

                    self.add_type(i, TypeSentinel::Unit.to_index());
                }
            },
            Val::None => panic!(),
        }
    }
}
