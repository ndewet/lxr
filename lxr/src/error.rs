use std::fmt::{Display, Formatter, Result};
use std::ops::Range;

/// A part of the input that the lexer cannot read.
///
/// The scan gives one error, then it reads the input after the part at fault. Thus the iterator
/// does not stop at the first fault, and a caller reads the tokens after it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ScanError {
    /// The bytes at fault.
    ///
    /// The range counts bytes from the start of the input, thus a caller slices the input with it.
    pub span: Range<usize>,
    /// The line of the first character, counted from 1.
    pub line: u32,
    /// The column of the first character, counted from 1 in characters and not in bytes.
    pub column: u32,
    /// The kind of the fault.
    pub kind: ScanErrorKind,
}

/// The kind of fault that a [`ScanError`] reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScanErrorKind {
    /// No rule of the start condition matches the character.
    ///
    /// The span covers that one character.
    NoRule,
    /// A rule matched, and its text does not fit the field of the token that it gives.
    ///
    /// A rule of `[0-9]+` matches a number of any length, and a field of `u32` holds a number up
    /// to 4294967295. The span covers the whole match.
    Value,
}

impl ScanError {
    /// Creates an error that reports the character at `span`, which no rule matches.
    pub(crate) fn no_rule(span: Range<usize>, line: u32, column: u32) -> Self {
        Self {
            span,
            line,
            column,
            kind: ScanErrorKind::NoRule,
        }
    }

    /// Creates an error that reports the match at `span`, which does not fit its field.
    pub(crate) fn value(span: Range<usize>, line: u32, column: u32) -> Self {
        Self {
            span,
            line,
            column,
            kind: ScanErrorKind::Value,
        }
    }

    /// Returns the correction that the lexer author must make.
    pub fn help(&self) -> &'static str {
        self.kind.help()
    }
}

impl ScanErrorKind {
    /// Returns the correction that the lexer author must make.
    pub fn help(&self) -> &'static str {
        match self {
            Self::NoRule => {
                "Add a rule that matches the character. To read the character and give no token, \
                 add a rule that skips it."
            }
            Self::Value => {
                "Give the token a field of a wider type, or write a pattern that matches only the \
                 text that the field holds."
            }
        }
    }
}

impl Display for ScanError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        write!(
            formatter,
            "{} at line {}, column {}",
            self.kind, self.line, self.column
        )
    }
}

impl Display for ScanErrorKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        match self {
            Self::NoRule => write!(formatter, "no rule matches the input"),
            Self::Value => write!(formatter, "the text does not fit the field of its token"),
        }
    }
}

impl std::error::Error for ScanError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_error_names_the_line_and_the_column_of_the_character() {
        let error = ScanError::no_rule(4..5, 2, 3);

        assert_eq!(
            error.to_string(),
            "no rule matches the input at line 2, column 3"
        );
        assert_eq!(error.span, 4..5);
        assert_eq!(error.kind, ScanErrorKind::NoRule);
    }

    #[test]
    fn an_error_of_a_value_names_the_field() {
        let error = ScanError::value(0..20, 1, 1);

        assert_eq!(
            error.to_string(),
            "the text does not fit the field of its token at line 1, column 1"
        );
        assert_eq!(error.kind, ScanErrorKind::Value);
    }

    #[test]
    fn each_kind_gives_a_correction() {
        for kind in [ScanErrorKind::NoRule, ScanErrorKind::Value] {
            assert!(kind.help().ends_with('.'));
            assert!(!kind.to_string().is_empty());
        }
    }

    #[test]
    fn an_error_gives_the_correction_of_its_kind() {
        let error = ScanError::no_rule(0..1, 1, 1);

        assert_eq!(error.help(), ScanErrorKind::NoRule.help());
    }
}
