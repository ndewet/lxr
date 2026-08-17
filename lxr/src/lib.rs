//! The runtime of a lexer that lxr generates.
//!
//! A derive macro reads an enum of tokens, and it emits the tables of a deterministic automaton.
//! This crate holds the scan that reads those tables. Thus a user crate compiles the runtime alone,
//! and it does not compile the regex parser or the automata.
//!
//! [`Lexer`] is the trait that the macro implements. [`Lexer::scan`] starts a [`Scan`], which gives
//! one token at a time and reports each character that no rule matches.
//!
//! `lxr-codegen` holds the parser, the automata, and the emitter.
//!
//! Each table comes from lxr, thus a function of this crate panics for a table that disagrees with
//! itself. A [`ScanError`] reports the input, and not the lexer.

mod action;
mod error;
mod lexer;
mod scan;
mod tables;

pub use self::{action::Action, error::ScanError, lexer::Lexer, scan::Scan, tables::Tables};
