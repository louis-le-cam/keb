use colored::Colorize as _;

use crate::{
    key_vec::Val,
    semantic::{ROOT_SEM, Sem, SemKind, Semantic, TypeSentinel, Types},
};

pub fn debug(semantic: &Semantic, types: &Types) {
    println!(
        "{:#?}",
        DebugSem {
            semantic,
            types,
            sem: ROOT_SEM
        }
    );
}

struct DebugSem<'a> {
    semantic: &'a Semantic,
    types: &'a Types,
    sem: Sem,
}

impl std::fmt::Debug for DebugSem<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sem = |sem| DebugSem {
            semantic: self.semantic,
            types: self.types,
            sem,
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

        match self.semantic.get(self.sem) {
            Val::None => panic!(),
            Val::Value(sem_data) => match &sem_data.kind {
                SemKind::Number(token) => display("number", &[token]),
                SemKind::False(token) => display("false", &[token]),
                SemKind::True(token) => display("true", &[token]),
                SemKind::Module { bindings } => bindings
                    .iter()
                    .fold(
                        &mut f.debug_struct("module".bright_green().to_string().as_str()),
                        |structure, (name, value)| structure.field(name, &sem(*value)),
                    )
                    .finish(),
                SemKind::Function { argument, body } => {
                    display("function", &[argument, &sem(*body)])
                }
                SemKind::Binding { name, value, body } => {
                    display("binding", &[name, &sem(*value), &sem(*body)])
                }
                SemKind::Reference { name } => display("reference", &[name]),
                SemKind::Access { field, expr } => display("access", &[field, &sem(*expr)]),
                SemKind::Application { function, argument } => {
                    display("application", &[&sem(*function), &sem(*argument)])
                }
                SemKind::Loop(body) => display("loop", &[&sem(*body)]),
                SemKind::If { condition, then } => display("if", &[&sem(*condition), &sem(*then)]),
                SemKind::IfElse {
                    condition,
                    then,
                    else_,
                } => display("if", &[&sem(*condition), &sem(*then), &sem(*else_)]),
                SemKind::BuildStruct { fields } => fields
                    .iter()
                    .fold(
                        &mut f.debug_struct("build_struct".bright_green().to_string().as_str()),
                        |structure, (name, value)| structure.field(name, &sem(*value)),
                    )
                    .finish(),
                SemKind::ChainOpen {
                    statements,
                    expression,
                } => statements
                    .iter()
                    .chain([expression])
                    .fold(
                        &mut f.debug_tuple("chain_open".bright_green().to_string().as_str()),
                        |structure, expression| structure.field(&sem(*expression)),
                    )
                    .finish(),
                SemKind::ChainClosed { statements } => statements
                    .iter()
                    .fold(
                        &mut f.debug_tuple("chain_closed".bright_green().to_string().as_str()),
                        |structure, expression| structure.field(&sem(*expression)),
                    )
                    .finish(),
            },
        }?;

        f.write_str(&match self.semantic.get(self.sem) {
            Val::None => panic!(),
            Val::Value(sem_data) => match self.types.get(sem_data.ty) {
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
