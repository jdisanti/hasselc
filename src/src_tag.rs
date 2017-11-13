/// Represents the character offset in the program code where something is located
#[derive(Debug, Clone, Copy)]
pub struct SrcTag(pub usize);

impl SrcTag {
    /// Returns the (row, column) of this tag in the given program text
    pub fn row_col(&self, program: &str) -> (usize, usize) {
        let mut row: usize = 1;
        let mut col: usize = 1;

        for i in 0..self.0 {
            if &program[i..i + 1] == "\n" {
                row += 1;
                col = 1;
            } else {
                col += 1;
            }
        }

        (row, col)
    }
}
