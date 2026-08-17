use std::fmt::{Debug, Formatter, Result as FormatResult};
use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::ops::Range;

use crate::error::ScanError;
use crate::lexer::Lexer;
use crate::located::Locations;
use crate::tables::Tables;

/// One scan of an input, in progress.
///
/// The scan gives one token at a time, thus it implements [`Iterator`]. It also holds where the
/// last token is. Read [`span`](Self::span), [`slice`](Self::slice), [`line`](Self::line), and
/// [`column`](Self::column) after each token.
///
/// A `for` loop takes the scan, thus the body of the loop cannot read the place of the token. Use
/// [`located`](Self::located) for that loop, and it gives the place with the token.
///
/// An offset counts bytes from the start of the input, and a token holds no borrow of the input.
///
/// To make a `Scan`, use [`Lexer::scan`].
pub struct Scan<'a, T> {
    input: &'a str,
    offset: usize,
    condition: u16,
    /// The line at which the last token starts.
    line: u32,
    /// The column at which the last token starts.
    column: u32,
    span: Range<usize>,
    /// The line at which the next token starts.
    next_line: u32,
    /// The column at which the next token starts.
    next_column: u32,
    token: PhantomData<fn() -> T>,
}

impl<'a, T: Lexer> Scan<'a, T> {
    /// Creates a scan of `input` under the first start condition.
    pub(crate) fn new(input: &'a str) -> Self {
        Self {
            input,
            offset: 0,
            condition: 0,
            line: 1,
            column: 1,
            span: 0..0,
            next_line: 1,
            next_column: 1,
            token: PhantomData,
        }
    }

    /// Returns the bytes of the last token, counted from the start of the input.
    ///
    /// The result is `0..0` before the first token.
    pub fn span(&self) -> Range<usize> {
        self.span.clone()
    }

    /// Returns the text of the last token.
    ///
    /// The result is empty before the first token. After a [`ScanError`], it is the text at fault.
    pub fn slice(&self) -> &'a str {
        &self.input[self.span.clone()]
    }

    /// Gives the place of each token with the token, in place of the token alone.
    ///
    /// A `for` loop takes the scan, thus the body of the loop cannot read [`span`](Self::span) or
    /// [`line`](Self::line). Each [`Located`](crate::Located) of this iterator carries them.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::Lexer;
    ///
    /// #[derive(Debug, PartialEq, Lexer)]
    /// #[lxr(skip = " +")]
    /// enum Token {
    ///     #[lxr(regex = "[a-z]+")]
    ///     Word,
    /// }
    ///
    /// let places: Vec<_> = Token::scan("one two")
    ///     .located()
    ///     .map(|found| found.expect("each character belongs to a token"))
    ///     .map(|found| (found.span, found.line, found.column))
    ///     .collect();
    ///
    /// assert_eq!(places, vec![(0..3, 1, 1), (4..7, 1, 5)]);
    /// ```
    pub fn located(self) -> Locations<'a, T> {
        Locations::new(self)
    }

    /// Returns the line at which the last token starts, counted from 1.
    pub fn line(&self) -> u32 {
        self.line
    }

    /// Returns the column at which the last token starts, counted from 1.
    ///
    /// The column counts characters, and not bytes. Thus a character above ASCII counts as one.
    pub fn column(&self) -> u32 {
        self.column
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
        T::condition(self.condition)
    }

    /// Returns the offset at which the next token starts.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Returns the input that the scan has not read.
    pub fn remainder(&self) -> &'a str {
        &self.input[self.offset..]
    }

    /// Moves the scan forward by `length` bytes, and counts the lines and the columns of them.
    ///
    /// A byte of the form `10xxxxxx` continues a character, thus the column does not count it.
    fn bump(&mut self, length: usize) {
        for &byte in &self.input.as_bytes()[self.offset..self.offset + length] {
            if byte == b'\n' {
                self.next_line += 1;
                self.next_column = 1;
            } else if byte & 0xC0 != 0x80 {
                self.next_column += 1;
            }
        }
        self.offset += length;
    }

    /// Returns the number of the bytes of the character at the offset of the scan.
    fn character(&self) -> usize {
        let bytes = self.input.as_bytes();
        let mut length = 1;
        while self.offset + length < bytes.len() && bytes[self.offset + length] & 0xC0 == 0x80 {
            length += 1;
        }
        length
    }

    /// Records the last token as the `length` bytes at the offset of the scan, then moves forward.
    fn take(&mut self, length: usize) {
        self.line = self.next_line;
        self.column = self.next_column;
        self.span = self.offset..self.offset + length;
        self.bump(length);
    }
}

impl<T: Lexer> Iterator for Scan<'_, T> {
    type Item = std::result::Result<T, ScanError>;

    fn next(&mut self) -> Option<Self::Item> {
        let tables = T::TABLES;

        loop {
            if self.offset >= self.input.len() {
                return None;
            }

            let Some((rule, length)) = longest_match(
                &tables,
                tables.start[usize::from(self.condition)],
                &self.input.as_bytes()[self.offset..],
            ) else {
                self.take(self.character());
                return Some(Err(ScanError::no_rule(
                    self.span.clone(),
                    self.line,
                    self.column,
                )));
            };

            let action = tables.actions[usize::from(rule)];
            self.take(length);
            if let Some(condition) = action.go {
                self.condition = condition;
            }
            if !action.skip {
                let value = T::token(rule, self.slice());
                return Some(
                    value
                        .ok_or_else(|| ScanError::value(self.span.clone(), self.line, self.column)),
                );
            }
        }
    }
}

impl<T: Lexer> FusedIterator for Scan<'_, T> {}

impl<T> Debug for Scan<'_, T> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
        formatter
            .debug_struct("Scan")
            .field("offset", &self.offset)
            .field("condition", &self.condition)
            .field("line", &self.line)
            .field("column", &self.column)
            .field("span", &self.span)
            .finish()
    }
}

/// Returns the rule of the longest match at the start of `input`, and the length of that match.
///
/// The scan reads each byte one time, and it keeps the last accept that it reached. Thus a rule
/// that matches a longer input wins, and the earliest rule wins a tie.
///
/// Each rule of a lexer matches at least one byte, thus a match of no length gives `None`.
///
/// # Panics
///
/// This function panics if `tables` disagrees with the conditions of [`Tables`].
fn longest_match(tables: &Tables<'_>, start: u16, input: &[u8]) -> Option<(u16, usize)> {
    let mut state = start;
    let mut best = None;

    for (index, &byte) in input.iter().enumerate() {
        state = tables.step(state, byte);
        if state == 0 {
            break;
        }
        if let Some(rule) = tables.accepts(state) {
            best = Some((rule, index + 1));
        }
    }

    best
}
