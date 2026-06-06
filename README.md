A toy compiled, statically typed, functionnal programming language.

# Examples

## Function argument pattern matching

Function do only take one argument, but this argument can be a tuple/struct.
The definition of a function's argument is a pattern which can destructure the
single argument into multiple.

```keb
let add = (a: u32, b: u32) => a + b;

let main = () => print add (8, 4);
# output: `12`
```

## Recursive factorial

```keb
let fact = match {
    0 => 1,
    n => n * (fact n - 1),
};

let main = print fact 8;
# output: `40320`
```

# Architecture

The compiler follows a pretty simple multi-stage architecture, each stage
consumes some of the precedent stages outputs and produces one or multiple
outputs.

Here's a pseudo-code of the compilation process, this is pretty much the code in
`src/main.rs` without debug noise:

```rust
fn compile_ssa(source: &str) -> (Types, Ssa) {
    let tokens = token::lex(&source);

    let syntax = syntax::parse(&tokens.kinds);

    let (mut semantic, mut types) = semantic::parse(&source, &tokens.offsets, &syntax);
    semantic::infer_types(&mut semantic, &mut types);

    let ssa = ssa::generate(&source, &tokens.offsets, &semantic, &mut types);

    (types, ssa)
}

fn compile_to_amd64(types: &Types, ssa: &Ssa) -> String {
    let allocations = codegen::amd64::allocation::allocate(&types, &ssa);

    codegen::amd64::asm::generate(&types, &ssa, &allocations)
}

fn compile_to_c(types: &Types, ssa: &Ssa) -> String {
    codegen::c::generate(&types, &ssa)
}
```
