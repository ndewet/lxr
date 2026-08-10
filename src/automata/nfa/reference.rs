use super::automaton::{Nfa, StartId};
use super::builder::NfaBuilder;
use super::simulation::Simulator;
use super::state::{AcceptId, State, StateId};

/// A match found by [`Nfa::longest_match`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    /// The accept that was reached.
    pub accept: AcceptId,
    /// The number of bytes the match spans.
    pub length: usize,
}

pub(super) fn literal(builder: &mut NfaBuilder, bytes: &[u8], accept: usize) -> StateId {
    let accept = builder.push(State::Match {
        accept: AcceptId::new(accept),
    });
    bytes.iter().rev().fold(accept, |next, &byte| {
        builder.push(State::Range {
            low: byte,
            high: byte,
            next,
        })
    })
}

pub(super) fn star(builder: &mut NfaBuilder, byte: u8, accept: usize) -> StateId {
    let accept = builder.push(State::Match {
        accept: AcceptId::new(accept),
    });
    let split = builder.reserve();
    let body = builder.push(State::Range {
        low: byte,
        high: byte,
        next: split,
    });
    builder.fill(
        split,
        State::Split {
            first: body,
            second: accept,
        },
    );
    split
}

fn accepted(nfa: &Nfa, states: &[StateId]) -> Option<AcceptId> {
    states
        .iter()
        .filter_map(|&id| match nfa.state(id) {
            State::Match { accept } => Some(accept),
            _ => None,
        })
        .min()
}

impl Simulator<'_> {
    /// Returns the longest match at the start of `input` under `start`, or `None` if no accept is
    /// reached.
    ///
    /// Where several accepts are reached at the same length, the lowest one wins.
    ///
    /// Only the rules behind `start` are live. The other start conditions take no part in the
    /// scan, so a rule of theirs can neither match here nor make this scan nullable.
    ///
    /// This scans one token. A scanner runs one simulator over a whole input, so the closure
    /// scratch is allocated once rather than once per token.
    ///
    /// # Panics
    ///
    /// Panics if `start` is not a start condition of the automaton.
    pub fn longest_match(&mut self, start: StartId, input: &[u8]) -> Option<Match> {
        let nfa = self.nfa();
        let mut current = Vec::new();
        let mut next = Vec::new();

        self.epsilon_closure(&[nfa.start_state(start)], &mut current);
        let mut best = accepted(nfa, &current).map(|accept| Match { accept, length: 0 });

        for (consumed, &byte) in input.iter().enumerate() {
            nfa.step(&current, byte, &mut next);
            if next.is_empty() {
                break;
            }
            self.epsilon_closure(&next, &mut current);
            best = accepted(nfa, &current)
                .map(|accept| Match {
                    accept,
                    length: consumed + 1,
                })
                .or(best);
        }

        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matched(accept: usize, length: usize) -> Option<Match> {
        Some(Match {
            accept: AcceptId::new(accept),
            length,
        })
    }

    fn scan(nfa: &Nfa, input: &[u8]) -> Option<Match> {
        scan_under(nfa, 0, input)
    }

    fn scan_under(nfa: &Nfa, start: usize, input: &[u8]) -> Option<Match> {
        Simulator::new(nfa).longest_match(StartId::new(start), input)
    }

    #[test]
    fn a_one_byte_pattern_matches_that_byte() {
        let mut builder = NfaBuilder::new();
        let start = literal(&mut builder, b"a", 0);
        let nfa = builder.build(&[start]);

        assert_eq!(scan(&nfa, b"a"), matched(0, 1));
    }

    #[test]
    fn a_one_byte_pattern_rejects_another_byte() {
        let mut builder = NfaBuilder::new();
        let start = literal(&mut builder, b"a", 0);
        let nfa = builder.build(&[start]);

        assert_eq!(scan(&nfa, b"b"), None);
        assert_eq!(scan(&nfa, b""), None);
    }

    #[test]
    fn a_chain_matches_a_multi_byte_character() {
        let mut builder = NfaBuilder::new();
        let start = literal(&mut builder, "é".as_bytes(), 0);
        let nfa = builder.build(&[start]);

        assert_eq!(scan(&nfa, "é".as_bytes()), matched(0, 2));
        assert_eq!(scan(&nfa, &[0xC3]), None);
        assert_eq!(scan(&nfa, &[0xC3, 0xA8]), None);
    }

    #[test]
    fn an_alternation_matches_either_branch() {
        let mut builder = NfaBuilder::new();
        let left = literal(&mut builder, b"ab", 0);
        let right = literal(&mut builder, b"cd", 1);
        let start = builder.push(State::Split {
            first: left,
            second: right,
        });
        let nfa = builder.build(&[start]);

        assert_eq!(scan(&nfa, b"ab"), matched(0, 2));
        assert_eq!(scan(&nfa, b"cd"), matched(1, 2));
        assert_eq!(scan(&nfa, b"ac"), None);
    }

    #[test]
    fn a_star_matches_any_number_of_repetitions() {
        let mut builder = NfaBuilder::new();
        let start = star(&mut builder, b'a', 0);
        let nfa = builder.build(&[start]);

        assert_eq!(scan(&nfa, b""), matched(0, 0));
        assert_eq!(scan(&nfa, b"zzz"), matched(0, 0));
        assert_eq!(scan(&nfa, b"a"), matched(0, 1));
        assert_eq!(scan(&nfa, b"aaaa"), matched(0, 4));
    }

    #[test]
    fn the_longer_match_wins() {
        let mut builder = NfaBuilder::new();
        let short = literal(&mut builder, b"a", 0);
        let long = literal(&mut builder, b"ab", 1);
        let start = builder.push(State::Split {
            first: short,
            second: long,
        });
        let nfa = builder.build(&[start]);

        assert_eq!(scan(&nfa, b"ab"), matched(1, 2));
        assert_eq!(scan(&nfa, b"ac"), matched(0, 1));
    }

    #[test]
    fn trailing_input_is_left_for_the_next_call() {
        let mut builder = NfaBuilder::new();
        let start = literal(&mut builder, b"ab", 0);
        let nfa = builder.build(&[start]);

        assert_eq!(scan(&nfa, b"abcdef"), matched(0, 2));
    }

    #[test]
    fn the_lower_accept_wins_a_tie() {
        let mut builder = NfaBuilder::new();
        let keyword = literal(&mut builder, b"if", 0);
        let identifier = literal(&mut builder, b"if", 1);
        let start = builder.push(State::Split {
            first: identifier,
            second: keyword,
        });
        let nfa = builder.build(&[start]);

        assert_eq!(scan(&nfa, b"if"), matched(0, 2));
    }

    #[test]
    fn a_start_with_no_reachable_match_accepts_nothing() {
        let mut builder = NfaBuilder::new();
        let stuck = builder.reserve();
        builder.fill(
            stuck,
            State::Split {
                first: stuck,
                second: stuck,
            },
        );
        let nfa = builder.build(&[stuck]);

        assert_eq!(scan(&nfa, b""), None);
        assert_eq!(scan(&nfa, b"anything"), None);
    }

    #[test]
    fn one_simulator_scans_a_sequence_of_matches() {
        let mut builder = NfaBuilder::new();
        let keyword = literal(&mut builder, b"if", 0);
        let space = literal(&mut builder, b" ", 1);
        let start = builder.push(State::Split {
            first: keyword,
            second: space,
        });
        let nfa = builder.build(&[start]);

        let mut simulator = Simulator::new(&nfa);
        let mut input: &[u8] = b"if if";
        let mut accepts = Vec::new();

        while let Some(found) = simulator.longest_match(StartId::new(0), input) {
            accepts.push(found.accept.index());
            input = &input[found.length..];
        }

        assert_eq!(accepts, vec![0, 1, 0]);
        assert_eq!(input, b"");
    }

    #[test]
    fn each_entry_point_scans_only_its_own_rules() {
        let mut builder = NfaBuilder::new();
        let code = literal(&mut builder, b"a", 0);
        let string = literal(&mut builder, b"b", 1);
        let nfa = builder.build(&[code, string]);

        assert_eq!(scan_under(&nfa, 0, b"a"), matched(0, 1));
        assert_eq!(scan_under(&nfa, 0, b"b"), None);

        assert_eq!(scan_under(&nfa, 1, b"b"), matched(1, 1));
        assert_eq!(scan_under(&nfa, 1, b"a"), None);
    }

    #[test]
    fn a_nullable_entry_point_does_not_make_another_nullable() {
        let mut builder = NfaBuilder::new();
        let literal_start = literal(&mut builder, b"ab", 0);
        let star_start = star(&mut builder, b'a', 1);
        let nfa = builder.build(&[literal_start, star_start]);

        assert_eq!(scan_under(&nfa, 1, b"zz"), matched(1, 0));
        assert_eq!(scan_under(&nfa, 0, b"zz"), None);
        assert_eq!(scan_under(&nfa, 0, b"ab"), matched(0, 2));
    }

    #[test]
    fn a_lower_accept_in_another_condition_does_not_win() {
        let mut builder = NfaBuilder::new();
        let code = literal(&mut builder, b"if", 0);
        let string = literal(&mut builder, b"if", 1);
        let nfa = builder.build(&[code, string]);

        assert_eq!(scan_under(&nfa, 1, b"if"), matched(1, 2));
    }

    #[test]
    #[should_panic(expected = "start 2 is outside an automaton with 2 start conditions")]
    fn scanning_under_a_start_the_automaton_does_not_have_panics() {
        let mut builder = NfaBuilder::new();
        let code = literal(&mut builder, b"a", 0);
        let string = literal(&mut builder, b"b", 1);
        let nfa = builder.build(&[code, string]);

        scan_under(&nfa, 2, b"a");
    }
}
