use std::{process::Command, time::Instant};

use colored::Colorize;
use keb::{
    amd64_asm_codegen, c_codegen,
    semantic::{self, Types},
    ssa::{self, Ssa},
    syntax, token,
};

fn main() {
    let source = std::fs::read_to_string("input.keb").unwrap();

    let (types, ssa) = compile_to_ssa(&source);

    // run_ssa_with_c_codegen(&types, &ssa);
    run_ssa_with_amd64_asm_codegen(&types, &ssa);
}

fn compile_to_ssa(source: &str) -> (Types, Ssa) {
    let start = Instant::now();
    let tokens = token::lex(&source);
    debug_header_duration("SOURCE (colored based on tokens)", start);
    token::debug(&source, &tokens);

    let start = Instant::now();
    let syntax = syntax::parse(&tokens.kinds);
    debug_header_duration("SYNTAX", start);
    syntax::debug(&syntax);

    let start = Instant::now();
    let (mut semantic, mut types) = semantic::parse(&source, &tokens.offsets, &syntax);
    semantic::infer_types(&mut semantic, &mut types);
    debug_header_duration("SEMANTIC", start);
    semantic::debug(&semantic, &types);

    let start = Instant::now();
    let ssa = ssa::generate(&source, &tokens.offsets, &semantic, &mut types);
    debug_header_duration("SSA", start);
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

fn run_ssa_with_amd64_asm_codegen(types: &Types, ssa: &Ssa) {
    let asm = amd64_asm_codegen::generate(&types, &ssa);
    std::fs::write("output.s", asm).unwrap();

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

fn debug_header_duration(text: &str, start: Instant) {
    let duration = Instant::now().duration_since(start);
    println!(
        "\n{}\n",
        format!(
            "======== {} {} ========",
            text.bright_green(),
            format!("{duration:?}").yellow().normal()
        )
        .bright_yellow()
        .bold()
    )
}
