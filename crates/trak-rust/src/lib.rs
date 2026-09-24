//! Dependency-free analysis of Rust source files: symbol and function/call-site
//! extraction sufficient to answer "who calls X", "what does X call", path and
//! impact queries without pulling in a full compiler front-end.

mod parse;

pub use parse::{functions, symbols};
