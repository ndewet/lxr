use std::fmt::{Display, Formatter, Result};
use std::ops::Range;

/// Reports invalid or unsupported regex syntax.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) struct ParseError {
    /// The byte range at fault.
    ///
    /// A failure at the end of the pattern has an empty range.
    pub(crate) span: Range<usize>,
    /// The kind of the failure.
    pub(crate) kind: ParseErrorKind,
}

/// Identifies the cause of a [`ParseError`].
///
/// An `Unsupported` variant identifies valid regex syntax that lxr does not support.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) enum ParseErrorKind {
    /// The pattern stops before the expression is complete.
    UnexpectedEnd,
    /// A character is invalid at the current position.
    UnexpectedCharacter(char),
    /// A required character is missing or different.
    Expected {
        /// The required character.
        wanted: char,
        /// The found character, or `None` at the end.
        found: Option<char>,
    },
    /// A quantifier has no expression before it.
    NothingToRepeat(char),
    /// A quantifier comes immediately after another quantifier.
    RepeatedQuantifier(char),
    /// A character range has its high end first.
    InvertedRange {
        /// The requested low end.
        low: char,
        /// The requested high end.
        high: char,
    },
    /// A repetition has its maximum count first.
    InvertedRepetition {
        /// The requested minimum.
        minimum: usize,
        /// The requested maximum.
        maximum: usize,
    },
    /// A repetition count is too large.
    RepetitionTooLarge,
    /// A group has no closing `)`.
    UnclosedGroup,
    /// A `)` has no opening `(`.
    UnmatchedCloseParenthesis,
    /// A character class has no closing `]`.
    UnclosedClass,
    /// A character class matches no character.
    EmptyClass,
    /// A multi-character class escape is a range end.
    ClassEscapeInRange(char),
    /// An escape sequence is unknown.
    UnknownEscape(char),
    /// An escape names a surrogate or a value above `U+10FFFF`.
    InvalidCodePoint(u64),
    /// The group depth exceeds the parser limit.
    NestingTooDeep(usize),
    /// The pattern contains an unsupported anchor.
    UnsupportedAnchor(char),
    /// The pattern contains an unsupported `(?` group.
    UnsupportedGroup,
    /// The pattern contains an unsupported POSIX character class.
    UnsupportedPosixClass,
    /// The pattern contains an unsupported octal escape.
    UnsupportedOctalEscape,
    /// The pattern contains an unsupported backreference.
    UnsupportedBackreference,
}

impl ParseErrorKind {
    /// Creates an error for `span`.
    pub(crate) fn spanning(self, span: Range<usize>) -> ParseError {
        ParseError { span, kind: self }
    }

    #[cfg(test)]
    pub(crate) fn at(self, position: usize) -> ParseError {
        self.spanning(position..position)
    }

    /// Returns a correction when lxr can suggest one.
    pub(crate) fn help(&self) -> Option<&'static str> {
        Some(match self {
            Self::UnexpectedEnd | Self::UnexpectedCharacter(_) | Self::Expected { .. } => {
                return None;
            }
            Self::NothingToRepeat(_) => "Put an expression before the quantifier.",
            Self::RepeatedQuantifier(_) => "Put the expression in a group, for example `(a*)?`.",
            Self::InvertedRange { .. } => "Write the low end first, for example `a-z`.",
            Self::InvertedRepetition { .. } => "Write the minimum first, for example `{2,5}`.",
            Self::RepetitionTooLarge => "Write a count of 65535 or below.",
            Self::UnclosedGroup => "Add a `)`.",
            Self::UnmatchedCloseParenthesis => {
                "Add a `(`, or write `\\)` for a literal parenthesis."
            }
            Self::UnclosedClass => "Add a `]`.",
            Self::EmptyClass => "The class matches no character, thus no input matches the rule.",
            Self::ClassEscapeInRange(_) => "Write one character at each end of the range.",
            Self::UnknownEscape(_) => "Write `\\\\` for a literal backslash.",
            Self::InvalidCodePoint(_) => {
                "Write a value from 0 to 10FFFF, and not a value from D800 to DFFF."
            }
            Self::NestingTooDeep(_) => "Make the pattern flat, or divide the rule.",
            Self::UnsupportedAnchor(_) => {
                "A lexer matches at the position of the scan, thus an anchor is not needed."
            }
            Self::UnsupportedGroup => {
                "Write a plain group `(...)`. A group of this parser captures nothing."
            }
            Self::UnsupportedPosixClass => {
                "Write the characters, for example `[a-zA-Z]`, or write `\\w`."
            }
            Self::UnsupportedOctalEscape => {
                "Write a hexadecimal escape, for example `\\x41` or `\\x{41}`."
            }
            Self::UnsupportedBackreference => {
                "A lexer reads a regular language, thus it holds no backreference. \
                 For the character 65, write `\\x41`."
            }
        })
    }
}

impl Display for ParseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        write!(formatter, "{} at position {}", self.kind, self.span.start)
    }
}

impl std::error::Error for ParseError {}

impl Display for ParseErrorKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        match self {
            Self::UnexpectedEnd => write!(formatter, "unexpected end of pattern"),
            Self::UnexpectedCharacter(found) => {
                write!(formatter, "unexpected '{}'", found.escape_debug())
            }
            Self::Expected {
                wanted,
                found: Some(found),
            } => write!(
                formatter,
                "expected '{}', found '{}'",
                wanted.escape_debug(),
                found.escape_debug()
            ),
            Self::Expected {
                wanted,
                found: None,
            } => write!(
                formatter,
                "expected '{}', found end of pattern",
                wanted.escape_debug()
            ),
            Self::NothingToRepeat(quantifier) => {
                write!(formatter, "'{quantifier}' has nothing to repeat")
            }
            Self::RepeatedQuantifier(quantifier) => {
                write!(formatter, "repeated quantifier '{quantifier}'")
            }
            Self::InvertedRange { low, high } => write!(
                formatter,
                "invalid range '{}-{}'",
                low.escape_debug(),
                high.escape_debug()
            ),
            Self::InvertedRepetition { minimum, maximum } => {
                write!(formatter, "invalid repetition {{{minimum},{maximum}}}")
            }
            Self::RepetitionTooLarge => write!(formatter, "repetition count is too large"),
            Self::UnclosedGroup => write!(formatter, "unclosed '('"),
            Self::UnmatchedCloseParenthesis => write!(formatter, "unmatched ')'"),
            Self::UnclosedClass => write!(formatter, "unclosed '['"),
            Self::EmptyClass => write!(formatter, "character class matches nothing"),
            Self::ClassEscapeInRange(escape) => {
                write!(formatter, "'\\{escape}' cannot be a range endpoint")
            }
            Self::UnknownEscape(found) => {
                write!(formatter, "unknown escape '\\{}'", found.escape_debug())
            }
            Self::InvalidCodePoint(value) => {
                write!(formatter, "invalid code point U+{value:04X}")
            }
            Self::NestingTooDeep(limit) => {
                write!(formatter, "groups nest more than {limit} deep")
            }
            Self::UnsupportedAnchor(anchor) => {
                write!(formatter, "anchor '{anchor}' is not supported")
            }
            Self::UnsupportedGroup => write!(formatter, "'(?' groups are not supported"),
            Self::UnsupportedPosixClass => {
                write!(formatter, "POSIX character classes are not supported")
            }
            Self::UnsupportedOctalEscape => write!(formatter, "octal escapes are not supported"),
            Self::UnsupportedBackreference => write!(formatter, "backreferences are not supported"),
        }
    }
}
