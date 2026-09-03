//! The finite automata of the lexer pipeline.
//!
//! An automaton in this module holds no lexer concept. A label of type `L` gives the condition on
//! a transition, and the caller selects that type. [`encoding`] gives the labels of a lexer that
//! reads UTF-8.
//!
//! An automaton knows which states accept. It stores each accept value without
//! interpreting it. A lexer can use a rule identifier as that value, and
//! [`longest_match`](nfa::Matcher::longest_match) asks the caller to select
//! among applicable values. Thus no rule of precedence lives here.
//!
//! [`nfa`] holds the nondeterministic automaton and Thompson construction. A deterministic
//! automaton, the subset construction that makes it, and the minimization that joins its states
//! are not written yet. Each one belongs beside [`nfa`].
//!
//! Each identifier comes from lxr, and not from a lexer author. Thus a function panics for an
//! identifier that its automaton does not hold. A full automaton gives a [`BuildError`].

pub mod encoding;
pub mod nfa;

mod adjacency;
mod dfa;
mod error;
mod id;
mod label;
mod state_set;
mod table;
#[cfg(test)]
mod testing;
mod transition;

pub use self::{error::BuildError, id::StateId, label::Label, transition::Transition};
