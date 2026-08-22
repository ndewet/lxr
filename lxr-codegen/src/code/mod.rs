//! Emits the scan of a lexer as code, and not as a table.
//!
//! A scan of a table reads the class of the byte, then it reads the state at that class. The
//! second read waits for the first, thus one byte costs the latency of two reads however the table
//! is shaped. This module writes the same automaton as code: each state becomes one arm, each
//! label becomes a comparison on the byte, and the state that a byte gives is a constant.
//!
//! [`find`](find()) gives the function that the emitted `impl` holds. The function is
//! [`Lexer::find`] of the runtime, thus a scan calls it for each token.
//!
//! The code holds no record of the states, and a scan of a region that no rule ends needs that
//! record. The runtime keeps the tables for that region, thus this code pays nothing for it.
//!
//! A lexer of more than [`MAX_CODE_STATES`] states gets no code. The source of it costs more
//! compile time than the scan saves, and the runtime then scans the tables.
//!
//! [`Lexer::find`]: https://docs.rs/lxr/latest/lxr/trait.Lexer.html#method.find

mod find;
mod pattern;
mod state;

#[allow(unused_imports, reason = "the tests of the module read the limit")]
pub use self::find::{MAX_CODE_STATES, find};
