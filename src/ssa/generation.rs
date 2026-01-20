use std::collections::HashMap;

use crate::{
    key_vec::{Sentinel, Val},
    semantic::{self, Node, NodeData, NodeKind, Nodes, TypeData, TypeSentinel, Types},
    token::{self, Tokens},
};

use super::*;

pub fn generate(source: &str, tokens: &Tokens, semantic: &Nodes, types: &mut Types) -> Ssa {
    let Val::Value(NodeData {
        kind: NodeKind::Module { bindings },
        ..
    }) = &semantic.get(semantic::ROOT_NODE)
    else {
        panic!();
    };

    let mut ssa = Ssa::default();

    let builtin_add = {
        let u32_tuple = types.push(TypeData::Product {
            fields: vec![
                (String::new(), TypeSentinel::Uint32.to_index()),
                (String::new(), TypeSentinel::Uint32.to_index()),
            ],
        });
        let builtin_add = ssa.function(
            "builtin_add".to_string(),
            u32_tuple,
            TypeSentinel::Uint32.to_index(),
        );
        let lhs = ssa.inst_field(builtin_add, Expr::BlockArg(builtin_add), 0);
        let rhs = ssa.inst_field(builtin_add, Expr::BlockArg(builtin_add), 1);
        let result = ssa.inst_add(builtin_add, Expr::Inst(lhs), Expr::Inst(rhs));
        ssa.inst_return(builtin_add, Expr::Inst(result));

        builtin_add
    };

    let mut functions = HashMap::from([
        (
            "print".to_string(),
            ssa.extern_function(
                "builtin_print".to_string(),
                TypeSentinel::Uint32.to_index(),
                TypeSentinel::Unit.to_index(),
            ),
        ),
        ("builtin_add".to_string(), builtin_add),
    ]);

    for (name, value) in bindings {
        let Val::Value(NodeData {
            kind: NodeKind::Function { .. },
            ty,
        }) = &semantic.get(*value)
        else {
            panic!();
        };

        let Val::Value(TypeData::Function {
            argument_type,
            return_type,
        }) = types.get(*ty)
        else {
            panic!()
        };

        functions.insert(
            name.clone(),
            ssa.function(name.clone(), *argument_type, *return_type),
        );
    }

    for (name, value) in bindings {
        let Val::Value(NodeData {
            kind: NodeKind::Function { argument, body },
            ..
        }) = &semantic.get(*value)
        else {
            panic!();
        };

        let mut block = functions[name.as_str()];
        let bindings = HashMap::from([(argument.clone(), Expr::BlockArg(block))]);

        let expr = generate_expression(
            &mut ssa, &mut block, source, tokens, semantic, types, *body, &bindings, &functions,
        );
        ssa.inst_return(block, expr);
    }

    ssa
}

fn generate_expression(
    ssa: &mut Ssa,
    block: &mut Block,
    source: &str,
    tokens: &Tokens,
    semantic: &Nodes,
    types: &mut Types,
    node: Node,
    bindings: &HashMap<String, Expr>,
    functions: &HashMap<String, Block>,
) -> Expr {
    match semantic.get(node) {
        Val::None => panic!(),
        Val::Value(node_data) => match &node_data.kind {
            NodeKind::Number(token) => {
                let value = token::parse_u64(source, tokens, *token) as u32;
                Expr::Const(ssa.const_u32(value))
            }
            NodeKind::False(_) => Expr::Const(ConstSentinel::False.to_index()),
            NodeKind::True(_) => Expr::Const(ConstSentinel::True.to_index()),
            NodeKind::Module { .. } => todo!(),
            NodeKind::Function { .. } => todo!(),
            NodeKind::Binding { name, value, body } => {
                let value = generate_expression(
                    ssa, block, source, tokens, semantic, types, *value, bindings, functions,
                );
                let mut bindings = bindings.clone();
                bindings.insert(name.to_string(), value);

                generate_expression(
                    ssa, block, source, tokens, semantic, types, *body, &bindings, functions,
                )
            }
            NodeKind::Reference { name } => bindings[name],
            NodeKind::Access { field, expr } => {
                let Val::Value(NodeData { ty, .. }) = semantic.get(*expr) else {
                    panic!()
                };

                let field_index = match types.get(*ty) {
                    Val::Value(TypeData::Product { fields }) => {
                        fields.iter().position(|(name, _)| field == name).unwrap()
                    }
                    Val::None | Val::Sentinel(_) | Val::Value(_) => panic!(),
                };

                let expr = generate_expression(
                    ssa, block, source, tokens, semantic, types, *expr, bindings, functions,
                );

                Expr::Inst(ssa.inst_field(*block, expr, field_index as u32))
            }
            NodeKind::Application { function, argument } => {
                let argument = generate_expression(
                    ssa, block, source, tokens, semantic, types, *argument, bindings, functions,
                );

                let Val::Value(NodeData {
                    kind: NodeKind::Reference { name },
                    ..
                }) = &semantic.get(*function)
                else {
                    panic!();
                };

                Expr::Inst(ssa.inst_call(*block, functions[name], argument))
            }
            NodeKind::Loop(body) => {
                let loop_block = ssa.basic_block(TypeSentinel::Unit.to_index());
                ssa.inst_jump(
                    *block,
                    loop_block,
                    Expr::Const(ConstSentinel::Unit.to_index()),
                );
                *block = loop_block;

                generate_expression(
                    ssa, block, source, tokens, semantic, types, *body, bindings, functions,
                );

                ssa.inst_jump(*block, *block, Expr::Const(ConstSentinel::Unit.to_index()));
                Expr::Const(ConstSentinel::Unit.to_index())
            }
            NodeKind::If { condition, then } => {
                let condition = generate_expression(
                    ssa, block, source, tokens, semantic, types, *condition, bindings, functions,
                );

                let mut then_block = ssa.basic_block(TypeSentinel::Unit.to_index());
                let after_block = ssa.basic_block(TypeSentinel::Unit.to_index());

                ssa.inst_jump_condition(*block, condition, then_block, after_block);

                generate_expression(
                    ssa,
                    &mut then_block,
                    source,
                    tokens,
                    semantic,
                    types,
                    *then,
                    bindings,
                    functions,
                );

                ssa.inst_jump(
                    then_block,
                    after_block,
                    Expr::Const(ConstSentinel::Unit.to_index()),
                );

                *block = after_block;

                Expr::Const(ConstSentinel::Unit.to_index())
            }
            NodeKind::IfElse {
                condition,
                then,
                else_,
            } => {
                let condition = generate_expression(
                    ssa, block, source, tokens, semantic, types, *condition, bindings, functions,
                );

                let mut then_block = ssa.basic_block(TypeSentinel::Unit.to_index());
                let mut else_block = ssa.basic_block(TypeSentinel::Unit.to_index());

                ssa.inst_jump_condition(*block, condition, then_block, else_block);

                let then_expr = generate_expression(
                    ssa,
                    &mut then_block,
                    source,
                    tokens,
                    semantic,
                    types,
                    *then,
                    bindings,
                    functions,
                );

                let else_expr = generate_expression(
                    ssa,
                    &mut else_block,
                    source,
                    tokens,
                    semantic,
                    types,
                    *else_,
                    bindings,
                    functions,
                );

                let Val::Value(NodeData { ty: type_, kind: _ }) = semantic.get(*then) else {
                    panic!()
                };

                let after_block = ssa.basic_block(*type_);
                ssa.inst_jump(then_block, after_block, then_expr);
                ssa.inst_jump(else_block, after_block, else_expr);

                *block = after_block;

                Expr::BlockArg(after_block)
            }
            NodeKind::BuildStruct { fields } => {
                let fields = fields
                    .iter()
                    .map(|(_, value)| {
                        generate_expression(
                            ssa, block, source, tokens, semantic, types, *value, bindings,
                            functions,
                        )
                    })
                    .collect();

                Expr::Inst(ssa.inst_product(types, *block, fields))
            }
            NodeKind::ChainOpen {
                statements,
                expression,
            } => {
                for statement in statements {
                    generate_expression(
                        ssa, block, source, tokens, semantic, types, *statement, bindings,
                        functions,
                    );
                }

                generate_expression(
                    ssa,
                    block,
                    source,
                    tokens,
                    semantic,
                    types,
                    *expression,
                    bindings,
                    functions,
                )
            }
            NodeKind::ChainClosed { statements } => {
                for statement in statements {
                    generate_expression(
                        ssa, block, source, tokens, semantic, types, *statement, bindings,
                        functions,
                    );
                }

                Expr::Const(ConstSentinel::Unit.to_index())
            }
        },
    }
}
