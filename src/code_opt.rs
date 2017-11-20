use code::{Code, CodeBlock, Global, Parameter};
use register::DATA_STACK_POINTER_LOCATION;
use error;

pub fn optimize_code(code: &Vec<CodeBlock>) -> error::Result<Vec<CodeBlock>> {
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
    optimized
        .body
        .extend(optimize_run(&body[last_branch..body.len()])?);

    Ok(optimized)
}

fn optimize_run(run: &[Code]) -> error::Result<Vec<Code>> {
    let mut result = Vec::new();
    result.extend(run.iter().cloned());

    loop {
        let mut optimized = false;
        optimized = try_redundant_load_accum(&mut result) || optimized;
        optimized = try_unchanged_stack_pointer(&mut result) || optimized;
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

// Looks through run of instructions, and if the stack pointer is loaded,
// and then loaded again without any changes being made to it, then that
// second load is removed.
fn try_unchanged_stack_pointer(run: &mut Vec<Code>) -> bool {
    if run.is_empty() {
        return false
    }

    let mut to_remove = Vec::new();
    for x in 0..(run.len() - 1) {
        if let Code::Ldx(ref first_load) = run[x] {
            for y in (x + 1)..run.len() {
                match run[y] {
                    // TODO: Code::Inx(_) => break,
                    // TODO: Code::Dex(_) => break,
                    // If we overwrite the stack pointer in memory, then we need to reload it
                    Code::Sta(ref p) => match *p {
                        Parameter::Absolute(ref gbl) => match *gbl {
                            Global::Resolved(val) => if val == DATA_STACK_POINTER_LOCATION {
                                break;
                            },
                            _ => {}
                        },
                        Parameter::ZeroPage(offset) => if offset as u16 == DATA_STACK_POINTER_LOCATION {
                            break;
                        },
                        _ => {}
                    },
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
    for index in &to_remove {
        run.remove(*index);
    }
    !to_remove.is_empty()
}
