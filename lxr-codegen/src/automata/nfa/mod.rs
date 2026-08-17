//! A nondeterministic finite automaton, its builder, and its execution.

mod automaton;
mod builder;
mod execution;

#[allow(unused_imports)]
pub use self::{
    automaton::NondeterministicFiniteAutomaton, builder::NfaBuilder,
    execution::NondeterministicExecution,
};
