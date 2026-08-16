//! A deterministic finite automaton, and its execution.

mod automaton;
mod execution;

#[allow(unused_imports)]
pub use self::{automaton::DeterministicFiniteAutomaton, execution::DeterministicExecution};
