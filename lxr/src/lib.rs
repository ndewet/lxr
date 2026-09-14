//! Provides runtime support for generated lxr lexers.
//!
//! Derive [`Lexer`] for a token enum and place one `#[lxr("pattern")]`
//! attribute on each variant.

#![deny(dead_code)]

pub use lxr_derive::Lexer;

use std::marker::PhantomData;
use std::ops::Range;
use std::str::FromStr;
use std::{
    error::Error,
    fmt::{Display, Formatter},
};

/// A token together with its byte range in the input.
#[derive(Debug, PartialEq, Eq)]
pub struct Spanned<T> {
    /// The token accepted by the lexer.
    pub token: T,
    /// The half-open UTF-8 byte range matched by this token.
    pub span: Range<usize>,
}

/// A failure produced while converting an accepted lexeme into a payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadError {
    message: String,
}

/// The result of selecting and executing one lexer rule.
#[doc(hidden)]
pub type RuleScan<T> = Result<(Option<T>, usize), (PayloadError, usize)>;

impl PayloadError {
    /// Creates a payload-conversion error with a human-readable message.
    #[doc(hidden)]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for PayloadError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for PayloadError {}

/// Parses one owned token payload with its [`FromStr`] implementation.
#[doc(hidden)]
pub fn parse_payload<T>(text: &str) -> Result<T, PayloadError>
where
    T: FromStr,
    T::Err: Display,
{
    text.parse()
        .map_err(|error: T::Err| PayloadError::new(error.to_string()))
}

/// An error encountered while scanning input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanError {
    /// No rule accepted the character at this UTF-8 byte range.
    Unrecognized {
        /// The UTF-8 byte range of the unrecognized character.
        span: Range<usize>,
    },
    /// A winning rule's payload conversion failed.
    InvalidPayload {
        /// The UTF-8 byte range matched by the rule with the invalid payload.
        span: Range<usize>,
        /// The payload conversion error's message.
        message: String,
    },
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
            let input = &self.input[self.offset..];
            if input.is_empty() {
                return None;
            }
            let Some(result) = T::scan_one(input) else {
                let width = self.input[self.offset..]
                    .chars()
                    .next()
                    .expect("a non-empty UTF-8 string has a first character")
                    .len_utf8();
                let span = self.offset..self.offset + width;
                self.offset += width;
                return Some(Err(ScanError::Unrecognized { span }));
            };
            let start = self.offset;
            let (token, consumed) = match result {
                Ok(result) => result,
                Err((error, consumed)) => {
                    let span = start..start + consumed;
                    self.offset = span.end;
                    return Some(Err(ScanError::InvalidPayload {
                        span,
                        message: error.to_string(),
                    }));
                }
            };
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
    /// Scans one token or skipped rule from the start of a string.
    ///
    /// `None` means no rule accepts a prefix. A successful result contains a
    /// skipped rule or token plus its consumed byte length. An error means a
    /// winning rule could not convert its payload.
    #[doc(hidden)]
    fn scan_one(input: &str) -> Option<RuleScan<Self>>;

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
