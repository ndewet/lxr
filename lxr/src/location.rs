//! Resolves byte positions for diagnostics.

use crate::Span;
use std::io;

/// A one-based line number and byte column.
///
/// LF starts a new line. CR and tabs each occupy one byte column.
/// Positions may identify any byte, including a UTF-8 continuation byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    /// The one-based line number.
    pub line: u64,
    /// The one-based byte column.
    pub column: u64,
}

/// The locations of both endpoints of a half-open span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocatedSpan {
    /// The inclusive starting location.
    pub start: Location,
    /// The exclusive ending location.
    pub end: Location,
}

/// Resolves positions relative to the source's initial position.
///
/// Lookup preserves the source's read position unless restoration fails.
/// EOF is a valid position. Offsets carry no source identity.
pub trait Locate {
    /// Returns the current read position in this source's coordinates.
    ///
    /// # Errors
    ///
    /// Returns an error if the source position is unavailable.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::{Locate, Replay};
    /// let mut source = Replay::new(std::io::Cursor::new(b"abc"))?;
    /// assert_eq!(source.current_offset()?, 0);
    /// # Ok::<(), std::io::Error>(())
    /// ```
    fn current_offset(&mut self) -> io::Result<u64>;

    /// Resolves a byte position.
    ///
    /// # Errors
    ///
    /// Returns an error for unavailable positions, column overflow, or I/O failures.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::{Locate, Replay};
    /// let mut source = Replay::new(std::io::Cursor::new(b"a\nb"))?;
    /// assert_eq!(source.locate(2)?.line, 2);
    /// # Ok::<(), std::io::Error>(())
    /// ```
    fn locate(&mut self, offset: u64) -> io::Result<Location>;

    /// Resolves both endpoints of a span.
    ///
    /// # Errors
    ///
    /// Returns an error for reversed spans or an endpoint lookup failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::{Locate, Replay, Span};
    /// let mut source = Replay::new(std::io::Cursor::new(b"abc"))?;
    /// assert_eq!(source.locate_span(Span::new(0, 3))?.end.column, 4);
    /// # Ok::<(), std::io::Error>(())
    /// ```
    fn locate_span(&mut self, span: Span) -> io::Result<LocatedSpan> {
        if span.start > span.end {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "reversed span"));
        }
        let start = self.locate(span.start)?;
        let end = self.locate(span.end)?;
        Ok(LocatedSpan { start, end })
    }
}

pub(crate) fn resolve(lines: &[u64], offset: u64) -> io::Result<Location> {
    let line = lines.partition_point(|&start| start <= offset);
    let column = (offset - lines[line - 1])
        .checked_add(1)
        .ok_or_else(|| io::Error::other("column exceeds u64"))?;
    Ok(Location {
        line: line as u64,
        column,
    })
}
