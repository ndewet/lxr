//! Locates a lexeme in the input by byte offset.

use std::{fmt, ops::Range};

/// A half-open range of UTF-8 byte offsets.
///
/// An offset is absolute, and it starts at zero when the scanner starts.
/// An offset uses `u64`, because a stream can be longer than `usize`.
/// An offset carries no source identity. Use the scanner that made the span.
///
/// # Examples
///
/// ```
/// use lxr::Span;
///
/// let span = Span::new(0, 4);
/// assert_eq!(span.text("name 42"), Some("name"));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    /// The inclusive start of the range.
    pub start: u64,
    /// The exclusive end of the range.
    pub end: u64,
}

impl Span {
    /// Creates a span from its two byte offsets.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::Span;
    ///
    /// assert_eq!(Span::new(2, 5).start, 2);
    /// ```
    #[must_use]
    pub const fn new(start: u64, end: u64) -> Self {
        Self { start, end }
    }

    /// Returns the number of bytes in the range.
    ///
    /// A reversed span has a length of zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::Span;
    ///
    /// assert_eq!(Span::new(2, 5).len(), 3);
    /// ```
    #[must_use]
    pub const fn len(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }

    /// Reports whether the range contains no bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::Span;
    ///
    /// assert!(Span::new(2, 2).is_empty());
    /// ```
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.end <= self.start
    }

    /// Converts the range for an index into memory.
    ///
    /// Returns `None` if an offset does not fit in `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::Span;
    ///
    /// assert_eq!(Span::new(2, 5).range(), Some(2..5));
    /// ```
    #[must_use]
    pub fn range(&self) -> Option<Range<usize>> {
        let start = usize::try_from(self.start).ok()?;
        let end = usize::try_from(self.end).ok()?;
        Some(start..end)
    }

    /// Returns the lexeme that this span identifies in `input`.
    ///
    /// Returns `None` if the range is outside `input`, if the range does not
    /// start and end at a character boundary, or if an offset does not fit in
    /// `usize`. Give the same input that the scanner read.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::Span;
    ///
    /// assert_eq!(Span::new(5, 7).text("name 42"), Some("42"));
    /// assert_eq!(Span::new(5, 99).text("name 42"), None);
    /// ```
    #[must_use]
    pub fn text<'a>(&self, input: &'a str) -> Option<&'a str> {
        input.get(self.range()?)
    }
}

impl From<Range<u64>> for Span {
    fn from(range: Range<u64>) -> Self {
        Self::new(range.start, range.end)
    }
}

impl From<Span> for Range<u64> {
    fn from(span: Span) -> Self {
        span.start..span.end
    }
}

impl fmt::Display for Span {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}..{}", self.start, self.end)
    }
}
