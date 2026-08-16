//! A lexer generator.
//!
//! The crate reads a pattern as a regular expression, then it builds an
//! automaton that scans bytes. The [`regex`] module holds the parser and the
//! syntax tree of a pattern.
//!
//! # Errors and panics
//!
//! A function that reads what a lexer author wrote gives a [`Result`]. A panic
//! reports a defect in lxr, and not a fault in the input. `CONTRIBUTING.md`
//! holds the full standard.

#![cfg_attr(test, allow(clippy::unwrap_used))]

mod automata;
mod compiler;
pub mod regex;
