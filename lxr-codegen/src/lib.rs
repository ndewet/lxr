//! Builds lexer data for the lxr derive macro.
//!
//! This crate parses lexer patterns and builds automata for generated matchers.
//! Lexer authors use the derive API instead of this crate.
//!
//! Functions return a [`Result`] for invalid lexer input. A panic reports a
//! defect in lxr.

#![cfg_attr(test, allow(clippy::unwrap_used))]

// The derive entry point will use these modules when the lexer pipeline exists.
#[allow(dead_code, unused_imports)]
mod automata;
#[allow(dead_code, unused_imports)]
pub(crate) mod emitter;
#[allow(dead_code)]
mod lexer;
#[allow(dead_code, unused_imports)]
mod regex;
