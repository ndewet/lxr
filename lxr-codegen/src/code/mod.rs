//! Emits the scan of a lexer as code, and not as a table.
//!
//! A scan of a table reads the class of the byte, then it reads the state at that class. The
//! second read waits for the first, thus one byte costs the latency of two reads however the table
//! is shaped. This module writes the hot region as structured Rust control flow. Cold paths are
//! outlined as functions, each byte range becomes a comparison, and byte classes alone use tables.
//!
//! [`step`](step()) gives the function that the emitted `impl` holds. The function is
//! [`Lexer::step`] of the runtime, thus the scan calls it for each token.
//!
//! An edge that closes a cycle writes a compact resume cursor and continues the matcher loop.
//! Ordinary hot edges remain branches inside the body of `step`.
//!
//! [`Lexer::step`]: https://docs.rs/lxr/latest/lxr/trait.Lexer.html#tymethod.step

mod emitter;
mod node;
mod pattern;
mod rule;
mod step;

pub use self::{rule::Rule, step::step};
