//! The runtime of a lexer that lxr generates.
//!
//! A derive macro reads an enum of tokens, and it emits the tables of a deterministic automaton.
//! This crate holds the scan that reads those tables. Thus a user crate compiles the runtime alone,
//! and it does not compile the regex parser or the automata.
//!
//! [`Lexer`] is the trait that the macro implements. [`Lexer::scan`] starts a [`Scan`], which gives
//! one token at a time and reports each character that no rule matches.
//! [`Scan::located`] gives the place of each token with the token.
//!
//! [`syntax`] holds the reference of the rules: each attribute, the sequence of the rules, the
//! pattern language, and each limit.
//!
//! `lxr-codegen` holds the parser, the automata, and the emitter.
//!
//! Each table comes from lxr, thus a function of this crate panics for a table that disagrees with
//! itself. A [`ScanError`] reports the input, and not the lexer.

mod action;
mod error;
mod lexer;
mod located;
mod scan;
mod tables;

pub mod syntax;

pub use self::{
    action::Action,
    error::{ScanError, ScanErrorKind},
    lexer::Lexer,
    located::{Located, Locations},
    scan::Scan,
    tables::Tables,
};

/// Derives a lexer from an enum of tokens.
///
/// The macro reads one `#[lxr(...)]` attribute for each rule, and it implements [`Lexer`]. The
/// `derive` feature holds it, and that feature is on by default.
///
/// A rule holds one pattern. `token` matches a literal, `regex` matches a regular expression, and
/// `skip` reads the match and gives no token. Write `skip` on the enum.
///
/// `in` gives the start conditions of a rule, and `go` changes the condition after the match. Write
/// `condition` on the enum to name the condition at which the scan begins. The type of the
/// conditions is that path without its last segment.
///
/// The longest match wins, and the earliest rule wins a tie.
///
/// A variant that holds one unnamed field carries a value. The field takes the text of the match
/// through [`FromStr`](std::str::FromStr), thus `Name(String)` holds the text and `Int(u64)` holds
/// the number. A text that the field does not hold gives a [`ScanError`] of the kind
/// [`Value`](ScanErrorKind::Value).
///
/// [`syntax`] holds each attribute, the pattern language, and each limit.
///
/// # Examples
///
/// ```
/// use lxr::Lexer;
///
/// #[derive(Clone, Copy, Debug, PartialEq)]
/// enum Context {
///     Code,
///     Text,
/// }
///
/// #[derive(Debug, PartialEq, Lexer)]
/// #[lxr(condition = Context::Code)]
/// #[lxr(skip = "[ \t\n]+")]
/// enum Token {
///     #[lxr(token = "let")]
///     Let,
///     #[lxr(regex = "[a-z][a-z0-9]*")]
///     Word(String),
///     #[lxr(token = "\"", go = Context::Text)]
///     Quote,
///     #[lxr(regex = "[^\"]+", in = [Context::Text])]
///     Text,
///     #[lxr(token = "\"", in = [Context::Text], go = Context::Code)]
///     End,
/// }
///
/// let tokens: Vec<_> = Token::scan("let a \"one two\"")
///     .map(|found| found.expect("each character belongs to a token"))
///     .collect();
///
/// assert_eq!(
///     tokens,
///     vec![
///         Token::Let,
///         Token::Word("a".to_owned()),
///         Token::Quote,
///         Token::Text,
///         Token::End,
///     ]
/// );
/// ```
#[cfg(feature = "derive")]
pub use lxr_derive::Lexer;
