use std::sync::Arc;
use src_tag::SrcTag;

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
        CompilerError(row_col: (usize, usize), reason: Box<Error>, compiler_output: ::compiler::CompilerOutput) {
            description("Failed to compile code")
            display("{}:{}: {}", row_col.0, row_col.1, reason)
        }
        DuplicateSymbol(src_tag: SrcTag, name: Arc<String>) {
            description("Duplicate symbol")
            display("Duplicate symbol \"{}\"", name)
        }
        SymbolNotFound(src_tag: SrcTag, name: Arc<String>) {
            description("Symbol not found")
            display("Symbol not found: \"{}\"", name)
        }
        OrgOutOfRange(src_tag: SrcTag) {
            description("Org out of  range")
            display("The org address must be between 0x0200 and 0xFFFF")
        }
    }
}

pub fn to_compiler_error(program: &str, err: Error, compiler_output: ::compiler::CompilerOutput) -> Error {
    let row_col = match err.0 {
        ErrorKind::DuplicateSymbol(ref src_tag, ..) => src_tag.row_col(program),
        ErrorKind::SymbolNotFound(ref src_tag, ..) => src_tag.row_col(program),
        _ => panic!("Unsupported compiler error type: {:#?}", err),
    };
    ErrorKind::CompilerError(row_col, Box::new(err), compiler_output).into()
}
