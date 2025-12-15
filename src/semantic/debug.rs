use colored::Colorize as _;

use crate::{
    key_vec::Val,
    semantic::{Node, NodeKind, Nodes, ROOT_NODE, TypeSentinel, Types},
};

pub fn debug(nodes: &Nodes, types: &Types) {
    println!(
        "{:#?}",
        DebugNode {
            nodes,
            types,
            node: ROOT_NODE
        }
    );
}

struct DebugNode<'a> {
    nodes: &'a Nodes,
    types: &'a Types,
    node: Node,
}

impl std::fmt::Debug for DebugNode<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let node = |node| DebugNode {
            nodes: self.nodes,
            types: self.types,
            node,
        };

        let mut display = |name: &str, children: &[&dyn std::fmt::Debug]| {
            children
                .iter()
                .fold(
                    &mut f.debug_tuple(name.bright_green().to_string().as_str()),
                    |tuple, field| tuple.field(field),
                )
                .finish()
        };

        match self.nodes.get(self.node) {
            Val::None => panic!(),
            Val::Value(node_data) => match &node_data.kind {
                NodeKind::Number(token) => display("number", &[token]),
                NodeKind::False(token) => display("false", &[token]),
                NodeKind::True(token) => display("true", &[token]),
                NodeKind::Module { bindings } => bindings
                    .iter()
                    .fold(
                        &mut f.debug_struct("module".bright_green().to_string().as_str()),
                        |structure, (name, value)| structure.field(name, &node(*value)),
                    )
                    .finish(),
                NodeKind::Function { argument, body } => {
                    display("function", &[argument, &node(*body)])
                }
                NodeKind::Binding { name, value, body } => {
                    display("binding", &[name, &node(*value), &node(*body)])
                }
                NodeKind::Reference { name } => display("reference", &[name]),
                NodeKind::Access { field, expr } => display("access", &[field, &node(*expr)]),
                NodeKind::Application { function, argument } => {
                    display("application", &[&node(*function), &node(*argument)])
                }
                NodeKind::BuildStruct { fields } => fields
                    .iter()
                    .fold(
                        &mut f.debug_struct("build_struct".bright_green().to_string().as_str()),
                        |structure, (name, value)| structure.field(name, &node(*value)),
                    )
                    .finish(),
            },
        }?;

        f.write_str(&match self.nodes.get(self.node) {
            Val::None => panic!(),
            Val::Value(node_data) => match self.types.get(node_data.ty) {
                Val::None => panic!(),
                Val::Sentinel(sentinel) => match sentinel {
                    TypeSentinel::Unknown => ": unknown".bright_blue().to_string(),
                    TypeSentinel::Unit => ": ()".bright_blue().to_string(),
                    TypeSentinel::Uint32 => ": u32".bright_blue().to_string(),
                    TypeSentinel::Bool => ": bool".bright_blue().to_string(),
                    TypeSentinel::False => ": false".bright_blue().to_string(),
                    TypeSentinel::True => ": true".bright_blue().to_string(),
                },
                Val::Value(type_data) => format!(": {:?}", type_data)
                    .to_string()
                    .bright_blue()
                    .to_string(),
            },
        })
    }
}
