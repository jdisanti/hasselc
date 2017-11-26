pub mod ast;

#[cfg_attr(rustfmt, rustfmt_skip)]
mod grammar;

fn unescape_string(s: &str) -> String {
    s.replace("\\\"", "\"")
        .replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\r", "\r")
}
