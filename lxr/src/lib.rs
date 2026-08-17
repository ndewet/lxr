//! The runtime of a lexer that lxr generates.
//!
//! A derive macro reads an enum of tokens, and it emits the tables of a
//! deterministic automaton. This crate holds the scan that reads those tables.
//! Thus a user crate compiles the runtime alone, and it does not compile the
//! regex parser or the automata.
//!
//! `lxr-codegen` holds the parser, the automata, and the emitter.
