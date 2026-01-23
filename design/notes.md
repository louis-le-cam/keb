> Note: We specify `awk` language in the code blocks because the highlighting
> matches somewhat `keb`, the code blocks are not actually `awk`.

# Split between type and memory layout

It would allow complex memory layouts for performant memory usage while keeping
the purest and simplest type declaration.

```awk
# This syntax has nothing particularly thoughtful, it is just for clarifying
# what kind of things the layout declaration could be concerned with.
let value_layout = layout!{
  let value = u32;

  if value == u32::MAX - 2 (
    Value::False
  ) else if value == u32::MAX - 1 (
    Value::True
  ) else if value & 1 << x (
    Value::Integer (i30, value & u30::MAX).to_i30_bitcast()
  ) else if  (
    Value::Index (value & u30::MAX).to_u30()
  )
};

#[layout(value_layout)]
type Value =
 | False
 | True
 | Integer i30
 | Index u30
```

# For derive-like feature, separate the configuration from the struct declaration.

```awk
let serialize_config = (
   name: [serde::rename("full_name")],
   # Should we explicitly mark other fields as using the default?
   # Maybe a `...serde::SerializeConfig::default()`, (which would mean we need mapped type also)
);

#[derive(serde::Serialize(serialize_config))]
type User = struct (
   name: String,
   age: u32,
);
```

# Inferred types with unsufficient constraints could be converted to generics

I'm not sure how feasible that thought is.

Example with a type having no constraint:

```awk
let identity = x => x;
# Would be inferred as (syntax is not definitive):
let identity['a] = (x: 'a) -> 'a => x;
```

Example with a type having a trait constraint:

```awk
let add = (x, y) => x + y;
# Would be inferred as (syntax is not definitive):
let add['x, 'y] where 'x: add['y] = (x: 'x, y: 'y) -> <'x as add>::Output => x + y;
```

Would we treat inferred generics the same way as explicit generics?
```awk
let explicit['a] = x: 'a => x;
let implicit = x => x;

# This one has no problem:
explicit[u32]
# This one doesn't feels right:
implicit[u32]
```

If we allow specifying inferred generics, how do we guarrantee the order if
multiple generics are inferred?

It would be preferable for the order and constraints to be evident to a user of
the function without using analysis tools such as a language server.

## Solutions

### Approach #1

We could require the writer of the function to specify the type parameters with
a syntax saying it infer their constraints, and the name of the type parameter
correspond to the name of the variable (which would require a syntax with a
symbol prefix like `'ident`).

```awk
let identity['x: infer] = x => x;
let add['x: infer, 'y: infer] = (x, y) => x + y;
```

### Approach #2

Or we could make type parameter named such that the caller would specify the
type parameter name corresponding to the parameter.

```awk
let identity = x => x;
let add = (x, y) => x + y;

identity['x: u32]
add['x: u32, 'y: u32]
```

### Thoughts

There is also the problem of type parameter not directly related to exactly one
type parameter.

In the approach #1, we could just require the minimal number of type parameter
parameter sufficient to correctly type the function:

```awk
let either['x] = (x, y) => if random() then x else y;
# 'return = 'x 
# 'y = 'return

# We could also specify
let either['y] = (x, y) => if random() then x else y;
# Or (necessiting `'return` to be a special type parameter)
let either['return] = (x, y) => if random() then x else y;
```

In the approach #2, we could let the user of the function use wathever number of
typed generics necessary for a well typed function.

```awk
let either = (x, y) => if random() then x else y;

# All of these are correct and equivalent:
either['x: u32]
either['y: u32]
either['return: u32]
# If we want to go all in
either['x: u32, 'y: u32, 'return: u32]
```

The advantage of this method might be that in some case the user can specify the
type parameters in a way that is simpler to it.
It would even more ressemble a way of constraining the type of a function
instead of really specifying type parameters.
