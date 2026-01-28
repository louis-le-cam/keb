use colored::Colorize as _;

use crate::semantic::{Types, debug_type};

use super::*;

pub fn debug(types: &Types, ssa: &Ssa) {
    for (block, block_data) in ssa.blocks.entries() {
        let insts = match block_data {
            BlockData::ExternFunction { name, arg, ret } => {
                println!(
                    "{} {} {} {} -> {}",
                    format!("@{}", block.as_u32()).bright_yellow(),
                    "extern".bright_red().bold(),
                    name.bright_yellow(),
                    debug_type(types, *arg),
                    debug_type(types, *ret),
                );
                continue;
            }
            BlockData::Function {
                name,
                arg,
                ret,
                insts,
            } => {
                println!(
                    "{} {} {} -> {}",
                    format!("@{}", block.as_u32()).bright_yellow(),
                    name.bright_yellow(),
                    debug_type(types, *arg),
                    debug_type(types, *ret),
                );
                insts
            }
            // Maybe not print block here?
            BlockData::Block { arg, insts } => {
                println!(
                    "{} {}",
                    format!("@{}", block.as_u32()).bright_yellow(),
                    debug_type(types, *arg),
                );
                insts
            }
        };

        for inst in insts {
            print!("  {} = ", format!("%{}", inst.as_u32()).bright_green());

            match &ssa.insts[*inst] {
                InstData::Field(expr, field) => print!(
                    "{} {}, {field}",
                    "field".bright_red().bold(),
                    debug_expr(expr),
                ),
                InstData::Record(fields, _) => {
                    print!("{} ", "record".bright_red().bold());
                    for (i, field) in fields.iter().enumerate() {
                        if i != 0 {
                            print!(", ");
                        }

                        print!("{}", debug_expr(field));
                    }
                }
                InstData::Equal(lhs, rhs) => print!(
                    "{} {}, {}",
                    "equal".bright_red().bold(),
                    debug_expr(lhs),
                    debug_expr(rhs),
                ),
                InstData::Add(lhs, rhs) => print!(
                    "{} {}, {}",
                    "add".bright_red().bold(),
                    debug_expr(lhs),
                    debug_expr(rhs),
                ),
                InstData::Sub(lhs, rhs) => print!(
                    "{} {}, {}",
                    "sub".bright_red().bold(),
                    debug_expr(lhs),
                    debug_expr(rhs),
                ),
                InstData::Mul(lhs, rhs) => print!(
                    "{} {}, {}",
                    "mul".bright_red().bold(),
                    debug_expr(lhs),
                    debug_expr(rhs),
                ),
                InstData::Div(lhs, rhs) => print!(
                    "{} {}, {}",
                    "div".bright_red().bold(),
                    debug_expr(lhs),
                    debug_expr(rhs),
                ),
                InstData::Call { function, argument } => print!(
                    "{} {}, {}",
                    "call".bright_red().bold(),
                    format!("@{}", function.as_u32()).bright_yellow(),
                    debug_expr(argument),
                ),
                InstData::Jump { block, argument } => print!(
                    "{} {}, {}",
                    "jump".bright_red().bold(),
                    format!("@{}", block.as_u32()).bright_yellow(),
                    debug_expr(argument),
                ),
                InstData::JumpCondition {
                    condition,
                    then,
                    else_,
                } => print!(
                    "{} {} {} {} {} {}",
                    "jump".bright_red().bold(),
                    format!("@{}", then.as_u32()).bright_yellow(),
                    "if".bright_red(),
                    debug_expr(condition),
                    "else".bright_red(),
                    format!("@{}", else_.as_u32()).bright_yellow(),
                ),
                InstData::Return(value) => {
                    print!("{} {}", "return".bright_red().bold(), debug_expr(value))
                }
            }
            println!("{}", ";".white());
        }

        println!()
    }

    for (const_, const_data) in ssa.consts.entries() {
        print!("{} = ", format!("${}", const_.as_u32()).bright_magenta());
        match const_data {
            ConstData::Uint32(value) => print!("{}", format!("{value}_u32").bright_magenta()),
            ConstData::Product(fields, _) => {
                print!("(");
                for (i, field) in fields.iter().enumerate() {
                    if i != 0 {
                        print!(", ");
                    }

                    print!("{}", format!("${field:?}").bright_magenta());
                }
                print!(")");
            }
        }
        println!("{}", ";".white());
    }
}

fn debug_expr(expr: &Expr) -> String {
    match expr {
        Expr::Const(const_) => {
            let text = match const_.sentinel() {
                Some(sentinel) => match sentinel {
                    ConstSentinel::Unit => "()",
                    ConstSentinel::False => "false",
                    ConstSentinel::True => "true",
                },
                None => &format!("${}", const_.as_u32()).to_string(),
            };

            text.bright_magenta().to_string()
        }
        Expr::Inst(inst) => format!("%{}", inst.as_u32()).bright_green().to_string(),
        Expr::BlockArg(block) => format!("param(@{})", block.as_u32())
            .bright_yellow()
            .to_string(),
    }
}
