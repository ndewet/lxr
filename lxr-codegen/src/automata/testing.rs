//! The alphabet and the automata that the tests of this module share.
//!
//! The module compiles only under `cfg(test)`. It ships in no build of the crate.
//!
//! An automaton knows no alphabet, thus each test selects one. [`Symbols`] is a range of
//! characters. A character alphabet holds more than a million symbols. Thus a test that reads this
//! alphabet catches code in this module that assumes 256 contiguous symbols.
//!
//! Thompson construction reads an [`Encoding`](super::encoding::Encoding), and [`Symbols`] is not
//! one. Thus [`literal`] makes the states of one word by hand.

use super::id::StateId;
use super::label::Label;
use super::nfa::Builder;

/// The test alphabet. An automaton knows no alphabet, thus a test selects one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Symbols {
    pub low: char,
    pub high: char,
}

impl Label for Symbols {
    type Symbol = char;

    fn matches(&self, symbol: char) -> bool {
        (self.low..=self.high).contains(&symbol)
    }
}

/// A label that matches only `symbol`.
pub(super) fn only(symbol: char) -> Symbols {
    Symbols {
        low: symbol,
        high: symbol,
    }
}

/// A label that matches each symbol from `low` to `high`.
pub(super) fn range(low: char, high: char) -> Symbols {
    Symbols { low, high }
}

/// The first state and the last state of the states that [`literal`] added.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Path {
    /// The state at which the word starts.
    pub entry: StateId,
    /// The state that accepts the whole word.
    pub exit: StateId,
}

/// Creates a [`Builder`] of the test alphabet.
pub(super) fn builder() -> Builder<Symbols> {
    Builder::new()
}

/// Adds the states that match `text`, then makes the last state accept.
pub(super) fn literal(builder: &mut Builder<Symbols>, text: &str) -> Path {
    let entry = builder.add_state();
    let exit = text.chars().fold(entry, |current, symbol| {
        let next = builder.add_state();
        builder.add_transition(current, only(symbol), next);
        next
    });
    builder.mark_accept(exit);
    Path { entry, exit }
}

/// Adds one state that matches any number of `symbol`, and that accepts.
///
/// The state is a loop. Thus a test scans a repetition, and determinization reads a set that
/// reaches itself.
pub(super) fn star(builder: &mut Builder<Symbols>, symbol: char) -> StateId {
    let state = builder.add_state();
    builder.add_transition(state, only(symbol), state);
    builder.mark_accept(state);
    state
}
