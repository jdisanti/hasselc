//
// Copyright 2017 hasselc Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
//

use symbol_table::{self, SymbolRef};
use llir::Location;

pub fn convert_location(frame: SymbolRef, input: &symbol_table::Location) -> Location {
    match *input {
        symbol_table::Location::UndeterminedGlobal => unreachable!(),
        symbol_table::Location::Global(addr) => Location::Global(addr),
        symbol_table::Location::FrameOffset(offset) => Location::FrameOffset(frame, offset),
    }
}
