//! Builds and executes nondeterministic finite automata.
//!
//! An [`Nfa`] permits overlapping labels and epsilon transitions. An
//! [`Execution`] tracks its active state set. A [`Matcher`] selects the longest
//! match.
//!
//! Thompson construction creates one [`Fragment`] for each regex node.

mod automaton;
mod builder;
mod closure;
#[cfg(test)]
mod execution;
mod fragment;
#[cfg(test)]
mod matcher;
pub(crate) mod thompson;

pub(crate) use self::{automaton::Nfa, builder::Builder};
#[cfg(test)]
pub(crate) use self::{execution::Execution, matcher::Matcher};

pub(crate) use self::closure::epsilon_closure;
