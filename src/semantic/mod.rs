mod debug;
mod parser;
mod sem;
mod r#type;
mod type_inference;

pub use self::{
    debug::{debug, debug_type},
    parser::parse,
    sem::{ROOT_SEM, Sem, SemKind, SemKinds, SemSentinel, SemTypes, Semantic},
    r#type::{Type, TypeData, TypeSentinel, Types, combine_types, types_equals},
    type_inference::infer_types,
};
