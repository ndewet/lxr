//! Provides shared automaton test data.

use super::id::StateId;
use super::label::Label;
use super::nfa::Builder;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Symbols {
    pub(super) low: char,
    pub(super) high: char,
}

impl Label for Symbols {
    type Symbol = char;

    fn matches(&self, symbol: char) -> bool {
        (self.low..=self.high).contains(&symbol)
    }
}

pub(super) fn only(symbol: char) -> Symbols {
    Symbols {
        low: symbol,
        high: symbol,
    }
}

pub(super) fn range(low: char, high: char) -> Symbols {
    Symbols { low, high }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Path {
    pub(super) entry: StateId,
    pub(super) exit: StateId,
}

pub(super) fn builder() -> Builder<Symbols> {
    Builder::new()
}

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

pub(super) fn star(builder: &mut Builder<Symbols>, symbol: char) -> StateId {
    let state = builder.add_state();
    builder.add_transition(state, only(symbol), state);
    builder.mark_accept(state);
    state
}
