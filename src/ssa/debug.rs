use std::fmt::Display;

use colored::Colorize as _;

use crate::semantic::Types;

use super::*;

pub fn debug(types: &Types, ssa: &Ssa) {
    for (block, block_data) in ssa.blocks.entries() {
        println!("{}", block.debug(types, ssa));

        let insts = match block_data {
            BlockData::ExternFunction { .. } => {
                println!();
                continue;
            }
            BlockData::Function { insts, .. } | BlockData::Block { insts, .. } => insts,
        };

        for inst in insts {
            println!("  {inst} = {}", inst.debug(ssa));
        }

        println!();
    }

    for (const_, const_data) in ssa.consts.entries() {
        println!("{const_} = {}", const_data.debug());
    }
}

impl Display for Inst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("%{}", self.as_u32()).bright_green())
    }
}

impl Inst {
    pub fn debug(self, ssa: &Ssa) -> impl Display {
        std::fmt::from_fn(move |f| match &ssa.insts[self] {
            InstData::Field(expr, field) => write!(
                f,
                "{} {}, {field}",
                "field".bright_red().bold(),
                expr.debug(),
            ),
            InstData::Record(fields, _) => {
                write!(f, "{} ", "record".bright_red().bold())?;
                for (i, field) in fields.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }

                    write!(f, "{}", field.debug())?;
                }

                Ok(())
            }
            InstData::Equal(lhs, rhs) => write!(
                f,
                "{} {}, {}",
                "equal".bright_red().bold(),
                lhs.debug(),
                rhs.debug(),
            ),
            InstData::Add(lhs, rhs) => write!(
                f,
                "{} {}, {}",
                "add".bright_red().bold(),
                lhs.debug(),
                rhs.debug(),
            ),
            InstData::Sub(lhs, rhs) => write!(
                f,
                "{} {}, {}",
                "sub".bright_red().bold(),
                lhs.debug(),
                rhs.debug(),
            ),
            InstData::Mul(lhs, rhs) => write!(
                f,
                "{} {}, {}",
                "mul".bright_red().bold(),
                lhs.debug(),
                rhs.debug(),
            ),
            InstData::Div(lhs, rhs) => write!(
                f,
                "{} {}, {}",
                "div".bright_red().bold(),
                lhs.debug(),
                rhs.debug(),
            ),
            InstData::Call { function, argument } => write!(
                f,
                "{} {}, {}",
                "call".bright_red().bold(),
                format!("@{}", function.as_u32()).bright_yellow(),
                argument.debug(),
            ),
            InstData::Jump { block, argument } => write!(
                f,
                "{} {}, {}",
                "jump".bright_red().bold(),
                format!("@{}", block.as_u32()).bright_yellow(),
                argument.debug(),
            ),
            InstData::JumpCondition {
                condition,
                then,
                else_,
            } => write!(
                f,
                "{} {} {} {} {} {}",
                "jump".bright_red().bold(),
                format!("@{}", then.as_u32()).bright_yellow(),
                "if".bright_red(),
                condition.debug(),
                "else".bright_red(),
                format!("@{}", else_.as_u32()).bright_yellow(),
            ),
            InstData::Return(value) => {
                write!(f, "{} {}", "return".bright_red().bold(), value.debug())
            }
        })
    }
}

impl Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("@{}", self.as_u32()).bright_yellow())
    }
}

impl Block {
    pub fn debug(self, types: &Types, ssa: &Ssa) -> impl Display {
        std::fmt::from_fn(move |f| match &ssa.blocks[self] {
            BlockData::ExternFunction { name, arg, ret } => write!(
                f,
                "{self} {} {} {} -> {}",
                "extern".bright_red().bold(),
                name.bright_yellow(),
                arg.debug(types),
                ret.debug(types),
            ),
            BlockData::Function {
                name,
                arg,
                ret,
                insts: _,
            } => write!(
                f,
                "{self} {} {} -> {}",
                name.bright_yellow(),
                arg.debug(types),
                ret.debug(types),
            ),
            BlockData::Block { arg, insts: _ } => {
                write!(f, "{self} {}", arg.debug(types))
            }
        })
    }
}

impl Display for Const {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("${}", self.as_u32()).bright_magenta())
    }
}

impl ConstData {
    pub fn debug(&self) -> impl Display {
        std::fmt::from_fn(move |f| match self {
            ConstData::Uint32(value) => write!(f, "{}", format!("{value}_u32").bright_magenta()),
            ConstData::Product(fields, _) => {
                write!(f, "(")?;
                for (i, field) in fields.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }

                    write!(f, "{}", format!("${field:?}").bright_magenta())?;
                }
                write!(f, ")")
            }
        })
    }
}

impl Expr {
    pub fn debug(self) -> impl Display {
        std::fmt::from_fn(move |f| match self {
            Expr::Const(const_) => {
                let text = match const_.sentinel() {
                    Some(sentinel) => match sentinel {
                        ConstSentinel::Unit => "()",
                        ConstSentinel::False => "false",
                        ConstSentinel::True => "true",
                    },
                    None => &format!("${}", const_.as_u32()).to_string(),
                };

                write!(f, "{}", text.bright_magenta().to_string())
            }
            Expr::Inst(inst) => inst.fmt(f),
            Expr::BlockArg(block) => write!(
                f,
                "{}",
                format!("param({block})").bright_yellow().to_string()
            ),
        })
    }
}
