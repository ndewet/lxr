//! Provides runtime support for generated lxr lexers.
//!
//! Derive [`Lexer`] for a unit enum and place one `#[lxr("pattern")]`
//! attribute on each variant.

#![deny(dead_code)]

pub use lxr_derive::Lexer;

use std::marker::PhantomData;
use std::ops::Range;

/// A token together with its byte range in the input.
#[derive(Debug, PartialEq, Eq)]
pub struct Spanned<T> {
    /// The token accepted by the lexer.
    pub token: T,
    /// The half-open UTF-8 byte range matched by this token.
    pub span: Range<usize>,
}

/// An input character that no lexer rule accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanError {
    /// The UTF-8 byte range of the unrecognized character.
    pub span: Range<usize>,
}

/// Iterates over tokens and recoverable invalid-input errors.
pub struct Scanner<'input, T> {
    input: &'input str,
    offset: usize,
    marker: PhantomData<T>,
}

impl<'input, T> Scanner<'input, T> {
    /// Creates a scanner at the beginning of `input`.
    pub fn new(input: &'input str) -> Self {
        Self {
            input,
            offset: 0,
            marker: PhantomData,
        }
    }
}

impl<T: Lexer> Iterator for Scanner<'_, T> {
    type Item = Result<Spanned<T>, ScanError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let input = &self.input.as_bytes()[self.offset..];
            if input.is_empty() {
                return None;
            }
            let Some((token, consumed)) = T::scan_one(input) else {
                let width = self.input[self.offset..]
                    .chars()
                    .next()
                    .expect("a non-empty UTF-8 string has a first character")
                    .len_utf8();
                let span = self.offset..self.offset + width;
                self.offset += width;
                return Some(Err(ScanError { span }));
            };
            let start = self.offset;
            self.offset += consumed;
            if let Some(token) = token {
                return Some(Ok(Spanned {
                    token,
                    span: start..self.offset,
                }));
            }
        }
    }
}

/// Scans UTF-8 input with a generated lexer.
pub trait Lexer: Sized {
    /// Scans one token or skipped rule from the start of a byte slice.
    ///
    /// `None` means no rule accepts a prefix. `Some(None, length)` consumes a
    /// skipped rule; `Some(Some(token), length)` consumes a token rule.
    #[doc(hidden)]
    fn scan_one(input: &[u8]) -> Option<(Option<Self>, usize)>;

    /// Creates a recoverable scanner for `input`.
    fn scanner(input: &str) -> Scanner<'_, Self> {
        Scanner::new(input)
    }

    /// Returns the first token in `input`, if one is accepted before an error.
    fn scan(input: &str) -> Option<(Self, usize)> {
        Self::scanner(input)
            .next()?
            .ok()
            .map(|spanned| (spanned.token, spanned.span.end))
    }
}
