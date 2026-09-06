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
mod execution;
mod fragment;
mod matcher;
mod thompson;

pub(crate) use self::{
    automaton::Nfa,
    builder::Builder,
    execution::Execution,
    fragment::Fragment,
    matcher::{Match, Matcher},
};

pub(crate) use self::closure::epsilon_closure;
