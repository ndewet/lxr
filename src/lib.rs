//! A lexer generator.
//!
//! The crate reads a pattern as a regular expression, then it builds an
//! automaton that scans bytes.
//!
//! A function that reads what a lexer author wrote gives a [`Result`]. A panic
//! reports a defect in lxr. `CONTRIBUTING.md` holds the standard.

#![cfg_attr(test, allow(clippy::unwrap_used))]

mod automata;
mod compiler;
pub mod regex;
