use std::fmt::{Debug, Display, Formatter, Result};
use std::ops::Range;

/// A part of the input that the lexer cannot read.
///
/// A plain scan records the span and kind only. A scan converted with
/// [`Scan::located`](crate::Scan::located) also records line and column. This
/// keeps fault recovery cheap when a caller does not request source locations.
#[derive(Clone, PartialEq, Eq)]
pub struct ScanError {
    span: Range<usize>,
    place: Option<(u32, u32)>,
    kind: ScanErrorKind,
}

/// The kind of fault reported by a scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScanErrorKind {
    /// No rule matches the character. The span covers that character.
    NoRule,
    /// A match does not fit the field of its token.
    Value,
}

impl ScanError {
    pub(crate) fn no_rule(span: Range<usize>) -> Self {
        Self::new(span, ScanErrorKind::NoRule)
    }

    pub(crate) fn value(span: Range<usize>) -> Self {
        Self::new(span, ScanErrorKind::Value)
    }

    fn new(span: Range<usize>, kind: ScanErrorKind) -> Self {
        Self {
            span,
            place: None,
            kind,
        }
    }

    pub(crate) fn locate(mut self, line: u32, column: u32) -> Self {
        self.place = Some((line, column));
        self
    }

    /// Returns the bytes at fault, counted from the start of the input.
    pub fn span(&self) -> Range<usize> {
        self.span.clone()
    }

    /// Returns the line when location tracking was requested.
    pub fn line(&self) -> Option<u32> {
        self.place.map(|place| place.0)
    }

    /// Returns the column when location tracking was requested.
    pub fn column(&self) -> Option<u32> {
        self.place.map(|place| place.1)
    }

    /// Returns the kind of fault.
    pub fn kind(&self) -> ScanErrorKind {
        self.kind
    }

    /// Returns the correction for this fault.
    pub fn help(&self) -> &'static str {
        self.kind.help()
    }
}

impl ScanErrorKind {
    /// Returns a correction for this kind of fault.
    pub fn help(&self) -> &'static str {
        match self {
            Self::NoRule => "Add a rule that matches the character, or add a rule that skips it.",
            Self::Value => "Correct the input, or use a wider field or a narrower pattern.",
        }
    }
}

impl Debug for ScanError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        formatter
            .debug_struct("ScanError")
            .field("span", &self.span)
            .field("place", &self.place)
            .field("kind", &self.kind)
            .finish()
    }
}

impl Display for ScanError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        if let Some((line, column)) = self.place {
            write!(formatter, "{} at line {line}, column {column}", self.kind)
        } else {
            write!(
                formatter,
                "{} at bytes {}..{}",
                self.kind, self.span.start, self.span.end
            )
        }
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
    fn a_plain_error_has_no_place() {
        let error = ScanError::no_rule(4..5);
        assert_eq!(error.span(), 4..5);
        assert_eq!((error.line(), error.column()), (None, None));
        assert_eq!(error.kind(), ScanErrorKind::NoRule);
    }

    #[test]
    fn a_located_error_has_a_place() {
        let error = ScanError::value(0..20).locate(2, 3);
        assert_eq!((error.line(), error.column()), (Some(2), Some(3)));
        assert_eq!(error.kind(), ScanErrorKind::Value);
    }

    #[test]
    fn each_kind_gives_a_correction() {
        for kind in [ScanErrorKind::NoRule, ScanErrorKind::Value] {
            assert!(kind.help().ends_with('.'));
        }
    }
}
