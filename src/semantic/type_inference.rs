use std::collections::HashMap;

use crate::{
    key_vec::{Sentinel, Val},
    semantic::{self, Sem, SemKind, Semantic, Type, TypeData, TypeSentinel, Types, combine_types},
};

pub fn infer_types(semantic: &mut Semantic, types: &mut Types) {
    let mut inferrer = Inferrer { semantic, types };

    inferrer.infer_root();
}

type Scope = HashMap<String, ScopeItem>;

#[derive(Clone, Debug)]
enum ScopeItem {
    Sem(Sem),
    Argument(Sem),
    Type(Type),
}

struct Inferrer<'a> {
    semantic: &'a mut Semantic,
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
        self.infer_expression(&scope, semantic::ROOT_SEM);
    }

    fn add_type(&mut self, sem: Sem, type_: Type) {
        match self.semantic.get_mut(sem) {
            Val::None => panic!(),
            Val::Value(sem_data) => sem_data.ty = combine_types(self.types, sem_data.ty, type_),
        }
    }

    fn infer_expression(&mut self, scope: &Scope, i: Sem) {
        match self.semantic.get_mut(i) {
            Val::Value(sem_data) => match &sem_data.kind {
                SemKind::Number { .. } => self.add_type(i, TypeSentinel::Uint32.to_index()),
                SemKind::False { .. } => self.add_type(i, TypeSentinel::False.to_index()),
                SemKind::True { .. } => self.add_type(i, TypeSentinel::True.to_index()),
                SemKind::Module { bindings } => {
                    let bindings = bindings.clone();

                    let mut scope = scope.clone();
                    for (name, sem) in &bindings {
                        scope.insert(name.clone(), ScopeItem::Sem(*sem));
                    }

                    for (_, sem) in &bindings {
                        self.infer_expression(&scope, *sem);
                    }

                    let fields = bindings
                        .iter()
                        .map(|(name, value)| {
                            (name.clone(), {
                                match self.semantic.get(*value) {
                                    Val::None => panic!(),
                                    Val::Value(sem_data) => sem_data.ty,
                                }
                            })
                        })
                        .collect::<Vec<(String, Type)>>();

                    let type_ = self.types.push(TypeData::Product { fields });
                    self.add_type(i, type_);
                }
                SemKind::Function { argument, body } => {
                    let body = *body;

                    let mut scope = scope.clone();
                    scope.insert(argument.clone(), ScopeItem::Argument(i));

                    {
                        let (argument_type, return_type) = match self.semantic.get(i) {
                            Val::None => panic!(),
                            Val::Value(sem_data) => match self.types.get(sem_data.ty) {
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

                        let body_type = match self.semantic.get(body) {
                            Val::None => panic!(),
                            Val::Value(sem_data) => sem_data.ty,
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
                        let sem_type = match self.semantic.get(i) {
                            Val::None => panic!(),
                            Val::Value(sem_data) => sem_data.ty,
                        };

                        let (argument_type, return_type) = match &self.types.get(sem_type) {
                            Val::Value(TypeData::Function {
                                argument_type,
                                return_type,
                            }) => (*argument_type, *return_type),
                            _ => (
                                TypeSentinel::Unknown.to_index(),
                                TypeSentinel::Unknown.to_index(),
                            ),
                        };

                        let body_type = match self.semantic.get(body) {
                            Val::None => panic!(),
                            Val::Value(sem_data) => sem_data.ty,
                        };

                        let return_type = combine_types(self.types, body_type, return_type);

                        let type_ = self.types.push(TypeData::Function {
                            argument_type,
                            return_type,
                        });
                        self.add_type(i, type_);
                    }
                }
                SemKind::Binding { name, value, body } => {
                    let name = name.clone();
                    let value = *value;
                    let body = *body;

                    self.infer_expression(scope, value);

                    let mut scope = scope.clone();
                    scope.insert(name, ScopeItem::Sem(value));

                    self.infer_expression(&scope, body);

                    let body_type = match self.semantic.get(body) {
                        Val::None => panic!(),
                        Val::Value(sem_data) => sem_data.ty,
                    };

                    self.add_type(i, body_type);
                }
                SemKind::Reference { name } => {
                    let type_ = match scope[name] {
                        ScopeItem::Sem(sem) => match self.semantic.get(sem) {
                            Val::Value(sem_data) => sem_data.ty,
                            Val::None => panic!(),
                        },
                        ScopeItem::Argument(sem) => match self.semantic.get(sem) {
                            Val::Value(sem_data) => match self.types.get(sem_data.ty) {
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
                SemKind::Access { field, expr } => {
                    let field = field.clone();
                    let expr = *expr;

                    self.infer_expression(scope, expr);

                    let expr_data = match self.semantic.get_mut(expr) {
                        Val::Value(sem_data) => sem_data,
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
                SemKind::Application { function, argument } => {
                    let function = *function;
                    let argument = *argument;

                    self.infer_expression(scope, function);
                    self.infer_expression(scope, argument);

                    let function_sem_data = match self.semantic.get_mut(function) {
                        Val::None => panic!(),
                        Val::Value(sem_data) => sem_data,
                    };

                    match self.types.get(function_sem_data.ty) {
                        Val::Value(TypeData::Function { return_type, .. }) => {
                            self.add_type(i, *return_type);
                        }
                        Val::None | Val::Sentinel(_) | Val::Value(_) => panic!(),
                    }
                }
                SemKind::Loop(body) => {
                    let body = *body;
                    self.infer_expression(scope, body);
                    // TODO: When we add `break`, the type inference should infer based on them
                    self.add_type(i, TypeSentinel::Unit.to_index());
                }
                SemKind::If { condition, then } => {
                    let condition = *condition;
                    let then = *then;

                    self.add_type(condition, TypeSentinel::Bool.to_index());
                    self.infer_expression(scope, then);
                    self.add_type(then, TypeSentinel::Unit.to_index());
                    self.add_type(i, TypeSentinel::Unit.to_index());
                }
                SemKind::IfElse {
                    condition,
                    then,
                    else_,
                } => {
                    let condition = *condition;
                    let then = *then;
                    let else_ = *else_;

                    self.add_type(condition, TypeSentinel::Bool.to_index());

                    self.infer_expression(scope, then);
                    self.infer_expression(scope, else_);

                    let then_type = match self.semantic.get(then) {
                        Val::None => panic!(),
                        Val::Value(sem_data) => sem_data.ty,
                    };
                    let else_type = match self.semantic.get(then) {
                        Val::None => panic!(),
                        Val::Value(sem_data) => sem_data.ty,
                    };

                    self.add_type(then, else_type);
                    self.add_type(else_, then_type);
                    self.add_type(i, then_type);
                }
                SemKind::BuildStruct { fields } => {
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
                                    match self.semantic.get(*value) {
                                        Val::None => panic!(),
                                        Val::Value(sem_data) => sem_data.ty,
                                    },
                                )
                            })
                            .collect::<Vec<(String, Type)>>(),
                    });

                    self.add_type(i, type_);
                }
                SemKind::ChainOpen {
                    statements,
                    expression,
                } => {
                    let expression = *expression;

                    for statement in statements.clone() {
                        self.infer_expression(scope, statement);
                    }

                    self.infer_expression(scope, expression);

                    if let Val::Value(sem_data) = self.semantic.get(expression) {
                        self.add_type(i, sem_data.ty);
                    }
                }
                SemKind::ChainClosed { statements } => {
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
