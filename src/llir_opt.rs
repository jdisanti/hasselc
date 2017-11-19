use llir::{BinaryOpData, Block, Statement, Value};
use error;

pub fn optimize_llir(llir: &Vec<Block>) -> error::Result<Vec<Block>> {
    let mut result = Vec::new();
    for block in llir {
        result.push(optimize_llir_block(block)?);
    }
    Ok(result)
}

fn optimize_llir_block(llir: &Block) -> error::Result<Block> {
    let mut optimized = llir.clone();
    optimized.statements.clear();

    let statements = &llir.statements;

    let mut last_branch = 0;
    for i in 0..statements.len() {
        if statements[i].is_branch() {
            if i != last_branch {
                optimized
                    .statements
                    .extend(optimize_run(&statements[last_branch..i], statements)?);
            }
            last_branch = i;
        }
    }
    optimized.statements.extend(optimize_run(
        &statements[last_branch..statements.len()],
        statements,
    )?);

    Ok(optimized)
}

fn optimize_run(run: &[Statement], full_block: &Vec<Statement>) -> error::Result<Vec<Statement>> {
    let mut result = Vec::new();
    result.extend(run.iter().cloned());

    while try_redundant_stores(&mut result, full_block) {}

    Ok(result)
}

// Reduces patterns that look like this:
// From:
//   store A -> B
//   store B -> C
// To:
//   store A -> C
fn try_redundant_stores(run: &mut Vec<Statement>, full_block: &Vec<Statement>) -> bool {
    for i in 0..(run.len() - 1) {
        let (matches, second_store) = {
            let first_store = match run[i] {
                Statement::Add(ref data) => &data.destination,
                _ => continue,
            };

            let (second_load, second_store) = match run[i + 1] {
                Statement::Copy(ref data) => match data.value {
                    Value::Memory(ref loc) => (loc, &data.destination),
                    _ => continue,
                },
                _ => continue,
            };

            // Make sure this is the only usage before erasing it
            // TODO: Figure out how to also free up the byte of memory
            let mut uses = 0;
            for statement in full_block {
                match *statement {
                    Statement::Copy(ref data) => match data.value {
                        Value::Memory(ref loc) => if second_load == loc {
                            uses += 1;
                        },
                        _ => continue,
                    },
                    _ => continue,
                }
            }

            (
                uses == 1 && first_store == second_load,
                second_store.clone(),
            )
        };
        if matches {
            run[i] = match run[i] {
                Statement::Add(ref data) => Statement::Add(BinaryOpData::new(
                    second_store,
                    data.left.clone(),
                    data.right.clone(),
                )),
                _ => unreachable!(),
            };
            run.remove(i + 1);
            return true;
        }
    }
    false
}
