//! A deterministic finite automaton, its builder, and its execution.

mod automaton;
mod builder;
mod execution;

#[allow(unused_imports)]
pub use self::{automaton::Dfa, builder::DfaBuilder, execution::DfaExecution};
