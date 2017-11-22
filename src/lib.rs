#![recursion_limit = "1024"]

extern crate lalrpop_util;

#[macro_use]
extern crate derive_new;

#[macro_use]
extern crate error_chain;

#[cfg_attr(rustfmt, rustfmt_skip)]
mod grammar;

mod code_gen;
mod code_opt;
mod ir_gen;
mod llir_gen;
mod llir_opt;
mod register;
mod symbol_table;
pub mod ast;
pub mod code;
pub mod compiler;
pub mod error;
pub mod ir;
pub mod llir;
pub mod src_tag;

pub use compiler::CompilerOutput;
pub use compiler::compile;
