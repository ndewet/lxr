//! The front of a lexer generator: a regular expression, and the automaton of
//! that expression.
//!
//! [`regex`] parses a pattern into a syntax tree.
//! [`automata`] holds finite automata, UTF-8 encoding, and Thompson
//! construction. The construction makes an NFA from a syntax tree.
//!
//! A function that reads what a lexer author wrote gives a [`Result`]. A panic
//! reports a defect in lxr.

#![cfg_attr(test, allow(clippy::unwrap_used))]

pub mod automata;
pub mod regex;
