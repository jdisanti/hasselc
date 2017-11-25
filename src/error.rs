use std::sync::Arc;
use compiler::CompilerOutput;
use src_tag::SrcTag;
use src_unit::SrcUnits;

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    links {
    }

    foreign_links {
        FmtError(::std::fmt::Error);
    }

    errors {
        ParseError(errors: Vec<String>) {
            description("Failed to parse code")
            display("Failed to parse code:\n{}", errors.join("\n"))
        }
        CompilerError(unit_name: String, row_col: (usize, usize), reason: Box<Error>, compiler_output: CompilerOutput) {
            description("Failed to compile code")
            display("{}:{}:{}: {}", unit_name, row_col.0, row_col.1, reason)
        }

        //
        // SrcTagged Compiler Errors
        //
        ConstCantBeVoid(src_tag: SrcTag) {
            description("Constant can't be void")
            display("Constant can't be void")
        }
        ConstEvaluationFailed(src_tag: SrcTag) {
            description("Constant evaluation failed")
            display("Constant evaluation failed")
        }
        DuplicateSymbol(src_tag: SrcTag, name: Arc<String>) {
            description("Duplicate symbol")
            display("Duplicate symbol \"{}\"", name)
        }
        ExpectedNArgumentsGotM(src_tag: SrcTag, function: Arc<String>, expected: usize, actual: usize) {
            description("Expected N args, got M")
            display("In function call to \"{}\", expected {} arguments, got {}", function, expected, actual)
        }
        InvalidLeftValue(src_tag: SrcTag) {
            description("Invalid left value")
            display("Cannot assign into expression")
        }
        MustReturnAValue(src_tag: SrcTag) {
            description("Must return a value")
            display("Must return a value")
        }
        OrgOutOfRange(src_tag: SrcTag) {
            description("Org out of range")
            display("The org address must be between 0x0200 and 0xFFFF")
        }
        OutOfBounds(src_tag: SrcTag, value: isize, min: isize, max: isize) {
            description("Integer out of bounds")
            display("Integer value {} must be between {} and {}", value, min, max)
        }
        SymbolNotFound(src_tag: SrcTag, name: Arc<String>) {
            description("Symbol not found")
            display("Symbol not found: \"{}\"", name)
        }
        TypeError(src_tag: SrcTag, expected: ::types::Type, actual: ::types::Type) {
            description("Type error")
            display("Expected type {:?}, found {:?}", expected, actual)
        }
    }
}

pub fn to_compiler_error(src_units: &SrcUnits, err: Error, compiler_output: CompilerOutput) -> Error {
    use self::ErrorKind::*;
    let (name, row_col) = match err.0 {
        ConstCantBeVoid(ref src_tag, ..)
        | ConstEvaluationFailed(ref src_tag, ..)
        | DuplicateSymbol(ref src_tag, ..)
        | ExpectedNArgumentsGotM(ref src_tag, ..)
        | InvalidLeftValue(ref src_tag, ..)
        | MustReturnAValue(ref src_tag, ..)
        | OrgOutOfRange(ref src_tag, ..)
        | OutOfBounds(ref src_tag, ..)
        | SymbolNotFound(ref src_tag, ..)
        | TypeError(ref src_tag, ..) => (
            src_units.name(src_tag.unit).clone(),
            src_tag.row_col(src_units.source(src_tag.unit)),
        ),
        _ => panic!("Unsupported compiler error type: {:#?}", err),
    };
    CompilerError(name, row_col, Box::new(err), compiler_output).into()
}
