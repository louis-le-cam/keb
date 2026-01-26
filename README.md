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
let fact = x: u32 => if x then x * (fact x - 1) else 1;

let main = () => print fact 8
# output: `40320`
```

## Recursive factorial with pattern matching (not yet implemented)

```keb
let fact = match {
    0 | 1 => 1,
    n => n * fact n - 1,
};

let main = print fact 8;
# output: `40320`
```
