use std::collections::{HashMap, HashSet};

use crate::{
    key_vec::{Sentinel, Val},
    semantic::{self, Sem, SemKind, Semantic, TypeData, TypeSentinel, Types},
    token::{self, TokenOffsets},
};

use super::*;

pub fn generate(
    source: &str,
    tokens: &TokenOffsets,
    semantic: &Semantic,
    types: &mut Types,
) -> Ssa {
    let mut generator = Generator {
        source,
        tokens,
        semantic,
        types,
        ssa: Ssa::default(),
    };

    generator.generate_module();

    generator.ssa
}

struct Generator<'a> {
    source: &'a str,
    tokens: &'a TokenOffsets,
    semantic: &'a Semantic,
    types: &'a mut Types,
    ssa: Ssa,
}

impl Generator<'_> {
    fn generate_module(&mut self) {
        let SemKind::Module { bindings } = &self.semantic.kinds[semantic::ROOT_SEM] else {
            panic!();
        };

        let mut functions = HashMap::from([(
            "print".to_string(),
            self.ssa.extern_function(
                "builtin_print".to_string(),
                TypeSentinel::Uint32.to_index(),
                TypeSentinel::Unit.to_index(),
            ),
        )]);

        let u32_tuple = self.types.push(TypeData::Product {
            fields: vec![
                (String::new(), TypeSentinel::Uint32.to_index()),
                (String::new(), TypeSentinel::Uint32.to_index()),
            ],
        });

        for (name, inst_data) in [
            ("builtin_add", InstData::Add as fn(Expr, Expr) -> InstData),
            ("builtin_sub", InstData::Sub),
            ("builtin_mul", InstData::Mul),
            ("builtin_div", InstData::Div),
        ] {
            let function =
                self.ssa
                    .function(name.to_string(), u32_tuple, TypeSentinel::Uint32.to_index());
            let lhs = self.ssa.inst_field(function, Expr::BlockArg(function), 0);
            let rhs = self.ssa.inst_field(function, Expr::BlockArg(function), 1);
            let result = self
                .ssa
                .inst(function, inst_data(Expr::Inst(lhs), Expr::Inst(rhs)));
            self.ssa.inst_return(function, Expr::Inst(result));

            functions.insert(name.to_string(), function);
        }

        for (name, value) in bindings {
            let SemKind::Function { .. } = self.semantic.kinds[*value] else {
                panic!()
            };

            let Val::Value(TypeData::Function {
                argument_type,
                return_type,
            }) = self.types.get(self.semantic.types[*value])
            else {
                panic!()
            };

            functions.insert(
                name.clone(),
                self.ssa
                    .function(name.clone(), *argument_type, *return_type),
            );
        }

        for (name, value) in bindings {
            let SemKind::Function { argument, body } = &self.semantic.kinds[*value] else {
                panic!();
            };

            let mut block = functions[name.as_str()];
            let mut scope = Scope {
                parent: None,
                mutable_bindings: HashSet::new(),
                bindings: HashMap::from([(argument.clone(), Expr::BlockArg(block))]),
                functions: functions.clone(),
            };

            let expr = self.generate_expression(&mut block, *body, &mut scope);
            self.ssa.inst_return(block, expr);
        }
    }

    pub fn generate_expression(&mut self, block: &mut Block, sem: Sem, scope: &mut Scope) -> Expr {
        match &self.semantic.kinds[sem] {
            SemKind::Number(token) => {
                let value = token::parse_u64(self.source, &self.tokens, *token) as u32;
                Expr::Const(self.ssa.const_u32(value))
            }
            SemKind::False(_) => Expr::Const(ConstSentinel::False.to_index()),
            SemKind::True(_) => Expr::Const(ConstSentinel::True.to_index()),
            SemKind::Module { .. } => todo!(),
            SemKind::Function { .. } => todo!(),
            SemKind::Binding { name, value, body } | SemKind::MutBinding { name, value, body } => {
                let value = self.generate_expression(block, *value, scope);

                let mutable_bindings = if let SemKind::MutBinding { .. } = self.semantic.kinds[sem]
                {
                    HashSet::from([name.to_string()])
                } else {
                    HashSet::new()
                };

                let mut scope = Scope {
                    parent: Some(scope),
                    mutable_bindings,
                    bindings: HashMap::from([(name.to_string(), value)]),
                    functions: HashMap::new(),
                };

                self.generate_expression(block, *body, &mut scope)
            }
            SemKind::Assignment { binding, value } => {
                // TODO: Does not work with branching, it will take the value
                // from the last branch because we just replace the binding.
                // Should we use block arguments?
                assert!(scope.is_mutable(binding));
                let value = self.generate_expression(block, *value, scope);
                scope.bindings.insert(binding.clone(), value).unwrap();
                Expr::Const(ConstSentinel::Unit.to_index())
            }
            SemKind::Reference { name } => scope.binding(name).unwrap(),
            SemKind::Access { field, expr } => {
                let field_index = match self.types.get(self.semantic.types[*expr]) {
                    Val::Value(TypeData::Product { fields }) => {
                        fields.iter().position(|(name, _)| field == name).unwrap()
                    }
                    Val::None | Val::Sentinel(_) | Val::Value(_) => panic!(),
                };

                let expr = self.generate_expression(block, *expr, scope);

                Expr::Inst(self.ssa.inst_field(*block, expr, field_index as u32))
            }
            SemKind::Application { function, argument } => {
                let argument = self.generate_expression(block, *argument, scope);

                let SemKind::Reference { name } = &self.semantic.kinds[*function] else {
                    panic!();
                };

                Expr::Inst(
                    self.ssa
                        .inst_call(*block, scope.function(name).unwrap(), argument),
                )
            }
            SemKind::Loop(body) => {
                let loop_block = self.ssa.basic_block(TypeSentinel::Unit.to_index());
                self.ssa.inst_jump(
                    *block,
                    loop_block,
                    Expr::Const(ConstSentinel::Unit.to_index()),
                );
                *block = loop_block;

                self.generate_expression(block, *body, scope);

                self.ssa
                    .inst_jump(*block, *block, Expr::Const(ConstSentinel::Unit.to_index()));
                Expr::Const(ConstSentinel::Unit.to_index())
            }
            SemKind::If { condition, then } => {
                let condition = self.generate_expression(block, *condition, scope);

                let mut then_block = self.ssa.basic_block(TypeSentinel::Unit.to_index());
                let after_block = self.ssa.basic_block(TypeSentinel::Unit.to_index());

                self.ssa
                    .inst_jump_condition(*block, condition, then_block, after_block);

                self.generate_expression(&mut then_block, *then, scope);

                self.ssa.inst_jump(
                    then_block,
                    after_block,
                    Expr::Const(ConstSentinel::Unit.to_index()),
                );

                *block = after_block;

                Expr::Const(ConstSentinel::Unit.to_index())
            }
            SemKind::IfElse {
                condition,
                then,
                else_,
            } => {
                let condition = self.generate_expression(block, *condition, scope);

                let mut then_block = self.ssa.basic_block(TypeSentinel::Unit.to_index());
                let mut else_block = self.ssa.basic_block(TypeSentinel::Unit.to_index());

                self.ssa
                    .inst_jump_condition(*block, condition, then_block, else_block);

                let then_expr = self.generate_expression(&mut then_block, *then, scope);

                let else_expr = self.generate_expression(&mut else_block, *else_, scope);

                let after_block = self.ssa.basic_block(self.semantic.types[*then]);
                self.ssa.inst_jump(then_block, after_block, then_expr);
                self.ssa.inst_jump(else_block, after_block, else_expr);

                *block = after_block;

                Expr::BlockArg(after_block)
            }
            SemKind::BuildStruct { fields } => {
                let fields = fields
                    .iter()
                    .map(|(_, value)| self.generate_expression(block, *value, scope))
                    .collect();

                Expr::Inst(self.ssa.inst_product(self.types, *block, fields))
            }
            SemKind::ChainOpen {
                statements,
                expression,
            } => {
                for statement in statements {
                    self.generate_expression(block, *statement, scope);
                }

                self.generate_expression(block, *expression, scope)
            }
            SemKind::ChainClosed { statements } => {
                for statement in statements {
                    self.generate_expression(block, *statement, scope);
                }

                Expr::Const(ConstSentinel::Unit.to_index())
            }
        }
    }
}

struct Scope<'a> {
    parent: Option<&'a Scope<'a>>,
    mutable_bindings: HashSet<String>,
    bindings: HashMap<String, Expr>,
    functions: HashMap<String, Block>,
}

impl Scope<'_> {
    fn is_mutable(&self, name: &str) -> bool {
        self.mutable_bindings.contains(name)
            || self
                .parent
                .map(|parent| parent.is_mutable(name))
                .unwrap_or(false)
    }

    fn binding(&self, name: &str) -> Option<Expr> {
        self.bindings
            .get(name)
            .copied()
            .or_else(|| self.parent.and_then(|parent| parent.binding(name)))
    }

    fn function(&self, name: &str) -> Option<Block> {
        self.functions
            .get(name)
            .copied()
            .or_else(|| self.parent.and_then(|parent| parent.function(name)))
    }
}
