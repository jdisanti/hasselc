#![recursion_limit = "1024"]

extern crate lalrpop_util;
extern crate num_traits;

#[macro_use]
extern crate derive_new;

#[macro_use]
extern crate error_chain;

mod symbol_table;
mod types;
pub mod code;
pub mod compiler;
pub mod error;
pub mod ir;
pub mod llir;
pub mod parse;
pub mod src_tag;

pub use compiler::CompilerOutput;
pub use compiler::compile;
