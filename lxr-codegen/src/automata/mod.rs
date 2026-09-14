//! Builds the finite automata of the lexer pipeline.
//!
//! A [`Label`] defines the alphabet. An accept value identifies a rule but does
//! not define rule precedence.
//!
//! The [`nfa`] module contains Thompson construction. The [`dfa`] module holds
//! deterministic automata for subset construction and minimization.
//!
//! Invalid state identifiers report an lxr defect and cause a panic. Capacity
//! limits come from lexer input and return a [`BuildError`].

pub(crate) mod encoding;
pub(crate) mod nfa;

mod adjacency;
pub(crate) mod dfa;
mod error;
mod id;
mod label;
#[cfg(test)]
mod language_tests;
mod state_set;
mod table;
#[cfg(test)]
mod testing;
mod transition;

pub(crate) use self::error::BuildError;
pub(crate) use self::{id::StateId, label::Label, transition::Transition};
