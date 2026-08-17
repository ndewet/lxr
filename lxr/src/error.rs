use std::fmt::{Display, Formatter, Result};
use std::ops::Range;

/// A part of the input that no rule of the lexer matches.
///
/// The scan gives one error for each character that it cannot read, then it moves forward by that
/// character. Thus the iterator does not stop at the first fault, and a caller reads the tokens
/// after it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ScanError {
    /// The bytes of the character at fault.
    ///
    /// The range counts bytes from the start of the input, thus a caller slices the input with it.
    pub span: Range<usize>,
    /// The line of the character, counted from 1.
    pub line: u32,
    /// The column of the character, counted from 1 in characters and not in bytes.
    pub column: u32,
}

impl ScanError {
    /// Creates an error that reports the character at `span`.
    pub(crate) fn new(span: Range<usize>, line: u32, column: u32) -> Self {
        Self { span, line, column }
    }

    /// Returns the correction that the lexer author must make.
    pub fn help(&self) -> &'static str {
        "Add a rule that matches the character. To read the character and give no token, add a \
         rule that skips it."
    }
}

impl Display for ScanError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        write!(
            formatter,
            "no rule matches the input at line {}, column {}",
            self.line, self.column
        )
    }
}

impl std::error::Error for ScanError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_error_names_the_line_and_the_column_of_the_character() {
        let error = ScanError::new(4..5, 2, 3);

        assert_eq!(
            error.to_string(),
            "no rule matches the input at line 2, column 3"
        );
        assert_eq!(error.span, 4..5);
    }

    #[test]
    fn an_error_gives_a_correction() {
        assert!(ScanError::new(0..1, 1, 1).help().starts_with("Add a rule"));
    }
}
