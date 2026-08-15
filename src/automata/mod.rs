//! The automata of the pipeline, and the parts that they share.
//!
//! An automaton in this module holds no lexer concept and no alphabet. A label of type `L` gives
//! the condition on a transition, and an accept of type `A` gives the meaning of a state that
//! accepts. The caller selects both types.

#![allow(dead_code)]

mod arena;
mod arena_builder;
mod label;
mod nfa;

#[allow(unused_imports)]
pub use self::{arena::Arena, arena_builder::ArenaBuilder, label::Label};
