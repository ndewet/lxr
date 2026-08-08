use std::fmt::{Display, Formatter, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub position: usize,
    pub kind: ParseErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    UnexpectedEnd,
    UnexpectedCharacter(char),
    Expected { wanted: char, found: Option<char> },
    NothingToRepeat(char),
    RepeatedQuantifier(char),
    InvertedRange { low: char, high: char },
    InvertedRepetition { minimum: usize, maximum: usize },
    RepetitionTooLarge,
    UnclosedGroup,
    UnmatchedCloseParenthesis,
    UnclosedClass,
    EmptyClass,
    ClassEscapeInRange(char),
    UnknownEscape(char),
    InvalidCodePoint(u64),
    NestingTooDeep(usize),
    UnsupportedAnchor(char),
    UnsupportedGroup,
    UnsupportedPosixClass,
    UnsupportedOctalEscape,
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
