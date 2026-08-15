use super::automaton::Nfa;
use super::builder::NfaBuilder;
use super::id::{StartId, StateId};
use super::simulation::Simulator;
use crate::automata::label::Label;

/// The test alphabet. The automaton itself knows no alphabet, thus a test selects one.
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

/// A match that [`longest_match`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match<A> {
    /// The accept that the scan reached.
    pub accept: A,
    /// The number of the symbols in the match.
    pub length: usize,
}

pub(super) fn literal(builder: &mut NfaBuilder<Symbols, u32>, text: &str, accept: u32) -> StateId {
    let start = builder.push();
    let end = text.chars().fold(start, |current, symbol| {
        let next = builder.push();
        builder.transition(current, only(symbol), next);
        next
    });
    builder.accept(end, accept);
    start
}

pub(super) fn star(builder: &mut NfaBuilder<Symbols, u32>, symbol: char, accept: u32) -> StateId {
    let state = builder.push();
    builder.transition(state, only(symbol), state);
    builder.accept(state, accept);
    state
}

fn accepted<L, A: Ord + Copy>(nfa: &Nfa<L, A>, states: &[StateId]) -> Option<A> {
    states
        .iter()
        .filter_map(|&id| nfa.accept(id).copied())
        .min()
}

/// Returns the longest match at the start of `input` under `start`.
///
/// The function returns `None` if the scan reaches no accept.
///
/// If the scan reaches more than one accept at the same length, the lowest accept is the result.
/// Maximal munch and that tie-break are the rules of the caller. The automaton does not apply
/// them.
///
/// Only the rules of `start` are applicable. The other start states take no part in the scan. Thus
/// a rule of another start state cannot match here. It also cannot make this scan nullable.
///
/// The function scans one token. A scanner runs one simulator over the full input. Thus the
/// scanner makes the scratch space for the closure one time, and not one time for each token.
///
/// # Panics
///
/// This function panics if `start` is not a start state of the automaton.
pub(super) fn longest_match<L: Label, A: Ord + Copy>(
    simulator: &mut Simulator<'_, L, A>,
    start: StartId,
    input: &[L::Symbol],
) -> Option<Match<A>> {
    let nfa = simulator.nfa();
    let mut current = Vec::new();
    let mut next = Vec::new();

    simulator.epsilon_closure(&[nfa.start_state(start)], &mut current);
    let mut best = accepted(nfa, &current).map(|accept| Match { accept, length: 0 });

    for (consumed, &symbol) in input.iter().enumerate() {
        nfa.step(&current, symbol, &mut next);
        if next.is_empty() {
            break;
        }
        simulator.epsilon_closure(&next, &mut current);
        best = accepted(nfa, &current)
            .map(|accept| Match {
                accept,
                length: consumed + 1,
            })
            .or(best);
    }

    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matched(accept: u32, length: usize) -> Option<Match<u32>> {
        Some(Match { accept, length })
    }

    fn scan(nfa: &Nfa<Symbols, u32>, input: &str) -> Option<Match<u32>> {
        scan_under(nfa, 0, input)
    }

    fn scan_under(nfa: &Nfa<Symbols, u32>, start: usize, input: &str) -> Option<Match<u32>> {
        let symbols: Vec<char> = input.chars().collect();
        longest_match(&mut Simulator::new(nfa), StartId::new(start), &symbols)
    }

    #[test]
    fn a_one_symbol_pattern_matches_that_symbol() {
        let mut builder = NfaBuilder::new();
        let start = literal(&mut builder, "a", 0);
        let nfa = builder.build(&[start]);

        assert_eq!(scan(&nfa, "a"), matched(0, 1));
    }

    #[test]
    fn a_one_symbol_pattern_rejects_another_symbol() {
        let mut builder = NfaBuilder::new();
        let start = literal(&mut builder, "a", 0);
        let nfa = builder.build(&[start]);

        assert_eq!(scan(&nfa, "b"), None);
        assert_eq!(scan(&nfa, ""), None);
    }

    #[test]
    fn a_chain_matches_each_symbol_in_sequence() {
        let mut builder = NfaBuilder::new();
        let start = literal(&mut builder, "ab", 0);
        let nfa = builder.build(&[start]);

        assert_eq!(scan(&nfa, "ab"), matched(0, 2));
        assert_eq!(scan(&nfa, "a"), None);
        assert_eq!(scan(&nfa, "ac"), None);
    }

    #[test]
    fn an_alternation_matches_either_branch() {
        let mut builder = NfaBuilder::new();
        let left = literal(&mut builder, "ab", 0);
        let right = literal(&mut builder, "cd", 1);
        let start = builder.push();
        builder.epsilon(start, left);
        builder.epsilon(start, right);
        let nfa = builder.build(&[start]);

        assert_eq!(scan(&nfa, "ab"), matched(0, 2));
        assert_eq!(scan(&nfa, "cd"), matched(1, 2));
        assert_eq!(scan(&nfa, "ac"), None);
    }

    #[test]
    fn a_star_matches_any_number_of_repetitions() {
        let mut builder = NfaBuilder::new();
        let start = star(&mut builder, 'a', 0);
        let nfa = builder.build(&[start]);

        assert_eq!(scan(&nfa, ""), matched(0, 0));
        assert_eq!(scan(&nfa, "zzz"), matched(0, 0));
        assert_eq!(scan(&nfa, "a"), matched(0, 1));
        assert_eq!(scan(&nfa, "aaaa"), matched(0, 4));
    }

    #[test]
    fn the_longer_match_wins() {
        let mut builder = NfaBuilder::new();
        let short = literal(&mut builder, "a", 0);
        let long = literal(&mut builder, "ab", 1);
        let start = builder.push();
        builder.epsilon(start, short);
        builder.epsilon(start, long);
        let nfa = builder.build(&[start]);

        assert_eq!(scan(&nfa, "ab"), matched(1, 2));
        assert_eq!(scan(&nfa, "ac"), matched(0, 1));
    }

    #[test]
    fn trailing_input_is_left_for_the_next_call() {
        let mut builder = NfaBuilder::new();
        let start = literal(&mut builder, "ab", 0);
        let nfa = builder.build(&[start]);

        assert_eq!(scan(&nfa, "abcdef"), matched(0, 2));
    }

    #[test]
    fn the_lower_accept_wins_a_tie() {
        let mut builder = NfaBuilder::new();
        let keyword = literal(&mut builder, "if", 0);
        let identifier = literal(&mut builder, "if", 1);
        let start = builder.push();
        builder.epsilon(start, identifier);
        builder.epsilon(start, keyword);
        let nfa = builder.build(&[start]);

        assert_eq!(scan(&nfa, "if"), matched(0, 2));
    }

    #[test]
    fn a_start_with_no_reachable_accept_accepts_nothing() {
        let mut builder = NfaBuilder::<Symbols, u32>::new();
        let stuck = builder.push();
        builder.epsilon(stuck, stuck);
        let nfa = builder.build(&[stuck]);

        assert_eq!(scan(&nfa, ""), None);
        assert_eq!(scan(&nfa, "anything"), None);
    }

    #[test]
    fn one_simulator_scans_a_sequence_of_matches() {
        let mut builder = NfaBuilder::new();
        let keyword = literal(&mut builder, "if", 0);
        let space = literal(&mut builder, " ", 1);
        let start = builder.push();
        builder.epsilon(start, keyword);
        builder.epsilon(start, space);
        let nfa = builder.build(&[start]);

        let mut simulator = Simulator::new(&nfa);
        let symbols: Vec<char> = "if if".chars().collect();
        let mut input = &symbols[..];
        let mut accepts = Vec::new();

        while let Some(found) = longest_match(&mut simulator, StartId::new(0), input) {
            accepts.push(found.accept);
            input = &input[found.length..];
        }

        assert_eq!(accepts, vec![0, 1, 0]);
        assert_eq!(input, []);
    }

    #[test]
    fn each_start_scans_only_its_own_rules() {
        let mut builder = NfaBuilder::new();
        let code = literal(&mut builder, "a", 0);
        let string = literal(&mut builder, "b", 1);
        let nfa = builder.build(&[code, string]);

        assert_eq!(scan_under(&nfa, 0, "a"), matched(0, 1));
        assert_eq!(scan_under(&nfa, 0, "b"), None);

        assert_eq!(scan_under(&nfa, 1, "b"), matched(1, 1));
        assert_eq!(scan_under(&nfa, 1, "a"), None);
    }

    #[test]
    fn a_nullable_start_does_not_make_another_start_nullable() {
        let mut builder = NfaBuilder::new();
        let literal_start = literal(&mut builder, "ab", 0);
        let star_start = star(&mut builder, 'a', 1);
        let nfa = builder.build(&[literal_start, star_start]);

        assert_eq!(scan_under(&nfa, 1, "zz"), matched(1, 0));
        assert_eq!(scan_under(&nfa, 0, "zz"), None);
        assert_eq!(scan_under(&nfa, 0, "ab"), matched(0, 2));
    }

    #[test]
    fn a_lower_accept_under_another_start_does_not_win() {
        let mut builder = NfaBuilder::new();
        let code = literal(&mut builder, "if", 0);
        let string = literal(&mut builder, "if", 1);
        let nfa = builder.build(&[code, string]);

        assert_eq!(scan_under(&nfa, 1, "if"), matched(1, 2));
    }

    #[test]
    #[should_panic(expected = "start 2 is outside an automaton with 2 start states")]
    fn scanning_under_a_start_the_automaton_does_not_have_panics() {
        let mut builder = NfaBuilder::new();
        let code = literal(&mut builder, "a", 0);
        let string = literal(&mut builder, "b", 1);
        let nfa = builder.build(&[code, string]);

        scan_under(&nfa, 2, "a");
    }
}
