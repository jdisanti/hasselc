/// Represents the character offset in the program code where something is located
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SrcTag(pub usize);

impl SrcTag {
    pub fn invalid() -> SrcTag {
        SrcTag(usize::max_value())
    }

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

    /// Given the source program, returns the line of code that the tag points to
    pub fn line<'a>(&self, source: &'a str) -> &'a str {
        match source[self.0..].find('\n') {
            Some(end_index) => &source[self.0..(self.0 + end_index)],
            None => &source[self.0..],
        }
    }
}

pub trait SrcTagged {
    fn src_tag(&self) -> SrcTag;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line() {
        let src = "\
            l1\n\
            l2\n\
            l3\
        ";
        let tag1 = SrcTag(0);
        let tag2 = SrcTag(3);
        let tag3 = SrcTag(6);

        assert_eq!("l1", tag1.line(src));
        assert_eq!("l2", tag2.line(src));
        assert_eq!("l3", tag3.line(src));
    }
}