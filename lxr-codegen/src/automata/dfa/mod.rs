//! Represents deterministic finite automata.
//!
//! A [`Dfa`] has disjoint outgoing labels at every state. Missing transitions
//! move to an implicit dead state. [`subset`] constructs a DFA from an NFA.

mod automaton;
mod builder;
mod minimization;
pub(crate) mod subset;

pub(crate) use self::{automaton::Dfa, builder::Builder};
