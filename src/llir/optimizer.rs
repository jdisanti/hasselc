//
// Copyright 2017 hasselc Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
//

use llir::FrameBlock;
use error;

pub fn optimize_llir(llir: &[FrameBlock]) -> error::Result<Vec<FrameBlock>> {
    let mut optimized = Vec::new();
    // TODO: Do actual optimizations when they are needed
    optimized.extend(llir.iter().cloned());
    Ok(optimized)
}
