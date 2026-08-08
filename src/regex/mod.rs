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

pub fn parse(pattern: &str) -> Result<Node, ParseError> {
    RegexParser::new(pattern).parse()
}
