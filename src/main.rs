#![feature(macro_attr)]
#![feature(macro_derive)]

use std::process::Command;

use colored::Colorize;

mod c;
mod key_vec;
mod semantic;
mod ssa;
mod syntax;
mod token;

fn main() {
    debug_header("SOURCE");
    let source = std::fs::read_to_string("input.keb").unwrap();
    println!("{source}");

    debug_header("SYNTAX");
    let tokens = token::lex(&source);
    let syntax = syntax::parse(&tokens);
    syntax::debug(&syntax);

    debug_header("SEMANTIC");
    let (mut semantic, mut types) = semantic::parse(&source, &tokens, &syntax);
    semantic::debug(&semantic, &types);

    debug_header("TYPED SEMANTIC");
    semantic::infer_types(&mut semantic, &mut types);
    semantic::debug(&semantic, &types);

    let ssa = ssa::generate(&source, &tokens, &semantic, &mut types);

    debug_header("SSA");
    ssa::debug(&types, &ssa);

    let c = c::generate(&types, &ssa);
    std::fs::write("output.c", c).unwrap();

    debug_header("CLANG");
    let clang_exit_status = Command::new("clang")
        .arg("output.c")
        .arg("-std=c23")
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
