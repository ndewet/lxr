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

mod encoding;
mod nfa;

mod adjacency;
mod dfa;
mod error;
mod id;
mod label;
mod state_set;
mod table;
#[cfg(test)]
mod testing;
mod transition;

pub(crate) use self::{error::BuildError, id::StateId, label::Label, transition::Transition};
