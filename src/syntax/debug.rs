use std::fmt::Debug;

use colored::{ColoredString, Colorize};

use crate::syntax::{ROOT_SYN, StringSegment, Syn, SynData, Syntax};

pub fn debug(syntax: &Syntax) {
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
        // it's more than innacurate, but it does the job.
        // We also don't take into account the current indentation.
        if oneline_format.len() <= 50 {
            return f.write_str(&oneline_format);
        }

        self.0.fmt(f)
    }
}

struct DebugSyn<'a> {
    syntax: &'a Syntax,
    syn: Syn,
}

impl Debug for DebugSyn<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let debug_syn = |syn| {
            DebugWrapper(DebugSyn {
                syntax: self.syntax,
                syn,
            })
        };

        let (name, fields): (ColoredString, &[Syn]) = match &self.syntax[self.syn] {
            SynData::Root(syns) => ("root".bright_green(), syns.as_slice()),
            SynData::Ident(_) => ("ident".bright_cyan(), &[]),
            SynData::False(_) => ("false".bright_purple(), &[]),
            SynData::True(_) => ("true".bright_purple(), &[]),
            SynData::Number(_) => ("number".bright_purple(), &[]),
            SynData::Equal(lhs, rhs) => ("equal".bright_yellow(), &[*lhs, *rhs]),
            SynData::Add(lhs, rhs) => ("add".bright_yellow(), &[*lhs, *rhs]),
            SynData::Subtract(lhs, rhs) => ("sub".bright_yellow(), &[*lhs, *rhs]),
            SynData::Multiply(lhs, rhs) => ("mul".bright_yellow(), &[*lhs, *rhs]),
            SynData::Divide(lhs, rhs) => ("div".bright_yellow(), &[*lhs, *rhs]),
            SynData::Binding { pattern, value } => ("let".bright_red(), &[*pattern, *value]),
            SynData::Mut { pattern } => ("mut".bright_red(), &[*pattern]),
            SynData::Assignment { pattern, value } => {
                ("assignment".bright_yellow(), &[*pattern, *value])
            }
            SynData::Function { pattern, body } => ("function".bright_green(), &[*pattern, *body]),
            SynData::ReturnAscription { syn, type_ } => {
                ("return_ascription".white(), &[*syn, *type_])
            }
            SynData::Ascription { syn, type_ } => ("ascription".white(), &[*syn, *type_]),
            SynData::Access { syn, key } => ("access".white(), &[*syn, *key]),
            SynData::EmptyParen(_) => ("empty_paren".white(), &[]),
            SynData::Paren(expr) => ("paren".white(), &[*expr]),
            SynData::EmptyCurly(_) => ("empty_curly".white(), &[]),
            SynData::Curly(expr) => ("curly".white(), &[*expr]),
            SynData::Tuple(syns) => ("tuple".white(), syns.as_slice()),
            SynData::Application { function, argument } => {
                ("application".bright_green(), &[*function, *argument])
            }
            SynData::Loop(body) => ("loop".bright_red(), &[*body]),
            SynData::Match(content) => ("match".bright_red(), &[*content]),
            SynData::If { condition, then } => ("if".bright_red(), &[*condition, *then]),
            SynData::IfElse {
                condition,
                then,
                else_,
            } => ("if_else".bright_red(), &[*condition, *then, *else_]),
            SynData::ChainOpen(syns) => ("chain_open".white(), syns.as_slice()),
            SynData::ChainClosed(syns) => ("chain_closed".white(), syns.as_slice()),
            SynData::String(segments) => {
                return segments
                    .iter()
                    .fold(
                        &mut f.debug_tuple(&"chain_closed".white().to_string()),
                        |tuple, segment| match segment {
                            StringSegment::Token(token) => tuple.field(&token),
                            StringSegment::Interpolation(syn) => tuple.field(&debug_syn(*syn)),
                        },
                    )
                    .finish();
            }
        };

        fields
            .iter()
            .fold(&mut f.debug_tuple(&name.to_string()), |tuple, field| {
                tuple.field(&debug_syn(*field))
            })
            .finish()
    }
}
