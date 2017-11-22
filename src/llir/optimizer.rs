use llir::FrameBlock;
use error;

pub fn optimize_llir(llir: &[FrameBlock]) -> error::Result<Vec<FrameBlock>> {
    let mut optimized = Vec::new();
    // TODO: Do actual optimizations when they are needed
    optimized.extend(llir.iter().cloned());
    Ok(optimized)
}
