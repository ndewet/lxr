use super::{Execution, Nfa};
use crate::automata::{Label, StateId};

/// A match that [`Matcher::longest_match`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match<T> {
    /// The value that `select` gave for the states at the end of the match.
    pub accept: T,
    /// The number of symbols in the match.
    pub length: usize,
}

/// Finds longest matches with an NFA execution.
///
/// The matcher adds the longest-match policy to [`Execution`]. It remembers
/// the last state set that accepts while it reads the input. It reuses the
/// execution buffers for each token.
#[derive(Debug)]
pub struct Matcher<'a, L, A = ()> {
    execution: Execution<'a, L, A>,
}

impl<'a, L: Label, A> Matcher<'a, L, A> {
    /// Creates a matcher for `nfa`.
    pub(super) fn new(nfa: &'a Nfa<L, A>) -> Self {
        Self {
            execution: Execution::new(nfa),
        }
    }

    /// Returns the longest match at the start of `input` under `start`.
    ///
    /// `select` resolves the accept values of the states at the end of a
    /// match. The function calls `select` only at a position that accepts.
    ///
    /// A start state that accepts gives a match of zero length. A lexer must
    /// reject such a rule or otherwise guarantee forward progress.
    /// Only the rules reachable from the selected start state take part.
    ///
    /// # Panics
    ///
    /// This function panics if `start` is not a start state of the NFA.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr_codegen::automata::encoding::ByteRange;
    /// use lxr_codegen::automata::nfa::Builder;
    ///
    /// let mut builder = Builder::<ByteRange>::new();
    /// let start = builder.add_state();
    /// let accept = builder.add_state();
    /// builder.add_transition(
    ///     start,
    ///     ByteRange::new(b'a', b'a'),
    ///     accept,
    /// );
    /// builder.mark_accept(accept);
    /// let nfa = builder.build(&[start]).unwrap();
    /// let mut matcher = nfa.matcher();
    ///
    /// let found = matcher.longest_match(0, b"ab", |_| "a").unwrap();
    /// assert_eq!(found.accept, "a");
    /// assert_eq!(found.length, 1);
    /// ```
    pub fn longest_match<T>(
        &mut self,
        start: usize,
        input: &[L::Symbol],
        select: impl Fn(&[StateId]) -> T,
    ) -> Option<Match<T>> {
        self.execution.restart(start);
        let mut best = self.accepted(&select, 0);

        for (consumed, &symbol) in input.iter().enumerate() {
            if !self.execution.step(symbol) {
                break;
            }
            best = self.accepted(&select, consumed + 1).or(best);
        }

        best
    }

    fn accepted<T>(&self, select: &impl Fn(&[StateId]) -> T, length: usize) -> Option<Match<T>> {
        self.execution.accepts().then(|| Match {
            accept: select(self.execution.states()),
            length,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::nfa::Builder;
    use crate::automata::testing::{Symbols, builder, literal, only, star};

    /// The accept of each state, in the manner of a lexer table.
    ///
    /// The automaton says which states accept. This table says what each accept means, and the
    /// lowest accept wins a tie.
    fn accepts(count: usize, marks: &[(StateId, u32)]) -> Vec<Option<u32>> {
        let mut table = vec![None; count];
        for &(state, accept) in marks {
            table[state.index()] = Some(accept);
        }
        table
    }

    /// Returns the longest match at the start of `input` under the first start, with the lowest
    /// accept of the table.
    fn scan(nfa: &Nfa<Symbols>, table: &[Option<u32>], input: &str) -> Option<Match<u32>> {
        scan_under(nfa, table, 0, input)
    }

    /// Returns the longest match at the start of `input` under `start`, with the lowest accept of
    /// the table.
    fn scan_under(
        nfa: &Nfa<Symbols>,
        table: &[Option<u32>],
        start: usize,
        input: &str,
    ) -> Option<Match<u32>> {
        let symbols: Vec<char> = input.chars().collect();
        nfa.matcher()
            .longest_match(start, &symbols, |states| lowest(table, states))
    }

    /// Builds an automaton of one rule for each word, and the table of their accepts.
    ///
    /// Each word gets its own start state, at the index of the word. The rule of a word accepts
    /// that index. Thus a scan under one start reads only the word of that start.
    fn conditions(words: &[&str]) -> (Nfa<Symbols>, Vec<Option<u32>>) {
        let mut builder = builder();
        let paths: Vec<_> = words
            .iter()
            .map(|word| literal(&mut builder, word))
            .collect();
        let starts: Vec<StateId> = paths.iter().map(|path| path.entry).collect();
        let nfa = builder
            .build(&starts)
            .expect("a test stays below the capacity");

        let marks: Vec<(StateId, u32)> = paths
            .iter()
            .enumerate()
            .map(|(index, path)| (path.exit, index as u32))
            .collect();
        let table = accepts(nfa.state_count(), &marks);
        (nfa, table)
    }

    /// Returns the lowest accept of the states in `states`.
    fn lowest(table: &[Option<u32>], states: &[StateId]) -> u32 {
        states
            .iter()
            .filter_map(|id| table[id.index()])
            .min()
            .expect("a state of the execution accepts")
    }

    fn matched(accept: u32, length: usize) -> Option<Match<u32>> {
        Some(Match { accept, length })
    }

    /// Builds an automaton of two rules, and the table of their accepts.
    ///
    /// The first rule matches `first` and accepts 0. The second rule matches `second` and accepts
    /// 1. Thus the first rule wins a tie.
    fn alternation(first: &str, second: &str) -> (Nfa<Symbols>, Vec<Option<u32>>) {
        let mut builder = Builder::new();
        let left = literal(&mut builder, first);
        let right = literal(&mut builder, second);
        let start = builder.add_state();
        builder.add_epsilon_transition(start, left.entry);
        builder.add_epsilon_transition(start, right.entry);
        let nfa = builder
            .build(&[start])
            .expect("a test stays below the capacity");

        let table = accepts(nfa.state_count(), &[(left.exit, 0), (right.exit, 1)]);
        (nfa, table)
    }

    #[test]
    fn the_longest_match_wins() {
        let (nfa, table) = alternation("a", "ab");

        assert_eq!(scan(&nfa, &table, "ab"), matched(1, 2));
        assert_eq!(scan(&nfa, &table, "ac"), matched(0, 1));
    }

    #[test]
    fn select_breaks_a_tie_at_the_same_length() {
        let (nfa, table) = alternation("if", "if");

        assert_eq!(scan(&nfa, &table, "if"), matched(0, 2));
    }

    #[test]
    fn a_scan_that_reaches_no_accept_gives_nothing() {
        let (nfa, table) = alternation("a", "ab");

        assert_eq!(scan(&nfa, &table, ""), None);
        assert_eq!(scan(&nfa, &table, "z"), None);
    }

    #[test]
    fn trailing_input_is_left_for_the_next_call() {
        let (nfa, table) = alternation("a", "ab");

        assert_eq!(scan(&nfa, &table, "abab"), matched(1, 2));
    }

    #[test]
    fn a_start_that_accepts_gives_a_match_of_no_length() {
        let mut builder = builder();
        let start = builder.add_state();
        builder.mark_accept(start);
        let nfa = builder
            .build(&[start])
            .expect("a test stays below the capacity");
        let table = accepts(nfa.state_count(), &[(start, 5)]);

        assert_eq!(scan(&nfa, &table, ""), matched(5, 0));
        assert_eq!(scan(&nfa, &table, "zz"), matched(5, 0));
    }

    #[test]
    fn a_star_matches_any_number_of_repetitions() {
        let mut builder = builder();
        let repeat = star(&mut builder, 'a');
        let nfa = builder
            .build(&[repeat])
            .expect("a test stays below the capacity");
        let table = accepts(nfa.state_count(), &[(repeat, 0)]);

        assert_eq!(scan(&nfa, &table, ""), matched(0, 0));
        assert_eq!(scan(&nfa, &table, "zzz"), matched(0, 0));
        assert_eq!(scan(&nfa, &table, "a"), matched(0, 1));
        assert_eq!(scan(&nfa, &table, "aaaa"), matched(0, 4));
    }

    #[test]
    fn a_start_with_no_reachable_accept_accepts_nothing() {
        let mut builder = builder();
        let stuck = builder.add_state();
        builder.add_epsilon_transition(stuck, stuck);
        let nfa = builder
            .build(&[stuck])
            .expect("a test stays below the capacity");
        let table = accepts(nfa.state_count(), &[]);

        assert_eq!(scan(&nfa, &table, ""), None);
        assert_eq!(scan(&nfa, &table, "anything"), None);
    }

    #[test]
    fn each_start_scans_only_its_own_rules() {
        let (nfa, table) = conditions(&["a", "b"]);

        assert_eq!(scan_under(&nfa, &table, 0, "a"), matched(0, 1));
        assert_eq!(scan_under(&nfa, &table, 0, "b"), None);
        assert_eq!(scan_under(&nfa, &table, 1, "b"), matched(1, 1));
        assert_eq!(scan_under(&nfa, &table, 1, "a"), None);
    }

    #[test]
    fn a_lower_accept_under_another_start_does_not_win() {
        let (nfa, table) = conditions(&["if", "if"]);

        assert_eq!(scan_under(&nfa, &table, 1, "if"), matched(1, 2));
        assert_eq!(scan_under(&nfa, &table, 0, "if"), matched(0, 2));
    }

    #[test]
    fn a_nullable_start_does_not_make_another_start_nullable() {
        let mut builder = builder();
        let word = literal(&mut builder, "ab");
        let repeat = star(&mut builder, 'a');
        let nfa = builder
            .build(&[word.entry, repeat])
            .expect("a test stays below the capacity");
        let table = accepts(nfa.state_count(), &[(word.exit, 0), (repeat, 1)]);

        assert_eq!(scan_under(&nfa, &table, 1, "zz"), matched(1, 0));
        assert_eq!(scan_under(&nfa, &table, 0, "zz"), None);
        assert_eq!(scan_under(&nfa, &table, 0, "ab"), matched(0, 2));
    }

    #[test]
    #[should_panic(expected = "start 2 is outside an automaton with 2 start states")]
    fn a_scan_under_a_start_the_automaton_does_not_have_panics() {
        let (nfa, table) = conditions(&["a", "b"]);

        scan_under(&nfa, &table, 2, "a");
    }

    #[test]
    fn one_matcher_scans_a_sequence_of_matches() {
        let (nfa, table) = alternation("if", " ");
        let mut matcher = nfa.matcher();
        let symbols: Vec<char> = "if if".chars().collect();
        let mut input = &symbols[..];
        let mut found = Vec::new();

        while let Some(scanned) = matcher.longest_match(0, input, |states| lowest(&table, states)) {
            found.push(scanned.accept);
            input = &input[scanned.length..];
        }

        assert_eq!(found, vec![0, 1, 0]);
        assert!(input.is_empty());
    }

    #[test]
    fn select_reads_the_states_at_the_end_of_the_match() {
        let (nfa, _) = alternation("a", "ab");
        let symbols: Vec<char> = "ab".chars().collect();

        let found = nfa
            .matcher()
            .longest_match(0, &symbols, <[StateId]>::to_vec)
            .expect("the automaton accepts ab");

        assert_eq!(found.length, 2);
        assert_eq!(found.accept, vec![StateId::new(4)]);
    }

    #[test]
    fn select_reads_accept_values_from_a_tagged_automaton() {
        let mut builder = Builder::<Symbols, u32>::new();
        let start = builder.add_state();
        let accept = builder.add_state();
        builder.add_transition(start, only('a'), accept);
        builder.set_accept(accept, 7);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        let found = nfa
            .matcher()
            .longest_match(0, &['a'], |states| {
                states
                    .iter()
                    .filter_map(|&state| nfa.accept(state))
                    .copied()
                    .min()
                    .expect("an accepting state has an accept value")
            })
            .expect("the automaton accepts a");

        assert_eq!(
            found,
            Match {
                accept: 7,
                length: 1
            }
        );
    }
}
