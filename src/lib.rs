#![recursion_limit = "1024"]

extern crate lalrpop_util;

#[macro_use]
extern crate error_chain;

#[cfg_attr(rustfmt, rustfmt_skip)]
mod grammar;

pub mod ast;
pub mod ir;
mod ir_gen;
pub mod llir;
mod llir_gen;
mod llir_opt;
pub mod code;
mod code_gen;
mod code_opt;
pub mod compiler;
pub mod error;

pub use compiler::CompilerOutput;
pub use compiler::compile;
