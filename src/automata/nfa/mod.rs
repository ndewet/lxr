#![allow(dead_code)]

mod automaton;
mod builder;
#[cfg(test)]
mod reference;
mod simulation;
mod state;

#[allow(unused_imports)]
pub use self::{
    automaton::{Nfa, StartId},
    builder::NfaBuilder,
    simulation::Simulator,
    state::{State, StateId},
};
