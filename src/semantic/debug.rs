use std::fmt::{Debug, Display};

use colored::Colorize as _;

use crate::{
    key_vec::Val,
    semantic::{ROOT_SEM, Sem, SemKind, Semantic, Type, TypeData, TypeSentinel, Types},
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

impl Debug for DebugSem<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sem = |sem| DebugSem {
            semantic: self.semantic,
            types: self.types,
            sem,
        };

        let mut display = |name: &str, children: &[&dyn Debug]| {
            children
                .iter()
                .fold(
                    &mut f.debug_tuple(name.bright_green().to_string().as_str()),
                    |tuple, field| tuple.field(field),
                )
                .finish()
        };

        match &self.semantic.kinds[self.sem] {
            SemKind::Number(_) => display("number", &[]),
            SemKind::False(_) => display("false", &[]),
            SemKind::True(_) => display("true", &[]),
            SemKind::Module { bindings } => bindings
                .iter()
                .fold(
                    &mut f.debug_struct("module".bright_green().to_string().as_str()),
                    |structure, (name, value)| {
                        structure.field(&name.bright_cyan().to_string(), &sem(*value))
                    },
                )
                .finish(),
            SemKind::Function { argument, body } => display(
                "function",
                &[&DebugUsingDisplay(argument.bright_cyan()), &sem(*body)],
            ),
            SemKind::Binding { name, value, body } => display(
                "binding",
                &[
                    &DebugUsingDisplay(name.bright_cyan()),
                    &sem(*value),
                    &sem(*body),
                ],
            ),
            SemKind::MutBinding { name, value, body } => display(
                "mut_binding",
                &[
                    &DebugUsingDisplay(name.bright_cyan()),
                    &sem(*value),
                    &sem(*body),
                ],
            ),
            SemKind::Reference { name } => f.write_str(&format!(
                "{}({})",
                "reference".bright_green(),
                name.bright_cyan()
            )),
            SemKind::Access { field, expr } => display(
                "access",
                &[&DebugUsingDisplay(field.bright_cyan()), &sem(*expr)],
            ),
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
                    |structure, (name, value)| {
                        structure.field(&name.bright_cyan().to_string(), &sem(*value))
                    },
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
        }?;

        f.write_str(&": ".white().to_string())?;
        f.write_str(&debug_type(self.types, self.semantic.types[self.sem]))
    }
}

struct DebugUsingDisplay<T>(T);

impl<T: Display> Debug for DebugUsingDisplay<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub fn debug_type(types: &Types, type_: Type) -> String {
    match types.get(type_) {
        Val::None => panic!(),
        Val::Sentinel(sentinel) => {
            let text = match sentinel {
                TypeSentinel::Unknown => "unknown",
                TypeSentinel::Unit => "()",
                TypeSentinel::Uint32 => "u32",
                TypeSentinel::Bool => "bool",
                TypeSentinel::False => "false",
                TypeSentinel::True => "true",
            };

            text.bright_blue().to_string()
        }
        Val::Value(type_data) => match type_data {
            TypeData::Function {
                argument_type,
                return_type,
            } => {
                let mut text = debug_type(types, *argument_type);
                text.push_str(" -> ");
                text.push_str(&debug_type(types, *return_type));
                text
            }
            TypeData::Product { fields } => {
                let mut text = "(".to_string();

                for (i, (_, field)) in fields.iter().enumerate() {
                    if i != 0 {
                        text.push_str(", ");
                    }

                    text.push_str(&debug_type(types, *field));
                }

                text.push(')');

                text
            }
        },
    }
}
