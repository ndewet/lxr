use std::fmt::{Debug, Formatter, Result as FormatResult};
use std::iter::FusedIterator;
use std::ops::Range;

use crate::error::ScanError;
use crate::lexer::Lexer;
use crate::scan::Scan;

/// One token of a scan, with the place at which it starts.
///
/// A [`Scan`] holds the place of the last token, thus a `for` loop over the scan cannot read it.
/// [`Scan::located`] gives this value instead, thus the token and its place arrive together.
///
/// An offset counts bytes from the start of the input, and the token holds no borrow of the input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located<T> {
    /// The token that the rule gave.
    pub token: T,
    /// The bytes of the token, counted from the start of the input.
    ///
    /// A caller slices the input with the range.
    pub span: Range<usize>,
    /// The line at which the token starts, counted from 1.
    pub line: u32,
    /// The column at which the token starts, counted from 1.
    ///
    /// The column counts characters, and not bytes. Thus a character above ASCII counts as one.
    pub column: u32,
}

/// One scan of an input, which gives the place of each token with the token.
///
/// [`Scan::located`] makes it. The iterator gives one [`Located`] at a time, and it reports each
/// character that no rule matches, exactly as a [`Scan`] does.
pub struct Locations<'a, T> {
    scan: Scan<'a, T>,
}

impl<'a, T: Lexer> Locations<'a, T> {
    /// Creates a scan that gives the place of each token of `scan`.
    pub(crate) fn new(scan: Scan<'a, T>) -> Self {
        Self { scan }
    }

    /// Returns the start condition under which the scan reads the next token.
    ///
    /// A rule changes the condition after it matches. Thus this is the condition of the next token,
    /// and not the condition of the last one.
    ///
    /// # Panics
    ///
    /// This function panics if the tables name a condition that the lexer does not hold.
    pub fn condition(&self) -> T::Condition {
        self.scan.condition()
    }
}

impl<T: Lexer> Iterator for Locations<'_, T> {
    type Item = Result<Located<T>, ScanError>;

    fn next(&mut self) -> Option<Self::Item> {
        let found = self.scan.next()?;

        Some(found.map(|token| Located {
            token,
            span: self.scan.span(),
            line: self.scan.line(),
            column: self.scan.column(),
        }))
    }
}

impl<T: Lexer> FusedIterator for Locations<'_, T> {}

impl<T> Debug for Locations<'_, T> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
        formatter
            .debug_struct("Locations")
            .field("scan", &self.scan)
            .finish()
    }
}
