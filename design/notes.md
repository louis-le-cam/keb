# Split between type and memory layout

It would allow complex memory layouts for performant memory usage while keeping
the purest and simplest type declaration.

```keb
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

```keb
let serialize_config = (
   name: [serde::rename("full_name")],
   # Should we explicitly mark other fields as using the default ?
   # Maybe a `...serde::SerializeConfig::default()`, (which would mean we need mapped type also)
);

#[derive(serde::Serialize(serialize_config))]
type User = struct (
   name: String,
   age: u32,
);
```
