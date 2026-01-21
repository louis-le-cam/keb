mod debug;
mod parser;
mod syn;

pub use self::{
    debug::debug,
    parser::parse,
    syn::{ROOT_SYN, StringSegment, Syn, SynData, SynSentinel, Syntax},
};
