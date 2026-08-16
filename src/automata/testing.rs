//! The alphabet and the automata that the tests of this module share.
//!
//! The module compiles only under `cfg(test)`. It ships in no build of the crate.
//!
//! An automaton knows no alphabet, thus each test selects one. [`Symbols`] is a range of
//! characters. A character alphabet holds a gap at the values of the surrogates, and it holds more
//! than a million symbols. Thus a test that reads this alphabet catches code in this module that
//! assumes 256 contiguous symbols.
//!
//! Thompson construction is in the compiler, and the automata module does not depend on the
//! compiler. Thus [`literal`] makes the states of one word by hand.

use super::arena::ArenaBuilder;
use super::automaton::Transition;
use super::dfa::DeterministicFiniteAutomaton;
use super::execution::Execution;
use super::id::StateId;
use super::label::Label;
use super::nfa::NfaBuilder;
use super::range::Range;
use super::scanner::Scanner;

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

    fn below(&self, symbol: char) -> bool {
        self.high < symbol
    }

    fn divide(labels: &[Self]) -> Vec<(Self, char)> {
        Self::classes(labels)
    }
}

impl Range for Symbols {
    const LAST: char = char::MAX;

    fn new(low: char, high: char) -> Self {
        range(low, high)
    }

    fn low(&self) -> char {
        self.low
    }

    fn high(&self) -> char {
        self.high
    }

    /// Returns the character after `symbol`, or `None` if `symbol` is the last character.
    ///
    /// The characters leave out the surrogates. Thus the function steps across that gap.
    fn after(symbol: char) -> Option<char> {
        if symbol == BELOW_GAP {
            return Some(ABOVE_GAP);
        }
        char::from_u32(symbol as u32 + 1)
    }

    /// Returns the character before `symbol`, or `None` if `symbol` is the first character.
    ///
    /// The characters leave out the surrogates. Thus the function steps across that gap.
    fn before(symbol: char) -> Option<char> {
        if symbol == ABOVE_GAP {
            return Some(BELOW_GAP);
        }
        char::from_u32((symbol as u32).checked_sub(1)?)
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

/// The first character above the values that the surrogates hold.
pub(super) const ABOVE_GAP: char = '\u{E000}';

/// The last character below the values that the surrogates hold.
pub(super) const BELOW_GAP: char = '\u{D7FF}';

/// The first state and the last state of the states that [`literal`] added.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Path {
    /// The state at which the word starts.
    pub entry: StateId,
    /// The state that accepts the whole word.
    pub exit: StateId,
}

/// Creates an [`NfaBuilder`] of the test alphabet.
pub(super) fn builder() -> NfaBuilder<Symbols> {
    NfaBuilder::new()
}

/// Returns the state at `index`.
pub(super) fn state(index: usize) -> StateId {
    StateId::new(index)
}

/// Adds the states that match `text`, then makes the last state accept.
pub(super) fn literal(builder: &mut NfaBuilder<Symbols>, text: &str) -> Path {
    let entry = builder.push();
    let exit = text.chars().fold(entry, |current, symbol| {
        let next = builder.push();
        builder.transition(current, only(symbol), next);
        next
    });
    builder.accept(exit);
    Path { entry, exit }
}

/// Adds one state that matches any number of `symbol`, and that accepts.
///
/// The state is a loop. Thus a test scans a repetition, and determinization reads a set that
/// reaches itself.
pub(super) fn star(builder: &mut NfaBuilder<Symbols>, symbol: char) -> StateId {
    let state = builder.push();
    builder.transition(state, only(symbol), state);
    builder.accept(state);
    state
}

/// Builds a [`DeterministicFiniteAutomaton`] from one group of transitions for each state, the
/// accepts, and the starts.
///
/// A transition is a label and the index of its target. The labels of one state are in ascending
/// sequence. Only determinization makes a deterministic automaton outside a test, thus a test
/// writes the transitions by hand.
pub(super) fn dfa(
    transitions: &[&[(Symbols, usize)]],
    accepts: &[bool],
    starts: &[usize],
) -> DeterministicFiniteAutomaton<Symbols> {
    let mut arena = ArenaBuilder::new();
    for (state, group) in transitions.iter().enumerate() {
        for &(label, target) in *group {
            arena.push(
                state,
                Transition {
                    label,
                    target: StateId::new(target),
                },
            );
        }
    }
    let arena = arena
        .build(accepts.len())
        .expect("a test stays below the capacity");
    DeterministicFiniteAutomaton::new(
        arena,
        accepts.to_vec(),
        starts.iter().map(|&index| StateId::new(index)).collect(),
    )
}

/// Builds the deterministic automaton that matches `"a"` and `"ab"`.
pub(super) fn chain() -> DeterministicFiniteAutomaton<Symbols> {
    dfa(
        &[&[(only('a'), 1)], &[(only('b'), 2)], &[]],
        &[false, true, true],
        &[0],
    )
}

/// Returns the length of the longest match at the start of `input`, under `start`.
///
/// The test reads a length, and not a meaning of an accept. Thus one call compares a
/// nondeterministic automaton with the deterministic automaton that it gives.
pub(super) fn scan<T>(automaton: &T, start: usize, input: &str) -> Option<usize>
where
    T: Scanner<Symbol = char>,
{
    let symbols: Vec<char> = input.chars().collect();
    automaton
        .execute()
        .longest_match(start, &symbols, |_| ())
        .map(|found| found.length)
}
