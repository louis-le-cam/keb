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
