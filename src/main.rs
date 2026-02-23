#![feature(macro_attr)]
#![feature(macro_derive)]

use std::process::Command;

use colored::Colorize;
use keb::{
    c_codegen,
    semantic::{self, Types},
    ssa::{self, Ssa},
    syntax, token, x86_asm_codegen,
};

fn main() {
    debug_header("SOURCE (colored based on tokens)");
    let source = std::fs::read_to_string("input.keb").unwrap();

    let (types, ssa) = compile_to_ssa(&source);

    // run_ssa_with_c_codegen(&types, &ssa);
    run_ssa_with_x86_asm_codegen(&types, &ssa);
}

fn compile_to_ssa(source: &str) -> (Types, Ssa) {
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

    (types, ssa)
}

fn run_ssa_with_c_codegen(types: &Types, ssa: &Ssa) {
    let c_code = c_codegen::generate(&types, &ssa);
    std::fs::write("output.c", c_code).unwrap();

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

fn run_ssa_with_x86_asm_codegen(types: &Types, ssa: &Ssa) {
    let x86_asm = x86_asm_codegen::generate(&types, &ssa);
    std::fs::write("output.s", x86_asm).unwrap();

    debug_header("GCC");
    let clang_exit_status = Command::new("gcc")
        .args([
            "-O0",
            "output.s",
            "-m64",
            "-Xlinker",
            "-z",
            "-Xlinker",
            "noexecstack",
        ])
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    if !clang_exit_status.success() {
        return;
    }

    debug_header("OUTPUT");
    let exit_status = Command::new("./a.out").spawn().unwrap().wait().unwrap();

    println!("Program exited with {exit_status}");
}

fn debug_header(text: &str) {
    println!(
        "\n{}\n",
        format!("======== {} ========", text.bright_green())
            .bright_yellow()
            .bold()
    );
}
