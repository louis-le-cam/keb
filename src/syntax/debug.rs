use std::fmt::Debug;

use colored::{ColoredString, Colorize};
use unicode_width::UnicodeWidthStr;

use crate::{
    key_vec::Value::{Item, Sentinel},
    syntax::{ROOT_SYN, Syn, SynKind, SynSentinel, Syntax},
};

pub fn debug(syntax: &Syntax) {
    println!("{:#?}", ROOT_SYN.debug(syntax));
}

fn debug_compact_wrapping(debug: impl Debug) -> impl Debug {
    std::fmt::from_fn(move |f| {
        let oneline_format = format!("{:?}", debug);

        // NOTE: We don't take indentation into account since `std::fmt` doesn't
        // seem to easily expose it.
        if strip_ansi_escapes::strip_str(&oneline_format).width() <= 40 {
            return f.write_str(&oneline_format);
        }

        debug.fmt(f)
    })
}

impl Syn {
    pub fn debug(self, syntax: &Syntax) -> impl Debug {
        debug_compact_wrapping(std::fmt::from_fn(move |f| {
            let (name, fields): (ColoredString, &[Syn]) = match syntax.kinds[self] {
                SynKind::Root => {
                    let mut debug_tuple = f.debug_tuple(&"root".bright_green().to_string());

                    let mut root = self;
                    loop {
                        assert_eq!(syntax.kinds[root], SynKind::Root);

                        let Item(syn) = syntax.lhs.get(root) else {
                            break;
                        };
                        let syn = Syn::from(*syn);
                        debug_tuple.field(&syn.debug(syntax));

                        let rhs = Syn::from(syntax.rhs[root]);
                        match rhs.sentinel() {
                            Some(SynSentinel::None) => break,
                            None => root = rhs,
                        };
                    }

                    return debug_tuple.finish();
                }
                SynKind::Ident => ("ident".bright_cyan(), &[]),
                SynKind::False => ("false".bright_purple(), &[]),
                SynKind::True => ("true".bright_purple(), &[]),
                SynKind::Number => ("number".bright_purple(), &[]),
                SynKind::Equal => (
                    "equal".bright_yellow(),
                    &[Syn::from(syntax.lhs[self]), Syn::from(syntax.rhs[self])],
                ),
                SynKind::Add => (
                    "add".bright_yellow(),
                    &[Syn::from(syntax.lhs[self]), Syn::from(syntax.rhs[self])],
                ),
                SynKind::Subtract => (
                    "sub".bright_yellow(),
                    &[Syn::from(syntax.lhs[self]), Syn::from(syntax.rhs[self])],
                ),
                SynKind::Multiply => (
                    "mul".bright_yellow(),
                    &[Syn::from(syntax.lhs[self]), Syn::from(syntax.rhs[self])],
                ),
                SynKind::Divide => (
                    "div".bright_yellow(),
                    &[Syn::from(syntax.lhs[self]), Syn::from(syntax.rhs[self])],
                ),
                SynKind::Binding => (
                    "let".bright_red(),
                    &[Syn::from(syntax.lhs[self]), Syn::from(syntax.rhs[self])],
                ),
                SynKind::Mut => ("mut".bright_red(), &[Syn::from(syntax.lhs[self])]),
                SynKind::Assignment => (
                    "assignment".bright_yellow(),
                    &[Syn::from(syntax.lhs[self]), Syn::from(syntax.rhs[self])],
                ),
                SynKind::Function => (
                    "function".bright_green(),
                    &[Syn::from(syntax.lhs[self]), Syn::from(syntax.rhs[self])],
                ),
                SynKind::ReturnAscription => (
                    "return_ascription".white(),
                    &[Syn::from(syntax.lhs[self]), Syn::from(syntax.rhs[self])],
                ),
                SynKind::Ascription => (
                    "ascription".white(),
                    &[Syn::from(syntax.lhs[self]), Syn::from(syntax.rhs[self])],
                ),
                SynKind::Access => (
                    "access".white(),
                    &[Syn::from(syntax.lhs[self]), Syn::from(syntax.rhs[self])],
                ),
                SynKind::Paren => (
                    "paren".white(),
                    match Syn::from(syntax.rhs[self]).sentinel() {
                        Some(SynSentinel::None) => &[],
                        None => &[Syn::from(syntax.rhs[self])],
                    },
                ),
                SynKind::Curly => (
                    "curly".white(),
                    match Syn::from(syntax.rhs[self]).sentinel() {
                        Some(SynSentinel::None) => &[],
                        None => &[Syn::from(syntax.rhs[self])],
                    },
                ),
                SynKind::Tuple => {
                    let mut debug_tuple = f.debug_tuple(&"tuple".white().to_string());

                    let mut tuple = self;
                    loop {
                        assert_eq!(syntax.kinds[tuple], SynKind::Tuple);

                        let Item(syn) = syntax.lhs.get(tuple) else {
                            break;
                        };
                        let syn = Syn::from(*syn);
                        debug_tuple.field(&syn.debug(syntax));

                        let rhs = Syn::from(syntax.rhs[tuple]);
                        match syntax.kinds.get(rhs) {
                            Sentinel(SynSentinel::None) => {
                                debug_tuple.field(&std::fmt::from_fn(|f| {
                                    write!(f, "{}", "closed".white())
                                }));
                                break;
                            }
                            Item(SynKind::Tuple) => tuple = rhs,
                            Item(_) => {
                                debug_tuple.field(&rhs.debug(syntax));
                                break;
                            }
                        };
                    }

                    return debug_tuple.finish();
                }
                SynKind::Application => (
                    "application".bright_green(),
                    &[Syn::from(syntax.lhs[self]), Syn::from(syntax.rhs[self])],
                ),
                SynKind::Loop => ("loop".bright_red(), &[Syn::from(syntax.lhs[self])]),
                SynKind::Match => ("match".bright_red(), &[Syn::from(syntax.lhs[self])]),
                SynKind::If => (
                    "if".bright_red(),
                    &[Syn::from(syntax.lhs[self]), Syn::from(syntax.rhs[self])],
                ),
                SynKind::Else => (
                    "else".bright_red(),
                    &[Syn::from(syntax.lhs[self]), Syn::from(syntax.rhs[self])],
                ),
                SynKind::Chain => {
                    let mut debug_tuple = f.debug_tuple(&"chain".white().to_string());

                    let mut chain = self;
                    loop {
                        assert_eq!(syntax.kinds[chain], SynKind::Chain);

                        let Item(syn) = syntax.lhs.get(chain) else {
                            break;
                        };
                        let syn = Syn::from(*syn);
                        debug_tuple.field(&syn.debug(syntax));

                        let rhs = Syn::from(syntax.rhs[chain]);
                        match syntax.kinds.get(rhs) {
                            Sentinel(SynSentinel::None) => {
                                debug_tuple.field(&std::fmt::from_fn(|f| {
                                    write!(f, "{}", "closed".white())
                                }));
                                break;
                            }
                            Item(SynKind::Chain) => chain = rhs,
                            Item(_) => {
                                debug_tuple.field(&rhs.debug(syntax));
                                break;
                            }
                        };
                    }

                    return debug_tuple.finish();
                }
                SynKind::String => {
                    todo!("Implement string syntax debug pretty-print")
                }
                SynKind::StringSegment | SynKind::StringInterpolation => {
                    unreachable!()
                }
            };

            fields
                .iter()
                .fold(&mut f.debug_tuple(&name.to_string()), |tuple, field| {
                    tuple.field(&field.debug(syntax))
                })
                .finish()
        }))
    }
}
