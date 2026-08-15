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

impl FromStr for Node {
    type Err = ParseError;

    /// Parses a regular expression into its syntax tree.
    ///
    /// # Errors
    ///
    /// This function returns a [`ParseError`] if `s` is not a valid regular
    /// expression. The error gives the position at which the parser stopped.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::regex::{CharSet, Node};
    ///
    /// let node: Node = "a".parse().unwrap();
    /// assert_eq!(node, Node::Class(CharSet::single('a')));
    ///
    /// assert!("a(b".parse::<Node>().is_err());
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        RegexParser::new(s).parse()
    }
}
