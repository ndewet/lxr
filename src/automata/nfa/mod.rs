//! A nondeterministic finite automaton, its builder, and its execution.

mod automaton;
mod builder;
mod execution;
#[cfg(test)]
mod reference;

#[allow(unused_imports)]
pub use self::{automaton::Nfa, builder::NfaBuilder, execution::NfaExecution};
