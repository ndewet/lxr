//! Builds the automaton of a lexer, and emits its source.
//!
//! The crate reads a pattern as a regular expression, then it builds an
//! automaton that scans bytes. A derive macro calls it at compile time, thus
//! this crate builds for the host and no user crate holds it.
//!
//! [`generate`](generate()) is the entry point. It reads a [`Specification`],
//! and it gives the source of the `impl` that the macro places in the crate of
//! the lexer author.
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
mod emit;
mod generate;
pub mod regex;
mod table;

pub use self::generate::{
    Conditions, GenerateError, GenerateErrorKind, Pattern, Rule, Specification, generate,
};
