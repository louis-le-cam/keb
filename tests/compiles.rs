#![feature(random)]

use std::{
    env::temp_dir,
    io::Write,
    process::{Command, Stdio},
    random::random,
};

use keb::{c, semantic, ssa, syntax, token};

fn test_program(source: &str, expected_output: &str) {
    let tokens = token::lex(&source);
    let syntax = syntax::parse(&tokens.kinds);
    let (mut semantic, mut types) = semantic::parse(&source, &tokens.offsets, &syntax);
    semantic::infer_types(&mut semantic, &mut types);
    let ssa = ssa::generate(&source, &tokens.offsets, &semantic, &mut types);
    let c = c::generate(&types, &ssa);

    let program_path = temp_dir().join(format!("keb-test-c-output-{:0>32x}.c", random::<u128>(..)));

    let mut clang = Command::new("clang")
        .args(["-xc", "-std=c23", "-", "-o"])
        .arg(&program_path)
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();

    let stdin = clang.stdin.as_mut().unwrap();
    stdin.write_all(c.as_bytes()).unwrap();
    stdin.flush().unwrap();

    assert!(clang.wait_with_output().unwrap().status.success());

    let program = Command::new(&program_path).output().unwrap();
    assert!(program.status.success());

    let stdout = &String::from_utf8(program.stdout).unwrap();
    assert_eq!(stdout, expected_output);

    std::fs::remove_file(&program_path).unwrap();
}

#[test]
fn addition_function_with_argument_destructuring() {
    let source = r#"
        let add = (a: u32, b: u32) => a + b;

        let main = () => print add (8, 4);
    "#;

    test_program(source, "12\n");
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

    test_program(source, "11\n");
}

#[test]
fn factorial_recursive_if_then_else() {
    let source = r#"
        let fact = (x: u32) => if x then x * (fact x - 1) else 1;
        let main = () => print fact 8
    "#;

    test_program(source, "40320\n");
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

    test_program(source, "Hello world!\n");
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

    test_program(source, "40320\n");
}
