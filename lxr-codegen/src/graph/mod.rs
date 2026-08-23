//! The rule graph of a lexer.
//!
//! A determinization gives one state for each byte of a literal, because the states of a keyword
//! are the states of the rule of a name as well. The scan then reads one byte at a time, and it
//! compares no literal.
//!
//! This module gives a graph of three kinds of node instead. A fork reads one byte. A rope reads a
//! sequence of forced bytes at one time. A leaf gives the token of one rule. A node that does not
//! match goes to its miss, and the scan reads that node at the same offset. Thus a rope survives a
//! rule that overlaps it: the rope reads the literal, and the miss reads the rules that remain.
//!
//! [`build`](build()) is the entry point. It reads the
//! [`Compilation`](crate::compiler::Compilation) of the rules, and it gives the [`Arena`] that the
//! emitter writes as code.
//!
//! The longest match wins, and the earliest rule wins a tie at the same length.
//!
//! Each identifier comes from lxr, and not from a lexer author. Thus a function panics for an
//! identifier that the graph does not hold. A lexer above [`MAX_NODES`] nodes gives an
//! [`Overflow`](crate::automata::Overflow).

mod arena;
mod build;
mod id;
mod node;

#[allow(unused_imports, reason = "the tests of the crate read each item")]
pub use self::{
    arena::Arena,
    build::{MAX_NODES, build},
    id::NodeId,
    node::{Arm, Carry, Edge, Fork, Leaf, Node, Rope},
};
