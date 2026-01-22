use colored::Colorize as _;

use crate::{
    key_vec::Val,
    semantic::{Type, TypeData, TypeSentinel, Types},
};

use super::*;

pub fn debug(types: &Types, ssa: &Ssa) {
    for (block, block_data) in ssa.blocks.entries() {
        let insts = match block_data {
            BlockData::ExternFunction { name, arg, ret } => {
                print!(
                    "{} {} {} ",
                    format!("@{}", block.as_u32()).bright_yellow(),
                    "extern".bright_red().bold(),
                    name.bright_yellow(),
                );
                debug_type(types, *arg);
                print!(" -> ");
                debug_type(types, *ret);
                println!("\n");
                continue;
            }
            BlockData::Function {
                name,
                arg,
                ret,
                insts,
            } => {
                print!(
                    "{} {} ",
                    format!("@{}", block.as_u32()).bright_yellow(),
                    name.bright_yellow()
                );
                debug_type(types, *arg);
                print!(" -> ");
                debug_type(types, *ret);
                println!();
                insts
            }
            // Maybe not print block here?
            BlockData::Block { arg, insts } => {
                print!("{}", format!("@{} ", block.as_u32()).bright_yellow());
                debug_type(types, *arg);
                println!();
                insts
            }
        };

        for inst in insts {
            print!("  {} = ", format!("%{}", inst.as_u32()).bright_green());

            match &ssa.insts[*inst] {
                InstData::Field(expr, field) => {
                    print!("{} ", "field".bright_red().bold());
                    debug_expr(expr);
                    print!(", {field}");
                }
                InstData::Record(fields, _) => {
                    print!("{} ", "record".bright_red().bold());
                    for (i, field) in fields.iter().enumerate() {
                        if i != 0 {
                            print!(", ");
                        }

                        debug_expr(field);
                    }
                }
                InstData::Add(lhs, rhs) => {
                    print!("{} ", "add".bright_red().bold());
                    debug_expr(lhs);
                    print!(", ");
                    debug_expr(rhs);
                }
                InstData::Call { function, argument } => {
                    print!(
                        "{} {}, ",
                        "call".bright_red().bold(),
                        format!("@{}", function.as_u32()).bright_yellow()
                    );
                    debug_expr(argument);
                }
                InstData::Jump { block, argument } => {
                    print!(
                        "{} {}",
                        "jump".bright_red().bold(),
                        format!("@{}", block.as_u32()).bright_yellow()
                    );
                    print!(", ");
                    debug_expr(argument);
                }
                InstData::JumpCondition {
                    condition,
                    then,
                    else_,
                } => {
                    print!(
                        "{} {} {} ",
                        "jump".bright_red().bold(),
                        format!("@{}", then.as_u32()).bright_yellow(),
                        "if".bright_red(),
                    );
                    debug_expr(condition);
                    print!(
                        " {} {}",
                        "else".bright_red(),
                        format!("@{}", else_.as_u32()).bright_yellow()
                    );
                }
                InstData::Return(value) => {
                    print!("{} ", "return".bright_red().bold());
                    debug_expr(value);
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

fn debug_type(types: &Types, type_: Type) {
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
            print!("{}", text.bright_blue())
        }
        Val::Value(type_data) => match type_data {
            TypeData::Function { .. } => todo!(),
            TypeData::Product { fields } => {
                print!("(");
                for (i, (_, field)) in fields.iter().enumerate() {
                    if i != 0 {
                        print!(", ");
                    }

                    debug_type(types, *field);
                }
                print!(")");
            }
        },
    };
}

fn debug_expr(expr: &Expr) {
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

            print!("{}", format!("{text}").bright_magenta())
        }
        Expr::Inst(inst) => print!("{}", format!("%{}", inst.as_u32()).bright_green()),
        Expr::BlockArg(block) => {
            print!("{}", format!("param(@{})", block.as_u32()).bright_yellow())
        }
    }
}
