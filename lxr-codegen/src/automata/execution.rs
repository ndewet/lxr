use super::id::StateId;

/// A match that [`Execution::longest_match`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match<T> {
    /// The value that `select` gave for the states at the end of the match.
    pub accept: T,
    /// The number of the symbols in the match.
    pub length: usize,
}

/// One scan of an automaton, in progress.
///
/// An automaton holds no state of a scan. It is read only. An execution holds where the scan is,
/// and the buffers that the scan needs. Thus a step makes no allocation, and one execution scans a
/// sequence of tokens.
///
/// [`restart`](Self::restart), [`step`](Self::step), [`states`](Self::states), and
/// [`accepts`](Self::accepts) move the scan one symbol at a time.
/// [`longest_match`](Self::longest_match) reads one whole token with them.
///
/// To make an execution, use [`Scanner::execute`](super::Scanner::execute).
pub trait Execution {
    /// One symbol of the alphabet that the automaton reads.
    type Symbol: Copy;

    /// Puts the execution back at the start state that `start` refers to.
    ///
    /// # Panics
    ///
    /// This function panics if `start` is not a start state of the automaton.
    fn restart(&mut self, start: usize);

    /// Reads `symbol`, then moves the execution.
    ///
    /// Returns `false` if the execution reaches no state. The execution then accepts nothing, and
    /// each later step also gives `false`. To scan again, use [`restart`](Self::restart).
    fn step(&mut self, symbol: Self::Symbol) -> bool;

    /// Returns the states that the execution is in.
    ///
    /// A deterministic automaton gives no state or one state. A nondeterministic automaton gives
    /// each state of its set, in ascending sequence and with no duplicate.
    fn states(&self) -> &[StateId];

    /// Returns `true` if a state that the execution is in accepts.
    ///
    /// The automaton says which states accept. The caller reads
    /// [`states`](Self::states) to get the meaning of the accept.
    fn accepts(&self) -> bool;

    /// Returns the longest match at the start of `input` under `start`.
    ///
    /// The automaton says where a match ends. It says which states accept, and it does not say
    /// what an accept means. `select` gives that meaning. The function calls `select` with the
    /// states of the execution, and only at a length at which the execution accepts. A lexer, for
    /// example, reads its table of tokens and gives the token of the rule of the highest
    /// precedence.
    ///
    /// The function returns `None` if the scan reaches no state that accepts. A start state that
    /// accepts before it reads a symbol gives a match of length 0. A scanner that moves forward by
    /// the length of the match then does not move. Thus reject a rule that matches no symbol, or
    /// move the scanner forward by one symbol.
    ///
    /// Only the rules of `start` are applicable. The other start states take no part in the scan.
    ///
    /// The function scans one token. Call it again on the same execution for the next token. Thus
    /// the scan makes the buffers one time, and not one time for each token.
    ///
    /// # Panics
    ///
    /// This function panics if `start` is not a start state of the automaton.
    fn longest_match<T>(
        &mut self,
        start: usize,
        input: &[Self::Symbol],
        select: impl Fn(&[StateId]) -> T,
    ) -> Option<Match<T>> {
        self.restart(start);
        let mut best = accepted(self, &select, 0);

        for (consumed, &symbol) in input.iter().enumerate() {
            if !self.step(symbol) {
                break;
            }
            best = accepted(self, &select, consumed + 1).or(best);
        }

        best
    }
}

/// Returns the match of `length` that `execution` reached, or `None` if it accepts nothing.
fn accepted<E: Execution + ?Sized, T>(
    execution: &E,
    select: &impl Fn(&[StateId]) -> T,
    length: usize,
) -> Option<Match<T>> {
    execution.accepts().then(|| Match {
        accept: select(execution.states()),
        length,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::automaton::Automaton;
    use crate::automata::nfa::{NfaBuilder, NondeterministicFiniteAutomaton};
    use crate::automata::scanner::Scanner;
    use crate::automata::testing::{Symbols, builder, literal, star};

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
    fn scan(
        nfa: &NondeterministicFiniteAutomaton<Symbols>,
        table: &[Option<u32>],
        input: &str,
    ) -> Option<Match<u32>> {
        scan_under(nfa, table, 0, input)
    }

    /// Returns the longest match at the start of `input` under `start`, with the lowest accept of
    /// the table.
    fn scan_under(
        nfa: &NondeterministicFiniteAutomaton<Symbols>,
        table: &[Option<u32>],
        start: usize,
        input: &str,
    ) -> Option<Match<u32>> {
        let symbols: Vec<char> = input.chars().collect();
        nfa.execute()
            .longest_match(start, &symbols, |states| lowest(table, states))
    }

    /// Builds an automaton of one rule for each word, and the table of their accepts.
    ///
    /// Each word gets its own start state, at the index of the word. The rule of a word accepts
    /// that index. Thus a scan under one start reads only the word of that start.
    fn conditions(words: &[&str]) -> (NondeterministicFiniteAutomaton<Symbols>, Vec<Option<u32>>) {
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
    fn alternation(
        first: &str,
        second: &str,
    ) -> (NondeterministicFiniteAutomaton<Symbols>, Vec<Option<u32>>) {
        let mut builder = NfaBuilder::new();
        let left = literal(&mut builder, first);
        let right = literal(&mut builder, second);
        let start = builder.push();
        builder.epsilon(start, left.entry);
        builder.epsilon(start, right.entry);
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
        let start = builder.push();
        builder.accept(start);
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
        let stuck = builder.push();
        builder.epsilon(stuck, stuck);
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
    fn one_execution_scans_a_sequence_of_matches() {
        let (nfa, table) = alternation("if", " ");
        let mut execution = nfa.execute();
        let symbols: Vec<char> = "if if".chars().collect();
        let mut input = &symbols[..];
        let mut found = Vec::new();

        while let Some(scanned) = execution.longest_match(0, input, |states| lowest(&table, states))
        {
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
            .execute()
            .longest_match(0, &symbols, <[StateId]>::to_vec)
            .expect("the automaton accepts ab");

        assert_eq!(found.length, 2);
        assert_eq!(found.accept, vec![StateId::new(4)]);
    }
}
