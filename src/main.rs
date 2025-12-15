#![feature(macro_attr)]
#![feature(macro_derive)]

use colored::Colorize;

mod c;
mod key_vec;
mod semantic;
mod ssa;
mod syntax;
mod token;

fn main() {
    let source = std::fs::read_to_string("input.keb").unwrap();
    debug_header("SOURCE");
    println!("{source}");

    let tokens = token::lex(&source);
    let syntax = syntax::parse(&tokens);

    debug_header("SYNTAX");
    syntax::debug(&syntax);

    let (mut semantic, mut types) = semantic::parse(&source, &tokens, &syntax);

    debug_header("SEMANTIC");
    semantic::debug(&semantic, &types);

    semantic::infer_types(&mut semantic, &mut types);

    debug_header("TYPED SEMANTIC");
    semantic::debug(&semantic, &types);

    let ssa = ssa::generate(&source, &tokens, &semantic, &mut types);
    let c = c::generate(&types, &ssa);

    debug_header("SSA");
    ssa::debug(&types, &ssa);

    std::fs::write("output.c", c).unwrap();

    debug_header("OUTPUT");
    std::process::Command::new("clang")
        .arg("output.c")
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    std::process::Command::new("./a.out")
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
}

fn debug_header(text: &str) {
    println!(
        "\n{}\n",
        format!("======== {} ========", text.bright_green())
            .bright_yellow()
            .bold()
    );
}
