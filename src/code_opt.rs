use code::{Code, CodeBlock};

pub fn optimize_code(code: &Vec<CodeBlock>) -> Result<Vec<CodeBlock>, ()> {
    let mut result = Vec::new();
    for code_block in code {
        result.push(optimize_code_block(code_block)?);
    }
    Ok(result)
}

fn optimize_code_block(code_block: &CodeBlock) -> Result<CodeBlock, ()> {
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
    optimized
        .body
        .extend(optimize_run(&body[last_branch..body.len()])?);

    Ok(optimized)
}

fn optimize_run(run: &[Code]) -> Result<Vec<Code>, ()> {
    let mut result = Vec::new();
    result.extend(run.iter().cloned());

    try_redundant_load_accum(&mut result);
    try_unchanged_stack_pointer(&mut result);

    Ok(result)
}

fn try_redundant_load_accum(run: &mut Vec<Code>) {
    for i in 0..run.len() {
        if run.len() >= i + 3 {
            let matches = {
                let load_param = match run[i] {
                    Code::Lda(ref p) => p,
                    _ => continue,
                };

                let store_param = match run[i + 1] {
                    Code::Sta(ref p) => p,
                    _ => continue,
                };

                let second_load_param = match run[i + 2] {
                    Code::Lda(ref p) => p,
                    _ => continue,
                };

                load_param != store_param && store_param == second_load_param
            };
            if matches {
                run.remove(i + 2);
            }
        }
    }
}

// Looks through run of instructions, and if the stack pointer is loaded,
// and then loaded again without any changes being made to it, then that
// second load is removed.
fn try_unchanged_stack_pointer(run: &mut Vec<Code>) {
    let mut to_remove = Vec::new();
    for x in 0..(run.len() - 1) {
        if let Code::Ldx(ref first_load) = run[x] {
            for y in (x + 1)..run.len() {
                match run[y] {
                    // TODO: Code::Inx(_) => break,
                    // TODO: Code::Dex(_) => break,
                    Code::Tax(_) => break,
                    Code::Ldx(ref p) => if p == first_load {
                        if !to_remove.contains(&y) {
                            to_remove.insert(0, y);
                        }
                    } else {
                        break;
                    },
                    _ => {}
                }
            }
        }
    }
    for index in to_remove {
        run.remove(index);
    }
}
