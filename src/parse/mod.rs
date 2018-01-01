//
// Copyright 2017 hasselc Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
//

pub mod ast;

#[cfg_attr(rustfmt, rustfmt_skip)]
mod grammar;

fn unescape_string(s: &str) -> String {
    s.replace("\\\"", "\"").replace("\\n", "\n").replace("\\t", "\t").replace(
        "\\r",
        "\r",
    )
}
