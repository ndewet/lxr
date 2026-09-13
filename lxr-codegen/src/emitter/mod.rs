//! Emits Rust source for minimized lexer automata.
//!
//! This module owns generated matcher layout and source rendering. Automaton
//! construction and minimization remain in [`crate::automata`].

#[allow(clippy::module_inception)]
mod emitter;

pub(crate) use self::emitter::emit;
