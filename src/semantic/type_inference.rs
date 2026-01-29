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
        let print = self.types.push(TypeData::Function {
            argument_type: TypeSentinel::Uint32.to_index(),
            return_type: TypeSentinel::Unit.to_index(),
        });

        let mut scope = Scope::from([("print".to_string(), ScopeItem::Type(print))]);

        let uint32_tuple = self.types.push(TypeData::Product {
            fields: vec![
                ("0".to_string(), TypeSentinel::Uint32.to_index()),
                ("1".to_string(), TypeSentinel::Uint32.to_index()),
            ],
        });
        let binary_function_type = self.types.push(TypeData::Function {
            argument_type: uint32_tuple,
            return_type: TypeSentinel::Uint32.to_index(),
        });

        for name in [
            "builtin_equal",
            "builtin_add",
            "builtin_sub",
            "builtin_mul",
            "builtin_div",
        ] {
            scope.insert(name.to_string(), ScopeItem::Type(binary_function_type));
        }

        self.infer_expression(&scope, semantic::ROOT_SEM);
    }

    fn add_type(&mut self, sem: Sem, type_: Type) {
        self.semantic.types[sem] = combine_types(self.types, self.semantic.types[sem], type_);
    }

    fn infer_expression(&mut self, scope: &Scope, i: Sem) {
        match &self.semantic.kinds[i] {
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
                    .map(|(name, value)| (name.clone(), { self.semantic.types[*value] }))
                    .collect::<Vec<(String, Type)>>();

                let type_ = if fields.is_empty() {
                    TypeSentinel::Unit.to_index()
                } else {
                    self.types.push(TypeData::Product { fields })
                };

                self.add_type(i, type_);
            }
            SemKind::Function { argument, body } => {
                let body = *body;

                let mut scope = scope.clone();
                scope.insert(argument.clone(), ScopeItem::Argument(i));

                {
                    let (argument_type, return_type) = match self.types.get(self.semantic.types[i])
                    {
                        Val::None => panic!(),
                        Val::Sentinel(_) => panic!(),
                        Val::Value(type_data) => match type_data {
                            TypeData::Function {
                                argument_type,
                                return_type,
                            } => (*argument_type, *return_type),
                            _ => panic!(),
                        },
                    };

                    let body_type = self.semantic.types[body];

                    let return_type = combine_types(self.types, body_type, return_type);

                    let type_ = self.types.push(TypeData::Function {
                        argument_type,
                        return_type,
                    });
                    self.add_type(i, type_);
                }

                self.infer_expression(&scope, body);

                {
                    let sem_type = self.semantic.types[i];

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

                    let body_type = self.semantic.types[body];

                    let return_type = combine_types(self.types, body_type, return_type);

                    let type_ = self.types.push(TypeData::Function {
                        argument_type,
                        return_type,
                    });
                    self.add_type(i, type_);
                }
            }
            SemKind::Binding { name, value, body } | SemKind::MutBinding { name, value, body } => {
                let name = name.clone();
                let value = *value;
                let body = *body;

                self.infer_expression(scope, value);

                let mut scope = scope.clone();
                scope.insert(name, ScopeItem::Sem(value));

                self.infer_expression(&scope, body);

                let body_type = self.semantic.types[body];

                self.add_type(i, body_type);
            }
            SemKind::Assignment { binding, value } => {
                let value = *value;

                match scope[binding] {
                    ScopeItem::Sem(sem) => self.add_type(value, self.semantic.types[sem]),
                    ScopeItem::Argument(sem) => {
                        match self.types.get(self.semantic.types[sem]) {
                            Val::None => panic!(),
                            Val::Value(TypeData::Function {
                                argument_type,
                                return_type: _,
                            }) => self.add_type(value, *argument_type),
                            Val::Sentinel(_) | Val::Value(_) => {}
                        }

                        todo!()
                    }
                    ScopeItem::Type(_) => panic!(),
                };

                self.infer_expression(scope, value);
            }
            SemKind::Reference { name } => {
                let type_ = match scope[name] {
                    ScopeItem::Sem(sem) => self.semantic.types[sem],
                    ScopeItem::Argument(sem) => match self.types.get(self.semantic.types[sem]) {
                        Val::Value(TypeData::Function { argument_type, .. }) => *argument_type,
                        Val::None | Val::Sentinel(_) | Val::Value(_) => panic!(),
                    },
                    ScopeItem::Type(ty) => ty,
                };

                self.add_type(i, type_);
            }
            SemKind::Access { field, expr } => {
                let field = field.clone();
                let expr = *expr;

                self.infer_expression(scope, expr);

                match self.types.get(self.semantic.types[expr]) {
                    Val::None => panic!(),
                    // TODO: Infer for unknown types
                    Val::Sentinel(_) => {}
                    Val::Value(TypeData::Product { fields }) => {
                        if let Some((_, field_type)) =
                            fields.iter().find(|(name, _)| name == &field)
                        {
                            self.add_type(i, *field_type);
                        }
                    }
                    Val::Value(_) => panic!(),
                }
            }
            SemKind::Application { function, argument } => {
                let function = *function;
                let argument = *argument;

                self.infer_expression(scope, function);
                self.infer_expression(scope, argument);

                match self.types.get(self.semantic.types[function]) {
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

                let then_type = self.semantic.types[then];
                let else_type = self.semantic.types[else_];

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
                        .map(|(name, value)| (name.clone(), self.semantic.types[*value]))
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

                self.add_type(i, self.semantic.types[expression]);
            }
            SemKind::ChainClosed { statements } => {
                for statement in statements.clone() {
                    self.infer_expression(scope, statement);
                }

                self.add_type(i, TypeSentinel::Unit.to_index());
            }
        }
    }
}
