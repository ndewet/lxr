mod ast;
mod charset;
mod cursor;
mod error;
mod escape;
mod parser;

pub use ast::{Node, Repetitions};
pub use charset::CharSet;
pub use error::{ParseError, ParseErrorKind};

use parser::RegexParser;

/// Parses a regular expression into its syntax tree.
///
/// # Errors
///
/// Returns a [`ParseError`] naming the position at which `pattern` stopped
/// being a valid regular expression.
pub fn parse(pattern: &str) -> Result<Node, ParseError> {
    RegexParser::new(pattern).parse()
}
