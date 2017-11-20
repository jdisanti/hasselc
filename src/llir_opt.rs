use llir::FrameBlock;
use error;

pub fn optimize_llir(llir: &Vec<FrameBlock>) -> error::Result<Vec<FrameBlock>> {
    // TODO: Do actual optimizations when they are needed
    Ok(llir.clone())
}
