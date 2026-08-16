//! The automata of the pipeline, and the parts that they share.
//!
//! An automaton in this module holds no lexer concept and no alphabet. A label of type `L` gives
//! the condition on a transition, and an accept of type `A` gives the meaning of a state that
//! accepts. The caller selects both types.
//!
//! Each automaton implements [`Automaton`]. Thus [`longest_match`] scans a nondeterministic
//! automaton and a deterministic automaton with the same code.
//!
//! Each identifier comes from lxr, and not from a lexer author. Thus a function panics for an
//! identifier that its automaton does not hold. A full automaton gives an [`Overflow`].

#![allow(dead_code)]

mod arena;
mod arena_builder;
mod automaton;
mod execution;
mod id;
mod label;
mod nfa;
mod overflow;
#[cfg(test)]
mod reference;
mod scan;
mod transition;

#[allow(unused_imports)]
pub use self::{
    arena::Arena,
    arena_builder::ArenaBuilder,
    automaton::Automaton,
    execution::Execution,
    id::{StartId, StateId},
    label::Label,
    nfa::{Nfa, NfaBuilder, NfaExecution},
    overflow::{Overflow, Part},
    scan::{Match, longest_match},
    transition::Transition,
};
