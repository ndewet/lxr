use std::fmt::{Debug, Display, Formatter, Result};
use std::ops::Range;

/// A part of the input that the lexer cannot read.
///
/// The scan gives one error, then it reads the input after the part at fault. Thus the iterator
/// does not stop at the first fault, and a caller reads the tokens after it.
///
/// The error holds one pointer to its detail, and the detail holds the span, the place, and the
/// kind. A scan gives a `Result` for each token, thus the size of this type is the size of each
/// token that the iterator moves. A fault is rare, and it pays one allocation for that size.
#[derive(Clone, PartialEq, Eq)]
pub struct ScanError {
    detail: Box<Detail>,
}

/// The span, the place, and the kind of a [`ScanError`].
#[derive(Debug, Clone, PartialEq, Eq)]
struct Detail {
    span: Range<usize>,
    line: u32,
    column: u32,
    kind: ScanErrorKind,
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
        Self::new(span, line, column, ScanErrorKind::NoRule)
    }

    /// Creates an error that reports the match at `span`, which does not fit its field.
    pub(crate) fn value(span: Range<usize>, line: u32, column: u32) -> Self {
        Self::new(span, line, column, ScanErrorKind::Value)
    }

    /// Creates an error of `kind` at `span`.
    fn new(span: Range<usize>, line: u32, column: u32, kind: ScanErrorKind) -> Self {
        Self {
            detail: Box::new(Detail {
                span,
                line,
                column,
                kind,
            }),
        }
    }

    /// Returns the bytes at fault, counted from the start of the input.
    ///
    /// A caller slices the input with the range.
    ///
    /// # Examples
    ///
    #[cfg_attr(feature = "derive", doc = "```")]
    #[cfg_attr(not(feature = "derive"), doc = "```ignore")]
    /// use lxr::Lexer;
    ///
    /// #[derive(Debug, PartialEq, Lexer)]
    /// enum Token {
    ///     #[lxr(regex = "[a-z]+")]
    ///     Word,
    /// }
    ///
    /// let error = Token::scan("one!")
    ///     .nth(1)
    ///     .expect("the scan reports the character")
    ///     .expect_err("no rule matches an exclamation mark");
    ///
    /// assert_eq!(error.span(), 3..4);
    /// ```
    pub fn span(&self) -> Range<usize> {
        self.detail.span.clone()
    }

    /// Returns the line of the first character, counted from 1.
    ///
    /// # Examples
    ///
    #[cfg_attr(feature = "derive", doc = "```")]
    #[cfg_attr(not(feature = "derive"), doc = "```ignore")]
    /// use lxr::Lexer;
    ///
    /// #[derive(Debug, PartialEq, Lexer)]
    /// #[lxr(skip = "\n")]
    /// enum Token {
    ///     #[lxr(regex = "[a-z]+")]
    ///     Word,
    /// }
    ///
    /// let error = Token::scan("one\n!")
    ///     .nth(1)
    ///     .expect("the scan reports the character")
    ///     .expect_err("no rule matches an exclamation mark");
    ///
    /// assert_eq!((error.line(), error.column()), (2, 1));
    /// ```
    pub fn line(&self) -> u32 {
        self.detail.line
    }

    /// Returns the column of the first character, counted from 1.
    ///
    /// The column counts characters, and not bytes. Thus a character above ASCII counts as one.
    ///
    /// # Examples
    ///
    #[cfg_attr(feature = "derive", doc = "```")]
    #[cfg_attr(not(feature = "derive"), doc = "```ignore")]
    /// use lxr::Lexer;
    ///
    /// #[derive(Debug, PartialEq, Lexer)]
    /// enum Token {
    ///     #[lxr(regex = "[a-z]+")]
    ///     Word,
    /// }
    ///
    /// let error = Token::scan("one!")
    ///     .nth(1)
    ///     .expect("the scan reports the character")
    ///     .expect_err("no rule matches an exclamation mark");
    ///
    /// assert_eq!(error.column(), 4);
    /// ```
    pub fn column(&self) -> u32 {
        self.detail.column
    }

    /// Returns the kind of the fault.
    ///
    /// # Examples
    ///
    #[cfg_attr(feature = "derive", doc = "```")]
    #[cfg_attr(not(feature = "derive"), doc = "```ignore")]
    /// use lxr::{Lexer, ScanErrorKind};
    ///
    /// #[derive(Debug, PartialEq, Lexer)]
    /// enum Token {
    ///     #[lxr(regex = "[a-z]+")]
    ///     Word,
    /// }
    ///
    /// let error = Token::scan("one!")
    ///     .nth(1)
    ///     .expect("the scan reports the character")
    ///     .expect_err("no rule matches an exclamation mark");
    ///
    /// assert_eq!(error.kind(), ScanErrorKind::NoRule);
    /// ```
    pub fn kind(&self) -> ScanErrorKind {
        self.detail.kind
    }

    /// Returns the correction of this fault.
    ///
    /// The correction names the input first. A fault of the input does not always show a fault of
    /// the lexer, thus a change to the lexer comes second.
    ///
    /// # Examples
    ///
    #[cfg_attr(feature = "derive", doc = "```")]
    #[cfg_attr(not(feature = "derive"), doc = "```ignore")]
    /// use lxr::{Lexer, ScanErrorKind};
    ///
    /// #[derive(Debug, PartialEq, Lexer)]
    /// enum Token {
    ///     #[lxr(regex = "[a-z]+")]
    ///     Word,
    /// }
    ///
    /// let error = Token::scan("!")
    ///     .next()
    ///     .expect("the scan reports the character")
    ///     .expect_err("no rule matches an exclamation mark");
    ///
    /// assert_eq!(error.help(), ScanErrorKind::NoRule.help());
    /// ```
    pub fn help(&self) -> &'static str {
        self.detail.kind.help()
    }
}

impl ScanErrorKind {
    /// Returns the correction of this kind of fault.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::ScanErrorKind;
    ///
    /// assert!(ScanErrorKind::NoRule.help().starts_with("Add a rule"));
    /// ```
    pub fn help(&self) -> &'static str {
        match self {
            Self::NoRule => {
                "Add a rule that matches the character. To read the character and give no token, \
                 add a rule that skips it."
            }
            Self::Value => {
                "The text does not fit the field. Correct the input, or give the token a wider \
                 field or a pattern that matches only the text that fits."
            }
        }
    }
}

impl Debug for ScanError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        formatter
            .debug_struct("ScanError")
            .field("span", &self.detail.span)
            .field("line", &self.detail.line)
            .field("column", &self.detail.column)
            .field("kind", &self.detail.kind)
            .finish()
    }
}

impl Display for ScanError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        write!(
            formatter,
            "{} at line {}, column {}",
            self.detail.kind, self.detail.line, self.detail.column
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
        assert_eq!(error.span(), 4..5);
        assert_eq!(error.kind(), ScanErrorKind::NoRule);
    }

    #[test]
    fn an_error_of_a_value_names_the_field() {
        let error = ScanError::value(0..20, 1, 1);

        assert_eq!(
            error.to_string(),
            "the text does not fit the field of its token at line 1, column 1"
        );
        assert_eq!(error.kind(), ScanErrorKind::Value);
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

    #[test]
    fn an_error_shows_its_span_and_its_place() {
        let error = ScanError::no_rule(4..5, 2, 3);

        assert_eq!(
            format!("{error:?}"),
            "ScanError { span: 4..5, line: 2, column: 3, kind: NoRule }"
        );
    }

    #[test]
    fn an_error_is_the_size_of_one_pointer() {
        assert_eq!(size_of::<ScanError>(), size_of::<usize>());
    }
}
