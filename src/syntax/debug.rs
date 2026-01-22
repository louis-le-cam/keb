use std::fmt::Debug;

use colored::Colorize;

use crate::syntax::{ROOT_SYN, StringSegment, Syn, SynData, Syntax};

pub fn debug(syntax: &Syntax) {
    struct DebugSyn<'a> {
        syntax: &'a Syntax,
        syn: Syn,
    }

    impl Debug for DebugSyn<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let dbg_syn = |syn| {
                DebugWrapper(DebugSyn {
                    syntax: self.syntax,
                    syn,
                })
            };

            match self.syntax.get(self.syn).unwrap() {
                SynData::Root(syns) => syns
                    .iter()
                    .fold(
                        &mut f.debug_tuple(&"root".bright_green().to_string()),
                        |tuple, expr| tuple.field(&dbg_syn(*expr)),
                    )
                    .finish(),
                SynData::Ident { .. } => f.debug_tuple(&"ident".bright_cyan().to_string()).finish(),
                SynData::False { .. } => {
                    f.debug_tuple(&"false".bright_purple().to_string()).finish()
                }
                SynData::True { .. } => f.debug_tuple(&"true".bright_purple().to_string()).finish(),
                SynData::Number { .. } => f
                    .debug_tuple(&"number".bright_purple().to_string())
                    .finish(),
                SynData::Add(lhs, rhs) => f
                    .debug_tuple(&"add".bright_yellow().to_string())
                    .field(&dbg_syn(*lhs))
                    .field(&dbg_syn(*rhs))
                    .finish(),
                SynData::Subtract(lhs, rhs) => f
                    .debug_tuple(&"subtract".bright_yellow().to_string())
                    .field(&dbg_syn(*lhs))
                    .field(&dbg_syn(*rhs))
                    .finish(),
                SynData::Binding { pattern, value } => f
                    .debug_tuple(&"let".bright_red().to_string())
                    .field(&dbg_syn(*pattern))
                    .field(&dbg_syn(*value))
                    .finish(),
                SynData::Function { pattern, body } => f
                    .debug_tuple(&"function".bright_green().to_string())
                    .field(&dbg_syn(*pattern))
                    .field(&dbg_syn(*body))
                    .finish(),
                SynData::ReturnAscription {
                    syn: pattern,
                    type_,
                } => f
                    .debug_tuple(&"return_ascription".white().to_string())
                    .field(&dbg_syn(*pattern))
                    .field(&dbg_syn(*type_))
                    .finish(),
                SynData::Ascription { syn, type_ } => f
                    .debug_tuple(&"ascription".white().to_string())
                    .field(&dbg_syn(*syn))
                    .field(&dbg_syn(*type_))
                    .finish(),
                SynData::Access { syn, key } => f
                    .debug_tuple(&"access".white().to_string())
                    .field(&dbg_syn(*syn))
                    .field(&dbg_syn(*key))
                    .finish(),
                SynData::EmptyParen { .. } => {
                    f.debug_tuple(&"empty_paren".white().to_string()).finish()
                }
                SynData::Paren(expr) => f
                    .debug_tuple(&"paren".white().to_string())
                    .field(&dbg_syn(*expr))
                    .finish(),
                SynData::EmptyCurly { .. } => {
                    f.debug_tuple(&"empty_curly".white().to_string()).finish()
                }
                SynData::Curly(expr) => f
                    .debug_tuple(&"curly".white().to_string())
                    .field(&dbg_syn(*expr))
                    .finish(),
                SynData::Tuple(syns) => syns
                    .iter()
                    .fold(
                        &mut f.debug_tuple(&"tuple".white().to_string()),
                        |tuple, expr| tuple.field(&dbg_syn(*expr)),
                    )
                    .finish(),
                SynData::Application { function, argument } => f
                    .debug_tuple(&"application".bright_green().to_string())
                    .field(&dbg_syn(*function))
                    .field(&dbg_syn(*argument))
                    .finish(),
                SynData::Loop(body) => f
                    .debug_tuple(&"loop".bright_red().to_string())
                    .field(&dbg_syn(*body))
                    .finish(),
                SynData::If { condition, then } => f
                    .debug_tuple(&"if".bright_red().to_string())
                    .field(&dbg_syn(*condition))
                    .field(&dbg_syn(*then))
                    .finish(),
                SynData::IfElse {
                    condition,
                    then,
                    else_,
                } => f
                    .debug_tuple(&"if_else".bright_red().to_string())
                    .field(&dbg_syn(*condition))
                    .field(&dbg_syn(*then))
                    .field(&dbg_syn(*else_))
                    .finish(),
                SynData::ChainOpen(syns) => syns
                    .iter()
                    .fold(
                        &mut f.debug_tuple(&"chain_open".white().to_string()),
                        |tuple, expr| tuple.field(&dbg_syn(*expr)),
                    )
                    .finish(),
                SynData::ChainClosed(syns) => syns
                    .iter()
                    .fold(
                        &mut f.debug_tuple(&"chain_closed".white().to_string()),
                        |tuple, expr| tuple.field(&dbg_syn(*expr)),
                    )
                    .finish(),
                SynData::String(segments) => segments
                    .iter()
                    .fold(
                        &mut f.debug_tuple(&"chain_closed".white().to_string()),
                        |tuple, segment| match segment {
                            StringSegment::Token(token) => tuple.field(&token),
                            StringSegment::Interpolation(syn) => tuple.field(&dbg_syn(*syn)),
                        },
                    )
                    .finish(),
            }
        }
    }

    println!(
        "{:#?}",
        DebugSyn {
            syntax,
            syn: ROOT_SYN,
        }
    );
}

struct DebugWrapper<T>(T);

impl<T: Debug> Debug for DebugWrapper<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let oneline_format = format!("{:?}", self.0);
        // NOTE: The `len` includes control character for color so
        // it's more than innacurate, but it does the job
        if oneline_format.len() <= 50 {
            return f.write_str(&oneline_format);
        }

        self.0.fmt(f)
    }
}
