mod ast;
mod charset;
mod cursor;
mod error;
mod escape;
mod parser;

pub use ast::{RegexNode, Repetitions};
pub use charset::CharSet;
pub use error::{RegexParseError, RegexParseErrorKind};

use parser::RegexParser;

pub fn parse(pattern: &str) -> Result<RegexNode, RegexParseError> {
    RegexParser::new(pattern).parse()
}
