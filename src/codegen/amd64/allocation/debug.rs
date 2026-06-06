use colored::Colorize;

use crate::{
    codegen::amd64::allocation::Allocations,
    semantic::Types,
    ssa::{BlockData, Ssa},
};

pub fn debug(types: &Types, ssa: &Ssa, allocations: &Allocations) {
    for (block, block_data) in ssa.blocks.entries() {
        println!(
            "{} {} -> {}",
            block.debug(types, ssa),
            format!("{:?}", allocations.arguments[block]).bright_cyan(),
            format!("{:?}", allocations.returns[block]).bright_cyan(),
        );

        let insts = match block_data {
            BlockData::ExternFunction { .. } => {
                println!();
                continue;
            }
            BlockData::Function { insts, .. } | BlockData::Block { insts, .. } => insts,
        };

        for inst in insts {
            println!(
                "  {inst} = {} {}",
                inst.debug(ssa),
                format!("{:?}", allocations.instructions[*inst].allocation).bright_cyan(),
            );
        }

        println!();
    }

    for (const_, const_data) in ssa.consts.entries() {
        println!("{const_} = {}", const_data.debug());
    }
}
