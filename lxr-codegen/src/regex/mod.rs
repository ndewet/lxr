//! Parses a regular expression into a syntax tree.
//!
//! [`Expression`] is the tree, and [`CharSet`] is the set that a
//! [`Class`](Expression::Class) leaf matches. To make a tree from a pattern, use
//! [`FromStr`]. A pattern that the parser cannot read gives a [`ParseError`].
//! [`Quantifier`] specifies the repetition count of an expression.

mod charset;
mod cursor;
mod error;
mod escape;
mod expression;
mod parser;
mod quantifier;

pub use charset::CharSet;
pub use error::{ParseError, ParseErrorKind};
pub use expression::Expression;
pub use quantifier::{Quantifier, QuantifierRangeError};
use std::str::FromStr;

use parser::Parser;

impl FromStr for Expression {
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
    /// use lxr_codegen::regex::{CharSet, Expression};
    ///
    /// let node: Expression = "a".parse().unwrap();
    /// assert_eq!(node, Expression::Class(CharSet::single('a')));
    ///
    /// assert!("a(b".parse::<Expression>().is_err());
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Parser::new(s).parse()
    }
}
