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
            SemKind::Assignment { binding, value } => display(
                "assignment",
                &[&DebugUsingDisplay(binding.bright_cyan()), &sem(*value)],
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

        write!(f, "{}", ": ".white().to_string())?;
        write!(f, "{}", self.semantic.types[self.sem].debug(&self.types))
    }
}

struct DebugUsingDisplay<T>(T);

impl<T: Display> Debug for DebugUsingDisplay<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Type {
    pub fn debug(self, types: &Types) -> impl Display {
        std::fmt::from_fn(move |f| match types.get(self) {
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

                write!(f, "{}", text.bright_blue().to_string())
            }
            Val::Value(type_data) => match type_data {
                TypeData::Function {
                    argument_type,
                    return_type,
                } => write!(
                    f,
                    "{} -> {}",
                    argument_type.debug(types),
                    return_type.debug(types)
                ),
                TypeData::Product { fields } => {
                    write!(f, "(")?;

                    for (i, (_, field)) in fields.iter().enumerate() {
                        if i != 0 {
                            write!(f, ", ")?;
                        }

                        write!(f, "{}", field.debug(types))?;
                    }

                    write!(f, ")")
                }
            },
        })
    }
}
