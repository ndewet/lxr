//! Provides runtime support for generated lxr lexers.
//!
//! Derive [`Lexer`] for a token enum and place one `#[lxr("pattern")]`
//! attribute on each variant.

#![deny(dead_code)]

pub use lxr_derive::Lexer;

mod span;

pub use span::Span;

use std::marker::PhantomData;
use std::str::FromStr;
use std::{
    error::Error,
    fmt::{Display, Formatter},
};

/// A token together with its byte range in the input.
///
/// # Examples
///
/// ```
/// use lxr::{Span, Spanned};
///
/// let token = Spanned {
///     token: "name",
///     span: Span::new(0, 4),
/// };
/// assert_eq!(token.span.text("name 42"), Some("name"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spanned<T> {
    /// The token accepted by the lexer.
    pub token: T,
    /// The half-open UTF-8 byte range matched by this token.
    pub span: Span,
}

/// Holds the items that generated code names, and that callers do not.
#[doc(hidden)]
pub mod __private {
    /// Restricts [`Lexer`](crate::Lexer) to the types that the derive makes.
    pub trait Sealed {}
}

/// A failure produced while converting an accepted lexeme into a payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadError {
    message: String,
}

/// The result of selecting and executing one lexer rule.
#[doc(hidden)]
pub type RuleScan<T> = Result<(Option<T>, usize, Transition), (PayloadError, usize)>;

/// A generated lexer rule's update to the start-condition stack.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    Stay,
    Begin(usize),
    Push(usize),
    Pop,
}

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
///
/// # Examples
///
/// ```
/// use lxr::{ScanError, Span};
///
/// let error = ScanError::Unrecognized {
///     span: Span::new(4, 5),
/// };
/// assert!(matches!(error, ScanError::Unrecognized { span } if span.len() == 1));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanError {
    /// No rule accepted the character at this UTF-8 byte range.
    Unrecognized {
        /// The UTF-8 byte range of the unrecognized character.
        span: Span,
    },
    /// A winning rule's payload conversion failed.
    InvalidPayload {
        /// The UTF-8 byte range matched by the rule with the invalid payload.
        span: Span,
        /// The payload conversion error's message.
        message: String,
    },
    /// Input ended while a pushed lexer mode was still active.
    UnterminatedMode {
        /// The range from the mode-opening rule through end of input.
        span: Span,
        /// The unclosed mode's declared name.
        mode: &'static str,
    },
}

/// Iterates over tokens and recoverable invalid-input errors.
///
/// # Examples
///
/// ```
/// use lxr::{Lexer, Scanner};
///
/// #[derive(Debug, PartialEq, Lexer)]
/// enum Token {
///     #[lxr("[a-z]+")]
///     Word,
/// }
///
/// let scanner = Scanner::<Token>::new("word");
/// assert_eq!(scanner.count(), 1);
/// ```
pub struct Scanner<'input, T> {
    input: &'input str,
    offset: usize,
    modes: Vec<ModeFrame>,
    finished: bool,
    marker: PhantomData<T>,
}

struct ModeFrame {
    id: usize,
    opened_at: usize,
}

impl<'input, T> Scanner<'input, T> {
    /// Creates a scanner at the beginning of `input`.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::{Lexer, Scanner};
    ///
    /// #[derive(Lexer)]
    /// enum Token {
    ///     #[lxr("x")]
    ///     X,
    /// }
    ///
    /// assert!(Scanner::<Token>::new("x").next().is_some());
    /// ```
    pub fn new(input: &'input str) -> Self {
        Self {
            input,
            offset: 0,
            modes: vec![ModeFrame {
                id: 0,
                opened_at: 0,
            }],
            finished: false,
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
                if self.finished {
                    return None;
                }
                self.finished = true;
                if let Some(frame) = self.modes.get(1) {
                    return Some(Err(ScanError::UnterminatedMode {
                        span: Span::new(frame.opened_at as u64, self.offset as u64),
                        mode: T::mode_name(frame.id),
                    }));
                }
                return None;
            }
            let mode = self
                .modes
                .last()
                .expect("the mode stack contains INITIAL")
                .id;
            let Some(result) = T::scan_one(input, mode) else {
                let width = self.input[self.offset..]
                    .chars()
                    .next()
                    .expect("a non-empty UTF-8 string has a first character")
                    .len_utf8();
                let span = Span::new(self.offset as u64, (self.offset + width) as u64);
                self.offset += width;
                return Some(Err(ScanError::Unrecognized { span }));
            };
            let start = self.offset;
            let (token, consumed, transition) = match result {
                Ok(result) => result,
                Err((error, consumed)) => {
                    let span = Span::new(start as u64, (start + consumed) as u64);
                    self.offset = start + consumed;
                    return Some(Err(ScanError::InvalidPayload {
                        span,
                        message: error.to_string(),
                    }));
                }
            };
            self.offset += consumed;
            match transition {
                Transition::Stay => {}
                Transition::Begin(id) => {
                    let frame = self
                        .modes
                        .last_mut()
                        .expect("the mode stack contains INITIAL");
                    frame.id = id;
                    frame.opened_at = start;
                }
                Transition::Push(id) => self.modes.push(ModeFrame {
                    id,
                    opened_at: start,
                }),
                Transition::Pop => {
                    if self.modes.len() > 1 {
                        self.modes.pop();
                    }
                }
            }
            if let Some(token) = token {
                return Some(Ok(Spanned {
                    token,
                    span: Span::new(start as u64, self.offset as u64),
                }));
            }
        }
    }
}

/// Scans UTF-8 input with a generated lexer.
///
/// Only the `Lexer` derive macro implements this trait. The scanner trusts
/// the generated methods, thus a hand-written implementation can break a
/// scan.
///
/// # Examples
///
/// ```
/// use lxr::Lexer;
///
/// #[derive(Debug, PartialEq, Lexer)]
/// enum Token {
///     #[lxr("[a-z]+")]
///     Word,
/// }
///
/// assert_eq!(Token::scan("word"), Some((Token::Word, 4)));
/// ```
pub trait Lexer: __private::Sealed + Sized {
    /// Scans one token or skipped rule from the start of a string.
    ///
    /// `None` means no rule accepts a prefix. A successful result contains a
    /// skipped rule or token plus its consumed byte length. An error means a
    /// winning rule could not convert its payload.
    #[doc(hidden)]
    fn scan_one(input: &str, mode: usize) -> Option<RuleScan<Self>>;

    /// Returns a generated start-condition name for diagnostics.
    #[doc(hidden)]
    fn mode_name(mode: usize) -> &'static str;

    /// Creates a recoverable scanner for `input`.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::Lexer;
    ///
    /// #[derive(Lexer)]
    /// enum Token {
    ///     #[lxr("[a-z]+")]
    ///     Word,
    /// }
    ///
    /// assert_eq!(Token::scanner("word").count(), 1);
    /// ```
    fn scanner(input: &str) -> Scanner<'_, Self> {
        Scanner::new(input)
    }

    /// Returns the first token in `input`, if one is accepted before an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::Lexer;
    ///
    /// #[derive(Debug, PartialEq, Lexer)]
    /// enum Token {
    ///     #[lxr("[0-9]+")]
    ///     Integer,
    /// }
    ///
    /// assert_eq!(Token::scan("42!"), Some((Token::Integer, 2)));
    /// ```
    fn scan(input: &str) -> Option<(Self, usize)> {
        Self::scanner(input)
            .next()?
            .ok()
            .map(|spanned| (spanned.token, spanned.span.end as usize))
    }
}
