
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
        CompilerError(reason: Box<Error>, compiler_output: ::compiler::CompilerOutput) {
            description("Failed to compile code")
            display("Failed to compile code:\n{}", reason)
        }
    }
}
