//! Provides runtime support for generated lxr lexers.
//!
//! Derive [`Lexer`] for a token enum and place one `#[lxr("pattern")]`
//! attribute on each variant.

#![deny(dead_code)]

pub use lxr_derive::Lexer;

mod limits;
mod location;
mod scanner;
mod source;
mod source_error;
mod span;

pub use limits::Limits;
pub use location::{Locate, LocatedSpan, Location};
pub use scanner::{Remainder, Scanner};
pub use source::{Replay, Tracking};
pub use source_error::SourceError;
pub use span::Span;

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

/// A failure produced while converting an accepted lexeme into a payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadError {
    message: String,
}

/// Holds the items that generated code names, and that callers do not.
#[doc(hidden)]
pub mod __private {
    /// Restricts [`Lexer`](crate::Lexer) to the types that the derive makes.
    pub trait Sealed {}
}

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
/// `Unrecognized` and `InvalidPayload` are recoverable, thus the scan
/// continues. Each other variant is terminal, thus iteration stops.
///
/// A terminal error discards a pending token. The scanner reports the
/// terminal error, and not the rule that it accepted before the failure.
///
/// # Examples
///
/// ```
/// use lxr::{ScanError, Span};
///
/// let error = ScanError::Unrecognized {
///     span: Span::new(4, 5),
/// };
/// assert_eq!(error.to_string(), "unrecognized input at 4..5");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanError {
    /// A source operation failed. Scanning stops.
    Input {
        /// The source position reached before the failure.
        offset: u64,
        /// The original I/O error.
        error: SourceError,
    },
    /// The input contains invalid or incomplete UTF-8. Scanning stops.
    InvalidEncoding {
        /// The byte position where the invalid sequence starts.
        offset: u64,
    },
    /// Retained input exceeds the configured limit. Scanning stops.
    RetentionLimit {
        /// The token's starting position.
        offset: u64,
        /// The configured byte limit.
        limit: usize,
    },
    /// The mode stack exceeds the configured limit. Scanning stops.
    ModeLimit {
        /// The rule's byte range.
        span: Span,
        /// The configured stack depth.
        limit: usize,
    },
    /// A byte position cannot fit in u64. Scanning stops.
    PositionOverflow,
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

/// Scans UTF-8 input with a generated lexer.
///
/// Only the `Lexer` derive macro implements this trait. The scanner trusts
/// the automaton methods, thus a hand-written implementation can break a
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
    /// Returns the initial execution state for a mode.
    #[doc(hidden)]
    fn start(mode: usize) -> usize;
    /// Advances the execution by one byte.
    #[doc(hidden)]
    fn step(state: usize, byte: u8) -> Option<usize>;
    /// Returns the accepting rule for a state.
    #[doc(hidden)]
    fn accept(state: usize) -> Option<usize>;
    /// Reports whether a state has outgoing transitions.
    #[doc(hidden)]
    fn continues(state: usize) -> bool;
    /// Converts the selected lexeme.
    #[doc(hidden)]
    fn action(rule: usize, text: &str) -> Result<(Option<Self>, Transition), PayloadError>;

    /// Creates a scanner with buffering for a blocking reader.
    ///
    /// This scanner does not resolve a line and a column. Memory stays
    /// bounded, thus a large stream is safe. Use
    /// [`from_tracked_reader`](Lexer::from_tracked_reader) for a diagnostic
    /// that needs a line number.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::Lexer;
    /// #[derive(Lexer)]
    /// enum Token { #[lxr("x")] X }
    /// assert_eq!(Token::from_reader(&b"x"[..]).count(), 1);
    /// ```
    fn from_reader<R: std::io::Read>(reader: R) -> Scanner<Self, std::io::BufReader<R>> {
        Scanner::<Self>::from_reader(reader)
    }

    /// Creates a scanner that resolves a line and a column for a reader.
    ///
    /// The adapter retains one `u64` for each line that it reads. This index
    /// has no bound, and [`Limits`] excludes it. Use
    /// [`from_reader`](Lexer::from_reader) for a large stream that needs no
    /// line number.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::{Lexer, Locate};
    /// #[derive(Lexer)]
    /// #[lxr(skip = r"\n")]
    /// enum Token { #[lxr("x")] X }
    /// let mut scanner = Token::from_tracked_reader(&b"x\nx"[..]);
    /// assert_eq!(scanner.by_ref().count(), 2);
    /// assert_eq!(scanner.locate(2)?.line, 2);
    /// # Ok::<(), std::io::Error>(())
    /// ```
    fn from_tracked_reader<R: std::io::Read>(
        reader: R,
    ) -> Scanner<Self, Tracking<std::io::BufReader<R>>> {
        Scanner::<Self>::from_bufread(Tracking::new(std::io::BufReader::new(reader)))
    }

    /// Creates a scanner from an existing blocking buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::Lexer;
    /// #[derive(Lexer)]
    /// enum Token { #[lxr("x")] X }
    /// assert_eq!(Token::from_bufread(&b"x"[..]).count(), 1);
    /// ```
    fn from_bufread<S: std::io::BufRead>(source: S) -> Scanner<Self, S> {
        Scanner::<Self>::from_bufread(source)
    }

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
    fn scanner(input: &str) -> Scanner<Self, Replay<std::io::Cursor<&[u8]>>> {
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
        Self::scanner(input).next()?.ok().map(|spanned| {
            (
                spanned.token,
                usize::try_from(spanned.span.end).expect("a string length fits usize"),
            )
        })
    }
}

impl Display for ScanError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unrecognized { span } => write!(formatter, "unrecognized input at {span}"),
            Self::InvalidPayload { span, message } => {
                write!(formatter, "invalid payload at {span}: {message}")
            }
            Self::UnterminatedMode { span, mode } => {
                write!(formatter, "unterminated mode {mode} at {span}")
            }
            Self::Input { offset, .. } => write!(formatter, "input error at {offset}"),
            Self::InvalidEncoding { offset } => write!(formatter, "invalid UTF-8 at {offset}"),
            Self::RetentionLimit { offset, limit } => write!(
                formatter,
                "retained input at {offset} exceeds {limit} bytes"
            ),
            Self::ModeLimit { span, limit } => {
                write!(formatter, "mode depth at {span} exceeds {limit}")
            }
            Self::PositionOverflow => formatter.write_str("input position exceeds u64"),
        }
    }
}

impl Error for ScanError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Input { error, .. } => Some(error),
            _ => None,
        }
    }
}
