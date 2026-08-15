use super::automaton::Automaton;
use super::execution::Execution;
use super::id::{StartId, StateId};
use super::label::Label;
use super::transition::Transition;

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

/// A deterministic automaton, for the tests of the [`Automaton`] trait.
///
/// One state has a maximum of one transition for each symbol. The automaton makes no epsilon
/// transition. Thus an execution is in one state, or in no state.
///
/// This automaton shows that the trait fits more than the NFA. Delete it when the DFA lands.
#[derive(Debug)]
pub(super) struct Dfa<L, A> {
    transitions: Vec<Vec<Transition<L>>>,
    accepts: Vec<Option<A>>,
    starts: Vec<StateId>,
}

impl<L, A> Dfa<L, A> {
    pub(super) fn new(
        transitions: Vec<Vec<Transition<L>>>,
        accepts: Vec<Option<A>>,
        starts: Vec<StateId>,
    ) -> Self {
        Self {
            transitions,
            accepts,
            starts,
        }
    }
}

impl<L: Label, A> Automaton for Dfa<L, A> {
    type Symbol = L::Symbol;
    type Accept = A;
    type Execution<'a>
        = DfaExecution<'a, L, A>
    where
        Self: 'a;

    fn execute(&self, start: StartId) -> Self::Execution<'_> {
        let mut execution = DfaExecution {
            dfa: self,
            state: None,
        };
        execution.restart(start);
        execution
    }
}

/// One scan of a [`Dfa`], in progress.
#[derive(Debug)]
pub(super) struct DfaExecution<'a, L, A> {
    dfa: &'a Dfa<L, A>,
    state: Option<StateId>,
}

impl<L: Label, A> Execution for DfaExecution<'_, L, A> {
    type Symbol = L::Symbol;
    type Accept = A;

    fn restart(&mut self, start: StartId) {
        self.state = Some(self.dfa.starts[start.index()]);
    }

    fn step(&mut self, symbol: Self::Symbol) -> bool {
        self.state = self.state.and_then(|id| {
            self.dfa.transitions[id.index()]
                .iter()
                .find(|transition| transition.label.matches(symbol))
                .map(|transition| transition.target)
        });
        self.state.is_some()
    }

    fn accepts(&self) -> impl Iterator<Item = &Self::Accept> {
        self.state
            .and_then(|id| self.dfa.accepts[id.index()].as_ref())
            .into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::super::scan::{Match, longest_match};
    use super::*;

    /// Builds the automaton that matches `"a"` as accept 1, and `"ab"` as accept 0.
    fn dfa() -> Dfa<Symbols, u32> {
        let target = |index| StateId::new(index);
        Dfa::new(
            vec![
                vec![Transition {
                    label: only('a'),
                    target: target(1),
                }],
                vec![Transition {
                    label: only('b'),
                    target: target(2),
                }],
                Vec::new(),
            ],
            vec![None, Some(1), Some(0)],
            vec![StateId::new(0)],
        )
    }

    fn scan(input: &str) -> Option<Match<u32>> {
        let dfa = dfa();
        let symbols: Vec<char> = input.chars().collect();
        longest_match(&mut dfa.execute(StartId::new(0)), StartId::new(0), &symbols)
    }

    #[test]
    fn the_longer_match_wins() {
        assert_eq!(
            scan("ab"),
            Some(Match {
                accept: 0,
                length: 2
            })
        );
        assert_eq!(
            scan("ac"),
            Some(Match {
                accept: 1,
                length: 1
            })
        );
    }

    #[test]
    fn a_scan_that_reaches_no_accept_gives_nothing() {
        assert_eq!(scan(""), None);
        assert_eq!(scan("b"), None);
    }

    #[test]
    fn trailing_input_is_left_for_the_next_call() {
        assert_eq!(
            scan("abab"),
            Some(Match {
                accept: 0,
                length: 2
            })
        );
    }

    #[test]
    fn one_execution_scans_a_sequence_of_matches() {
        let dfa = dfa();
        let symbols: Vec<char> = "abaab".chars().collect();
        let mut execution = dfa.execute(StartId::new(0));
        let mut input = &symbols[..];
        let mut accepts = Vec::new();

        while let Some(found) = longest_match(&mut execution, StartId::new(0), input) {
            accepts.push(found.accept);
            input = &input[found.length..];
        }

        assert_eq!(accepts, vec![0, 1, 0]);
        assert_eq!(input, []);
    }
}
