#![recursion_limit = "1024"]

extern crate lalrpop_util;
extern crate num_traits;

#[macro_use]
extern crate derive_builder;

#[macro_use]
extern crate derive_new;

#[macro_use]
extern crate error_chain;

pub mod symbol_table;
mod types;
mod type_expr;
pub mod code;
pub mod compiler;
pub mod error;
pub mod ir;
pub mod llir;
pub mod parse;
pub mod src_tag;
pub mod src_unit;

pub use compiler::{Compiler, CompilerOptions, CompilerOptionsBuilder, CompilerOutput};
