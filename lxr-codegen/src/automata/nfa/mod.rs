//! The nondeterministic finite automaton, and the construction that makes it.
//!
//! [`Nfa`] is the automaton. A state can hold two
//! transitions of the same label, and it can hold an epsilon transition. Thus
//! one scan is in a set of states, and [`Execution`] holds that set.
//! [`Matcher`] applies longest-match selection to an execution.
//!
//! [`thompson`] is the construction. It walks a syntax tree of the
//! [`regex`](crate::regex) module, and it gives one [`Fragment`] for each
//! node. [`Builder`] holds the states that the construction makes.
//!
//! The automaton reads the labels that the selected
//! [`Encoding`](crate::automata::encoding::Encoding) makes.

mod automaton;
mod builder;
mod closure;
mod execution;
mod fragment;
mod matcher;
pub mod thompson;

pub use self::{
    automaton::Nfa,
    builder::Builder,
    execution::Execution,
    fragment::Fragment,
    matcher::{Match, Matcher},
};

pub(crate) use self::closure::epsilon_closure;
