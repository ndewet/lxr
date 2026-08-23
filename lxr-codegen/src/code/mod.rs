//! Emits the scan of a lexer as code, and not as a table.
//!
//! A scan of a table reads the class of the byte, then it reads the state at that class. The
//! second read waits for the first, thus one byte costs the latency of two reads however the table
//! is shaped. This module writes the rule graph as code instead: each node becomes one function,
//! each byte range becomes a comparison, and the node that a byte gives is a constant.
//!
//! [`step`](step()) gives the function that the emitted `impl` holds. The function is
//! [`Lexer::step`] of the runtime, thus the scan calls it for each token.
//!
//! A call at the end of a function becomes a jump, thus the state of the scan lives in the program
//! counter. A build that makes no optimization keeps each call, thus an edge that closes a cycle
//! writes the node and returns to the driver of the step. The depth of the stack is then the
//! length of the longest path of the graph that holds no cycle.
//!
//! [`Lexer::step`]: https://docs.rs/lxr/latest/lxr/trait.Lexer.html#tymethod.step

mod emitter;
mod node;
mod pattern;
mod rule;
mod step;

pub use self::{rule::Rule, step::step};
