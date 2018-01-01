//
// Copyright 2017 hasselc Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
//

#![recursion_limit = "1024"]

extern crate lalrpop_util;

#[macro_use]
extern crate derive_builder;

#[macro_use]
extern crate derive_new;

#[macro_use]
extern crate error_chain;

extern crate hassel_asm;

pub mod symbol_table;
mod base_type;
pub mod code;
pub mod compiler;
pub mod error;
pub mod ir;
pub mod llir;
pub mod parse;
pub mod src_tag;
pub mod src_unit;

pub use compiler::{Compiler, CompilerOptions, CompilerOptionsBuilder, CompilerOutput};
