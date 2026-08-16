use std::fmt::{Display, Formatter, Result};

/// A failure to parse a regular expression.
///
/// The error gives the kind of the failure. It also gives the position in the
/// pattern at which the parser stopped. The position is a byte offset from the
/// start of the pattern.
///
/// # Examples
///
/// ```
/// use lxr::regex::Node;
///
/// let error = "a(b".parse::<Node>().unwrap_err();
/// assert_eq!(error.position, 1);
/// assert_eq!(error.to_string(), "unclosed '(' at position 1");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParseError {
    /// The byte offset in the pattern at which the parser stopped.
    pub position: usize,
    /// The kind of the failure.
    pub kind: ParseErrorKind,
}

/// The kind of failure that a [`ParseError`] reports.
///
/// A variant with the name `Unsupported...` shows a construction that this
/// parser does not accept. Each other variant shows a fault in the pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseErrorKind {
    /// The pattern stops before the expression is complete.
    UnexpectedEnd,
    /// The pattern has a character that the parser cannot use at this
    /// position.
    UnexpectedCharacter(char),
    /// The parser needs one specific character. The pattern has a different
    /// character, or the pattern stops.
    Expected {
        /// The character that the parser needs.
        wanted: char,
        /// The character that the pattern has. The end of the pattern gives
        /// `None`.
        found: Option<char>,
    },
    /// A quantifier has no expression before it.
    NothingToRepeat(char),
    /// A quantifier comes immediately after another quantifier.
    RepeatedQuantifier(char),
    /// A range in a character class has a low end above its high end.
    InvertedRange {
        /// The low end of the range.
        low: char,
        /// The high end of the range.
        high: char,
    },
    /// A repetition has a minimum count above its maximum count.
    InvertedRepetition {
        /// The minimum count of the repetition.
        minimum: usize,
        /// The maximum count of the repetition.
        maximum: usize,
    },
    /// A repetition count is too large.
    RepetitionTooLarge,
    /// A group starts with `(`, but the pattern has no `)` for it.
    UnclosedGroup,
    /// The pattern has a `)` with no `(` before it.
    UnmatchedCloseParenthesis,
    /// A character class starts with `[`, but the pattern has no `]` for it.
    UnclosedClass,
    /// A character class holds no characters, thus it matches nothing.
    EmptyClass,
    /// A class escape such as `\d` is an end of a range. An end of a range
    /// must be one character.
    ClassEscapeInRange(char),
    /// An escape sequence has a character that the parser does not know.
    UnknownEscape(char),
    /// An escape gives a value that is not a character. A surrogate and a
    /// value above `U+10FFFF` are not characters.
    InvalidCodePoint(u64),
    /// The groups in the pattern nest deeper than the limit.
    NestingTooDeep(usize),
    /// The pattern has an anchor, for example `^` or `$`. This parser does not
    /// support anchors.
    UnsupportedAnchor(char),
    /// The pattern has a `(?` group, for example a non-capturing group. This
    /// parser does not support these groups.
    UnsupportedGroup,
    /// The pattern has a POSIX character class, for example `[:alpha:]`. This
    /// parser does not support POSIX character classes.
    UnsupportedPosixClass,
    /// The pattern has an octal escape, for example `\101`. This parser does
    /// not support octal escapes.
    UnsupportedOctalEscape,
    /// The pattern has a backreference, for example `\1`. This parser does not
    /// support backreferences.
    UnsupportedBackreference,
}

impl ParseErrorKind {
    pub(crate) fn at(self, position: usize) -> ParseError {
        ParseError {
            position,
            kind: self,
        }
    }
}

impl Display for ParseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        write!(formatter, "{} at position {}", self.kind, self.position)
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
