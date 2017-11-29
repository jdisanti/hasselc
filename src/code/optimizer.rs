use code::{Code, CodeBlock};
use error;

pub fn optimize_code(code: &[CodeBlock]) -> error::Result<Vec<CodeBlock>> {
    let mut result = Vec::new();
    for code_block in code {
        result.push(optimize_code_block(code_block)?);
    }
    Ok(result)
}

fn optimize_code_block(code_block: &CodeBlock) -> error::Result<CodeBlock> {
    let mut optimized = code_block.clone();
    optimized.body.clear();

    let body = &code_block.body;

    let mut last_branch = 0;
    for i in 0..body.len() {
        if body[i].is_branch() {
            if i != last_branch {
                optimized.body.extend(optimize_run(&body[last_branch..i])?);
            }
            last_branch = i;
        }
    }
    optimized.body.extend(optimize_run(&body[last_branch..body.len()])?);

    Ok(optimized)
}

fn optimize_run(run: &[Code]) -> error::Result<Vec<Code>> {
    let mut result = Vec::new();
    result.extend(run.iter().cloned());

    loop {
        let mut optimized = false;
        optimized = try_redundant_load_accum(&mut result) || optimized;
        if !optimized {
            break;
        }
    }

    Ok(result)
}

// Reduces:
//   Sta X
//   Lda X
// To:
//   Sta X
fn try_redundant_load_accum(run: &mut Vec<Code>) -> bool {
    for i in 0..run.len() {
        if run.len() >= i + 2 {
            let matches = {
                let store_param = match run[i] {
                    Code::Sta(ref p) => p,
                    _ => continue,
                };

                let load_param = match run[i + 1] {
                    Code::Lda(ref p) => p,
                    _ => continue,
                };

                store_param == load_param
            };
            if matches {
                run.remove(i + 1);
                return true;
            }
        }
    }
    false
}
