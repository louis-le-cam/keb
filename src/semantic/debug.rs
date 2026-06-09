use std::fmt::{Debug, Display};

use colored::Colorize as _;

use crate::{
    key_vec::Val,
    semantic::{ROOT_SEM, Sem, SemKind, Semantic, Type, TypeData, TypeSentinel, Types},
};

pub fn debug(semantic: &Semantic, types: &Types) {
    println!("{:#?}", ROOT_SEM.debug(semantic, types));
}

impl Sem {
    pub fn debug(self, semantic: &Semantic, types: &Types) -> impl Debug {
        std::fmt::from_fn(move |f| {
            match &semantic.kinds[self] {
                SemKind::Number(_) => f.debug_tuple(&"number".bright_green().to_string()).finish(),
                SemKind::False(_) => f.debug_tuple(&"false".bright_green().to_string()).finish(),
                SemKind::True(_) => f.debug_tuple(&"true".bright_green().to_string()).finish(),
                SemKind::Module { bindings } => bindings
                    .iter()
                    .fold(
                        &mut f.debug_struct(&"module".bright_green().to_string()),
                        |structure, (name, value)| {
                            structure.field(
                                &name.bright_cyan().to_string(),
                                &value.debug(semantic, types),
                            )
                        },
                    )
                    .finish(),
                SemKind::Function { argument, body } => f
                    .debug_tuple(&"function".bright_green().to_string())
                    .field(&debug_with_display(argument.bright_cyan()))
                    .field(&body.debug(semantic, types))
                    .finish(),
                SemKind::Binding { name, value, body } => f
                    .debug_tuple(&"binding".bright_green().to_string())
                    .field(&debug_with_display(name.bright_cyan()))
                    .field(&value.debug(semantic, types))
                    .field(&body.debug(semantic, types))
                    .finish(),
                SemKind::MutBinding { name, value, body } => f
                    .debug_tuple(&"mut_binding".bright_green().to_string())
                    .field(&debug_with_display(name.bright_cyan()))
                    .field(&value.debug(semantic, types))
                    .field(&body.debug(semantic, types))
                    .finish(),
                SemKind::Assignment { binding, value } => f
                    .debug_tuple(&"assignment".bright_green().to_string())
                    .field(&debug_with_display(binding.bright_cyan()))
                    .field(&value.debug(semantic, types))
                    .finish(),
                SemKind::Reference { name } => {
                    write!(f, "{}({})", "reference".bright_green(), name.bright_cyan())
                }
                SemKind::Access { field, expr } => f
                    .debug_tuple(&"access".bright_green().to_string())
                    .field(&debug_with_display(field.bright_cyan()))
                    .field(&expr.debug(semantic, types))
                    .finish(),
                SemKind::Application { function, argument } => f
                    .debug_tuple(&"application".bright_green().to_string())
                    .field(&function.debug(semantic, types))
                    .field(&argument.debug(semantic, types))
                    .finish(),
                SemKind::Loop(body) => f
                    .debug_tuple(&"loop".bright_green().to_string())
                    .field(&body.debug(semantic, types))
                    .finish(),
                SemKind::If { condition, then } => f
                    .debug_tuple(&"if".bright_green().to_string())
                    .field(&condition.debug(semantic, types))
                    .field(&then.debug(semantic, types))
                    .finish(),
                SemKind::IfElse {
                    condition,
                    then,
                    else_,
                } => f
                    .debug_tuple(&"if".bright_green().to_string())
                    .field(&condition.debug(semantic, types))
                    .field(&then.debug(semantic, types))
                    .field(&else_.debug(semantic, types))
                    .finish(),
                SemKind::BuildStruct { fields } => fields
                    .iter()
                    .fold(
                        &mut f.debug_struct(&"build_struct".bright_green().to_string()),
                        |structure, (name, value)| {
                            structure.field(
                                &name.bright_cyan().to_string(),
                                &value.debug(semantic, types),
                            )
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
                        &mut f.debug_tuple(&"chain_open".bright_green().to_string()),
                        |structure, expression| structure.field(&expression.debug(semantic, types)),
                    )
                    .finish(),
                SemKind::ChainClosed { statements } => statements
                    .iter()
                    .fold(
                        &mut f.debug_tuple(&"chain_closed".bright_green().to_string()),
                        |structure, expression| structure.field(&expression.debug(semantic, types)),
                    )
                    .finish(),
            }?;

            write!(f, "{}", ": ".white())?;
            write!(f, "{}", semantic.types[self].debug(&types))
        })
    }
}

pub fn debug_with_display(display: impl Display) -> impl Debug {
    std::fmt::from_fn(move |f| display.fmt(f))
}

impl Type {
    pub fn debug(self, types: &Types) -> impl Display {
        std::fmt::from_fn(move |f| match types.get(self) {
            Val::Sentinel(sentinel) => {
                let text = match sentinel {
                    TypeSentinel::Unknown => "unknown",
                    TypeSentinel::Unit => "()",
                    TypeSentinel::Uint32 => "u32",
                    TypeSentinel::Bool => "bool",
                    TypeSentinel::False => "false",
                    TypeSentinel::True => "true",
                };

                write!(f, "{}", text.bright_blue())
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
