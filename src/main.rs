#![feature(macro_attr)]
#![feature(macro_derive)]

use std::process::Command;

use colored::Colorize;
use keb::{c, semantic, ssa, syntax, token};

fn main() {
    debug_header("SOURCE (colored based on tokens)");
    let source = std::fs::read_to_string("input.keb").unwrap();
    let tokens = token::lex(&source);
    token::debug(&source, &tokens);

    debug_header("SYNTAX");
    let syntax = syntax::parse(&tokens.kinds);
    syntax::debug(&syntax);

    debug_header("SEMANTIC");
    let (mut semantic, mut types) = semantic::parse(&source, &tokens.offsets, &syntax);
    semantic::infer_types(&mut semantic, &mut types);
    semantic::debug(&semantic, &types);

    let ssa = ssa::generate(&source, &tokens.offsets, &semantic, &mut types);

    debug_header("SSA");
    ssa::debug(&types, &ssa);

    let c = c::generate(&types, &ssa);
    std::fs::write("output.c", c).unwrap();

    debug_header("CLANG");
    let clang_exit_status = Command::new("clang")
        .args(["output.c", "-std=c23"])
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    if !clang_exit_status.success() {
        return;
    }

    debug_header("OUTPUT");
    Command::new("./a.out").spawn().unwrap().wait().unwrap();
}

fn debug_header(text: &str) {
    println!(
        "\n{}\n",
        format!("======== {} ========", text.bright_green())
            .bright_yellow()
            .bold()
    );
}
