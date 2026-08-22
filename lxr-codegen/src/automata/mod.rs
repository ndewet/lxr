//! The automata of the pipeline, and the parts that they share.
//!
//! An automaton in this module holds no lexer concept and no alphabet. A label of type `L` gives
//! the condition on a transition, and the caller selects that type.
//!
//! An automaton knows which states accept. It does not know what an accept means. A lexer holds
//! the token of each state that accepts, and [`Execution::longest_match`] asks the caller for that
//! meaning. Thus no rule of precedence lives here.
//!
//! Each automaton implements [`Automaton`], which gives the structure, and [`Scanner`], which
//! starts a scan. Thus [`Execution::longest_match`] scans a nondeterministic automaton and a
//! deterministic automaton with the same code.
//!
//! [`NondeterministicFiniteAutomaton::determinize`] joins the two. It gives the
//! [`DeterministicFiniteAutomaton`] that accepts the same input, and the set of states behind each
//! of its states. It divides the labels with [`Label::divide`], thus it needs no alphabet of its
//! own.
//!
//! Each identifier comes from lxr, and not from a lexer author. Thus a function panics for an
//! identifier that its automaton does not hold. A full automaton gives an [`Overflow`].

mod arena;
mod automaton;
mod determinize;
mod dfa;
mod execution;
mod id;
mod label;
mod nfa;
mod overflow;
mod range;
mod scanner;
mod table;
#[cfg(test)]
mod testing;

#[allow(unused_imports)]
pub use self::{
    arena::{Arena, ArenaBuilder},
    automaton::{Automaton, Transition},
    determinize::{Determinization, MAX_STATES},
    dfa::{DeterministicExecution, DeterministicFiniteAutomaton},
    execution::{Execution, Match},
    id::StateId,
    label::Label,
    nfa::{NfaBuilder, NondeterministicExecution, NondeterministicFiniteAutomaton},
    overflow::{Overflow, Part},
    range::Range,
    scanner::Scanner,
    table::StateTable,
};
