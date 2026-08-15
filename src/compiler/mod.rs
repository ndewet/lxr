//! Compiles the rules of a lexer into an automaton.
//!
//! The module reads a [`Node`](crate::regex::Node) tree from the
//! [`regex`](crate::regex) module. It gives an [`Nfa`](crate::automata::Nfa)
//! to the [`automata`](crate::automata) module. Thus the alphabet of the
//! lexer lives here, and not in either of the other two modules.
//!
//! [`Lexicon`] is the input. It holds the rules, and it hands out the
//! identifier of each start condition. [`compile`] is the entry point. It
//! joins three parts:
//!
//! 1. [`thompson::fragment`] makes the states of each operator of a pattern.
//!    Only [`compile`] calls it.
//! 2. An [`Alphabet`] makes the states of each character set.
//! 3. [`Bytes`] is the alphabet of a lexer that reads UTF-8. It lowers each
//!    character set with [`utf8::lower`].

#![allow(dead_code)]

mod alphabet;
mod bytes;
mod compile;
mod fragment;
mod lexicon;
mod rule;
mod thompson;
mod utf8;

#[allow(unused_imports)]
pub use self::{
    alphabet::Alphabet,
    bytes::Bytes,
    compile::compile,
    fragment::Fragment,
    lexicon::Lexicon,
    utf8::{ByteRange, ByteSequence},
};
