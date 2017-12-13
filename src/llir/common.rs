use symbol_table::{self, SymbolRef};
use llir::Location;

pub fn convert_location(frame: SymbolRef, input: &symbol_table::Location) -> Location {
    match *input {
        symbol_table::Location::UndeterminedGlobal => unreachable!(),
        symbol_table::Location::Global(addr) => Location::Global(addr),
        symbol_table::Location::FrameOffset(offset) => Location::FrameOffset(frame, offset),
    }
}
