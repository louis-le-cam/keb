use colored::Colorize as _;

use crate::{
    key_vec::Val,
    semantic::{Type, TypeData, Types},
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
            // Maybe not print block here ?
            BlockData::Block { arg, insts } => {
                print!("@{} ", block.as_u32());
                debug_type(types, *arg);
                println!();
                insts
            }
        };

        for inst in insts {
            print!("  {} = ", format!("%{}", inst.as_u32()).bright_green());

            match ssa.insts.get(*inst) {
                Val::None => panic!(),
                Val::Value(inst_data) => match inst_data {
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
                    InstData::Jump {
                        block,
                        condition,
                        argument,
                    } => {
                        print!(
                            "{} {} if ",
                            "jump".bright_red().bold(),
                            format!("@{}", block.as_u32()).bright_yellow()
                        );
                        debug_expr(condition);
                        print!(", ");
                        debug_expr(argument);
                    }
                    InstData::Return(value) => {
                        print!("{} ", "return".bright_red().bold());
                        debug_expr(value);
                    }
                },
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
        Val::Sentinel(sentinel) => print!("{sentinel:?}"),
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
        Expr::Const(const_) => print!("{}", format!("${}", const_.as_u32()).bright_magenta()),
        Expr::Inst(inst) => print!("{}", format!("%{}", inst.as_u32()).bright_green()),
        Expr::BlockArg(block) => {
            print!("{}", format!("param(@{})", block.as_u32()).bright_yellow())
        }
    }
}
