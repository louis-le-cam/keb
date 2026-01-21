use std::{
    io::Write,
    process::{Command, Stdio},
};

use keb::{c, semantic, ssa, syntax, token};

fn test_if_compiles(source: &str) {
    let tokens = token::lex(&source);
    let syntax = syntax::parse(&tokens);
    let (mut semantic, mut types) = semantic::parse(&source, &tokens, &syntax);
    semantic::infer_types(&mut semantic, &mut types);
    let ssa = ssa::generate(&source, &tokens, &semantic, &mut types);
    let c = c::generate(&types, &ssa);

    let mut clang = Command::new("clang")
        .args(["-xc", "-std=c23", "-"])
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();

    let stdin = clang.stdin.as_mut().unwrap();
    stdin.write_all(c.as_bytes()).unwrap();
    stdin.flush().unwrap();

    assert!(clang.wait().unwrap().success());
}

#[test]
fn addition_function_with_argument_destructuring() {
    let source = r#"
        let add = (a: u32, b: u32) => a + b;

        let main = () => print add (8, 4);
    "#;

    test_if_compiles(source);
}

#[test]
fn if_then_else_expression() {
    let source = r#"
        let main = () => (
            let a = false;
            let x = if a then 8 else 3 + 8;
            print x;
        );
    "#;

    test_if_compiles(source);
}

// NOTE: Not yet implemented
#[test]
#[should_panic]
fn print_string() {
    let source = r#"
        let main = (
            let hello_world = "Hello world!";

            print hello_world;
        );
    "#;

    test_if_compiles(source);
}

// NOTE: Not yet implemented
#[test]
#[should_panic]
fn factorial_recursive_match() {
    let source = r#"
        let fact = match {
            0 | 1 => 1,
            n => n * fact n - 1,
        };

        let main = print fact 8;
    "#;

    test_if_compiles(source);
}
