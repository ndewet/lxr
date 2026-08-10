mod ast;
mod charset;
mod cursor;
mod error;
mod escape;
mod parser;

pub use ast::{Node, Repetitions};
pub use charset::CharSet;
pub use error::ParseError;
use std::str::FromStr;

use parser::RegexParser;

/// Parses a regular expression into its syntax tree.
///
/// # Errors
///
/// Returns a [`ParseError`] naming the position at which `pattern` stopped
/// being a valid regular expression.
impl FromStr for Node {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        RegexParser::new(s).parse()
    }
}
