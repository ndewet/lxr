//! Parses lexer regex patterns into syntax trees.
//!
//! An [`Expression`] contains [`CharSet`] leaves and [`Quantifier`] nodes.
//! Invalid patterns return a [`ParseError`].

mod charset;
mod cursor;
mod error;
mod escape;
mod expression;
mod parser;
mod quantifier;

pub(crate) use charset::CharSet;
pub(crate) use error::{ParseError, ParseErrorKind};
pub(crate) use expression::Expression;
pub(crate) use quantifier::{Quantifier, QuantifierRangeError};
use std::str::FromStr;

use parser::Parser;

impl FromStr for Expression {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Parser::new(s).parse()
    }
}
