//! Provides runtime support for generated lxr lexers.
//!
//! Derive [`Lexer`] for a token enum and place one `#[lxr("pattern")]`
//! attribute on each variant.

#![deny(dead_code)]

pub use lxr_derive::Lexer;
pub use source::{Reader, Remainder, Slice, Source};

mod source;

mod payload;
mod span;

pub use payload::PayloadResult;
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
/// let error: ScanError = ScanError::Unrecognized {
///     span: Span::new(4, 5),
/// };
/// assert!(matches!(error, ScanError::Unrecognized { span } if span.len() == 1));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanError<E = std::convert::Infallible> {
    /// No rule accepted the character at this UTF-8 byte range.
    Unrecognized {
        /// The UTF-8 byte range of the unrecognized character.
        span: Span,
    },
    /// The source contained a byte sequence that is not valid UTF-8.
    InvalidUtf8 {
        /// The byte range of the invalid sequence.
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
    /// Reading more input from the source failed.
    Source {
        /// The byte offset at which the read failed.
        offset: u64,
        /// The error returned by the source.
        error: E,
    },
}

impl<E: Display> Display for ScanError<E> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unrecognized { span } => write!(formatter, "unrecognized input at {span}"),
            Self::InvalidUtf8 { span } => write!(formatter, "invalid UTF-8 at {span}"),
            Self::InvalidPayload { span, message } => {
                write!(formatter, "invalid token payload at {span}: {message}")
            }
            Self::UnterminatedMode { span, mode } => {
                write!(formatter, "unterminated lexer mode {mode} at {span}")
            }
            Self::Source { offset, error } => {
                write!(formatter, "source error at byte {offset}: {error}")
            }
        }
    }
}

impl<E: Error + 'static> Error for ScanError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Source { error, .. } => Some(error),
            _ => None,
        }
    }
}

/// Iterates over tokens and scanning errors.
///
/// Invalid input and payload errors consume their byte ranges and scanning
/// continues. A source error ends iteration because maximal-munch selection
/// cannot know whether unread bytes would extend the current token.
///
/// # Examples
///
/// ```
/// use lxr::{Lexer, Scanner, Slice};
///
/// #[derive(Debug, PartialEq, Lexer)]
/// enum Token {
///     #[lxr("[a-z]+")]
///     Word,
/// }
///
/// let scanner = Scanner::<Token, _>::new(Slice::from("word"));
/// assert_eq!(scanner.count(), 1);
/// ```
pub struct Scanner<T, S> {
    source: S,
    buffer: Vec<u8>,
    buffer_start: usize,
    offset: u64,
    modes: Vec<ModeFrame>,
    eof: bool,
    finished: bool,
    marker: PhantomData<T>,
}

struct ModeFrame {
    id: usize,
    opened_at: u64,
}

impl<T, S: Source> Scanner<T, S> {
    /// Creates a scanner at the beginning of `source`.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::{Lexer, Scanner, Slice};
    ///
    /// #[derive(Lexer)]
    /// enum Token {
    ///     #[lxr("x")]
    ///     X,
    /// }
    ///
    /// let source = Slice::from("x");
    /// assert!(Scanner::<Token, _>::new(source).next().is_some());
    /// ```
    pub fn new(source: S) -> Self {
        Self {
            source,
            buffer: Vec::new(),
            buffer_start: 0,
            offset: 0,
            modes: vec![ModeFrame {
                id: 0,
                opened_at: 0,
            }],
            eof: false,
            finished: false,
            marker: PhantomData,
        }
    }

    /// Returns the byte offset at which the next token will begin.
    pub const fn position(&self) -> u64 {
        self.offset
    }

    /// Stops scanning and returns all unread input.
    ///
    /// The returned source first replays bytes read during token lookahead,
    /// then continues with the original source.
    pub fn into_source(self) -> Remainder<S> {
        Remainder::new(self.buffer, self.buffer_start, self.source)
    }

    fn unread(&self) -> &[u8] {
        &self.buffer[self.buffer_start..]
    }

    fn read_more(&mut self) -> Result<(), S::Error> {
        const CHUNK_SIZE: usize = 8 * 1024;

        if self.buffer_start > 0 {
            self.buffer.drain(..self.buffer_start);
            self.buffer_start = 0;
        }
        let mut chunk = [0; CHUNK_SIZE];
        let length = self.source.read(&mut chunk)?;
        assert!(
            length <= chunk.len(),
            "a Source returned more bytes than its buffer can hold"
        );
        if length == 0 {
            self.eof = true;
        } else {
            self.buffer.extend_from_slice(&chunk[..length]);
        }
        Ok(())
    }

    fn byte_at(&mut self, index: usize) -> Result<Option<u8>, S::Error> {
        while index >= self.unread().len() && !self.eof {
            self.read_more()?;
        }
        Ok(self.unread().get(index).copied())
    }

    fn commit(&mut self, length: usize) {
        self.buffer_start += length;
        self.offset = self
            .offset
            .checked_add(length as u64)
            .expect("a source byte offset exceeds u64");
    }

    fn buffered_end(&self) -> u64 {
        self.offset
            .checked_add(self.unread().len() as u64)
            .expect("a source byte offset exceeds u64")
    }

    fn invalid_input_length(&mut self) -> Result<(usize, bool), S::Error> {
        let first = self
            .byte_at(0)?
            .expect("invalid input is only measured when one byte is available");
        let expected = match first {
            0x00..=0x7f => 1,
            0xc2..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf4 => 4,
            _ => 1,
        };
        for index in 1..expected {
            if self.byte_at(index)?.is_none() {
                break;
            }
        }
        let available = expected.min(self.unread().len());
        match std::str::from_utf8(&self.unread()[..available]) {
            Ok(_) => Ok((available, false)),
            Err(error) => Ok((error.error_len().unwrap_or(available), true)),
        }
    }
}

impl<T: Lexer, S: Source> Scanner<T, S> {
    fn select_rule(&mut self, mode: usize) -> Result<Option<(usize, usize)>, S::Error> {
        let mut state = T::start_state(mode).expect("the mode stack contains a generated mode");
        let mut latest = T::accepting_rule(state).map(|rule| (rule, 0));
        let mut index = 0;

        while let Some(byte) = self.byte_at(index)? {
            let Some(next) = T::next_state(state, byte) else {
                break;
            };
            state = next;
            index += 1;
            if let Some(rule) = T::accepting_rule(state) {
                latest = Some((rule, index));
            }
        }
        Ok(latest)
    }

    fn apply_transition(&mut self, transition: Transition, start: u64) {
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
    }
}

impl<T: Lexer, S: Source> Iterator for Scanner<T, S> {
    type Item = Result<Spanned<T>, ScanError<S::Error>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        loop {
            let mode = self
                .modes
                .last()
                .expect("the mode stack contains INITIAL")
                .id;
            let selected = match self.select_rule(mode) {
                Ok(selected) => selected,
                Err(error) => {
                    self.finished = true;
                    return Some(Err(ScanError::Source {
                        offset: self.buffered_end(),
                        error,
                    }));
                }
            };
            let Some((rule, consumed)) = selected else {
                if self.eof && self.unread().is_empty() {
                    self.finished = true;
                    if let Some(frame) = self.modes.get(1) {
                        return Some(Err(ScanError::UnterminatedMode {
                            span: Span::new(frame.opened_at, self.offset),
                            mode: T::mode_name(frame.id),
                        }));
                    }
                    return None;
                }
                let (length, invalid_utf8) = match self.invalid_input_length() {
                    Ok(result) => result,
                    Err(error) => {
                        self.finished = true;
                        return Some(Err(ScanError::Source {
                            offset: self.buffered_end(),
                            error,
                        }));
                    }
                };
                let start = self.offset;
                self.commit(length);
                let span = Span::new(start, self.offset);
                return Some(Err(if invalid_utf8 {
                    ScanError::InvalidUtf8 { span }
                } else {
                    ScanError::Unrecognized { span }
                }));
            };

            let start = self.offset;
            let result = {
                let text = std::str::from_utf8(&self.unread()[..consumed])
                    .expect("a generated lexer only accepts valid UTF-8");
                T::run_action(rule, text)
            };
            self.commit(consumed);
            let (token, transition) = match result {
                Ok(result) => result,
                Err(error) => {
                    return Some(Err(ScanError::InvalidPayload {
                        span: Span::new(start, self.offset),
                        message: error.to_string(),
                    }));
                }
            };
            self.apply_transition(transition, start);
            if let Some(token) = token {
                return Some(Ok(Spanned {
                    token,
                    span: Span::new(start, self.offset),
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
    /// Returns the DFA start state for a generated start condition.
    #[doc(hidden)]
    fn start_state(mode: usize) -> Option<usize>;

    /// Advances the generated DFA by one byte.
    #[doc(hidden)]
    fn next_state(state: usize, byte: u8) -> Option<usize>;

    /// Returns the winning rule at an accepting DFA state.
    #[doc(hidden)]
    fn accepting_rule(state: usize) -> Option<usize>;

    /// Executes a generated rule action for an accepted lexeme.
    #[doc(hidden)]
    fn run_action(rule: usize, text: &str) -> Result<(Option<Self>, Transition), PayloadError>;

    /// Scans one token or skipped rule from the start of a string.
    ///
    /// `None` means no rule accepts a prefix. A successful result contains a
    /// skipped rule or token plus its consumed byte length. An error means a
    /// winning rule could not convert its payload.
    #[doc(hidden)]
    fn scan_one(input: &str, mode: usize) -> Option<RuleScan<Self>> {
        let mut state = Self::start_state(mode)?;
        let mut latest = Self::accepting_rule(state).map(|rule| (rule, 0));

        for (index, &byte) in input.as_bytes().iter().enumerate() {
            let Some(next) = Self::next_state(state, byte) else {
                break;
            };
            state = next;
            if let Some(rule) = Self::accepting_rule(state) {
                latest = Some((rule, index + 1));
            }
        }

        let (rule, length) = latest?;
        Some(
            Self::run_action(rule, &input[..length])
                .map(|(token, transition)| (token, length, transition))
                .map_err(|error| (error, length)),
        )
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
    fn scanner(input: &str) -> Scanner<Self, Slice<'_>> {
        Self::scanner_from(Slice::from(input))
    }

    /// Creates a recoverable scanner over any byte source.
    ///
    /// Use [`Slice`] for in-memory input, [`Reader`] for [`std::io::Read`]
    /// values such as files, or implement [`Source`] for custom storage.
    fn scanner_from<S: Source>(source: S) -> Scanner<Self, S> {
        Scanner::new(source)
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
