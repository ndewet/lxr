//! A nondeterministic finite automaton, its builder, and its simulator.

mod automaton;
mod builder;
mod id;
#[cfg(test)]
mod reference;
mod simulation;
mod transition;

#[allow(unused_imports)]
pub use self::{
    automaton::Nfa,
    builder::NfaBuilder,
    id::{StartId, StateId},
    simulation::Simulator,
    transition::Transition,
};
