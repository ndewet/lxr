//! Compiles the rules of a lexer into an automaton.
//!
//! The module reads a [`Node`](crate::regex::Node) tree from the
//! [`regex`](crate::regex) module. It gives a [nondeterministic automaton] to
//! the [`automata`](crate::automata) module. Thus the alphabet of the lexer
//! lives here, and not in either of the other two modules.
//!
//! [`Lexicon`] is the input. It holds the rules, and it hands out the
//! identifier of each start condition. [`compile`](compile()) is the entry
//! point. It
//! joins three parts:
//!
//! 1. [`thompson::fragment`] makes the states of each operator of a pattern.
//!    Only [`compile`](compile()) calls it.
//! 2. An [`Alphabet`] makes the states of each character set.
//! 3. [`Bytes`] is the alphabet of a lexer that reads UTF-8. It lowers each
//!    character set with [`utf8::lower`].
//!
//! [`Lexicon::rule`] and [`compile`](compile()) read what a lexer author wrote.
//! Thus each one gives a [`BuildError`], and neither one panics.
//!
//! [nondeterministic automaton]:
//!     crate::automata::NondeterministicFiniteAutomaton

#![allow(dead_code)]

mod accepts;
mod alphabet;
mod bytes;
mod compile;
mod error;
mod fragment;
mod lexicon;
mod rule;
mod thompson;
mod utf8;

#[allow(unused_imports)]
pub use self::{
    accepts::Accepts,
    alphabet::Alphabet,
    bytes::Bytes,
    compile::compile,
    error::{BuildError, BuildErrorKind},
    fragment::Fragment,
    lexicon::{Lexicon, MAX_PATTERN_SIZE},
    utf8::{ByteRange, ByteSequence},
};
