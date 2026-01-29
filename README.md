A toy compiled, statically typed, functionnal programming language.

# Examples

## Function argument pattern matching

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
consumes some of the precedent stages outputs and produces on or multiple
outputs:

- lexer/tokenizer: source => tokens
- syntax parser: tokens => syns(syntax nodes)
- semantic parser: source, tokens, syns => sems(semantic nodes), types
- type inference: sems, types => *sems*, *types*
- ssa generation: source, tokens, sems, types => blocks, instructions, consts, *types*
- c codegen: sems, types => c source code

All theses steps are explicitly written down in `src/main.rs`
