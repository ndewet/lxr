//! Builds the automaton of a lexer, and emits its source.
//!
//! The crate reads a pattern as a regular expression, then it builds an
//! automaton that scans bytes. A derive macro calls it at compile time, thus
//! this crate builds for the host and no user crate holds it.
//!
//! [`lxr`] holds the runtime that the emitted source calls.
//!
//! A function that reads what a lexer author wrote gives a [`Result`]. A panic
//! reports a defect in lxr. `CONTRIBUTING.md` holds the standard.
//!
//! [`lxr`]: https://docs.rs/lxr

#![cfg_attr(test, allow(clippy::unwrap_used))]

mod automata;
mod compiler;
pub mod regex;
mod table;
