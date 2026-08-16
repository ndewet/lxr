use super::automaton::Dfa;
use crate::automata::execution::Execution;
use crate::automata::id::{StartId, StateId};
use crate::automata::label::Label;

/// One scan of a [`Dfa`], in progress.
///
/// The execution is in one state, or in no state. Thus a step makes no allocation and reads no
/// buffer. Use the same execution for each token of the input.
///
/// To make a `DfaExecution`, use [`Automaton::execute`](crate::automata::Automaton::execute).
#[derive(Debug)]
pub struct DfaExecution<'a, L, A> {
    dfa: &'a Dfa<L, A>,
    state: Option<StateId>,
}

impl<'a, L, A> DfaExecution<'a, L, A> {
    /// Creates an execution of `dfa` that is in no state.
    pub(super) fn new(dfa: &'a Dfa<L, A>) -> Self {
        Self { dfa, state: None }
    }

    /// Returns the automaton that this execution scans.
    pub fn dfa(&self) -> &'a Dfa<L, A> {
        self.dfa
    }

    /// Returns the state that the execution is in, or `None` if the scan stopped.
    pub fn state(&self) -> Option<StateId> {
        self.state
    }
}

impl<L: Label, A> Execution for DfaExecution<'_, L, A> {
    type Symbol = L::Symbol;
    type Accept = A;

    fn restart(&mut self, start: StartId) {
        self.state = Some(self.dfa.start_state(start));
    }

    fn step(&mut self, symbol: Self::Symbol) -> bool {
        self.state = self.state.and_then(|id| self.dfa.step(id, symbol));
        self.state.is_some()
    }

    fn accepts(&self) -> impl Iterator<Item = &Self::Accept> {
        self.state.and_then(|id| self.dfa.accept(id)).into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::automaton::Automaton;
    use crate::automata::dfa::DfaBuilder;
    use crate::automata::scan::{Match, longest_match};
    use crate::automata::testing::{Symbols, only};

    /// Builds the automaton that matches `"a"` as accept 1, and `"ab"` as accept 0.
    fn dfa() -> Dfa<Symbols, u32> {
        let mut builder = DfaBuilder::new();
        let start = builder.push();
        let first = builder.push();
        let second = builder.push();
        builder.transition(start, only('a'), first);
        builder.transition(first, only('b'), second);
        builder.accept(first, 1);
        builder.accept(second, 0);
        builder
            .build(&[start])
            .expect("a test stays below the capacity")
    }

    /// Builds an automaton that has one start for `"a"` and one start for `"b"`.
    fn two_starts() -> Dfa<Symbols, u32> {
        let mut builder = DfaBuilder::new();
        let code = builder.push();
        let string = builder.push();
        let accept = builder.push();
        builder.transition(code, only('a'), accept);
        builder.transition(string, only('b'), accept);
        builder.accept(accept, 0);
        builder
            .build(&[code, string])
            .expect("a test stays below the capacity")
    }

    fn execute(dfa: &Dfa<Symbols, u32>) -> DfaExecution<'_, Symbols, u32> {
        dfa.execute(StartId::new(0))
    }

    fn scan(dfa: &Dfa<Symbols, u32>, start: usize, input: &str) -> Option<Match<u32>> {
        let start = StartId::new(start);
        let symbols: Vec<char> = input.chars().collect();
        longest_match(&mut dfa.execute(start), start, &symbols)
    }

    fn matched(accept: u32, length: usize) -> Option<Match<u32>> {
        Some(Match { accept, length })
    }

    #[test]
    fn an_execution_starts_at_its_start_state() {
        let dfa = dfa();
        let execution = execute(&dfa);

        assert_eq!(execution.state(), Some(StateId::new(0)));
        assert_eq!(execution.accepts().count(), 0);
    }

    #[test]
    fn a_step_moves_the_execution_to_the_target_of_the_label() {
        let dfa = dfa();
        let mut execution = execute(&dfa);

        assert!(execution.step('a'));
        assert_eq!(execution.state(), Some(StateId::new(1)));
        assert_eq!(execution.accepts().collect::<Vec<_>>(), vec![&1]);
    }

    #[test]
    fn a_step_into_no_state_empties_the_execution() {
        let dfa = dfa();
        let mut execution = execute(&dfa);

        assert!(!execution.step('b'));
        assert_eq!(execution.state(), None);
        assert_eq!(execution.accepts().count(), 0);
        assert!(!execution.step('a'));
    }

    #[test]
    fn an_execution_gives_a_maximum_of_one_accept() {
        let dfa = dfa();
        let mut execution = execute(&dfa);
        execution.step('a');
        execution.step('b');

        assert_eq!(execution.accepts().count(), 1);
    }

    #[test]
    fn a_restart_puts_the_execution_back_at_its_start() {
        let dfa = dfa();
        let mut execution = execute(&dfa);

        assert!(execution.step('a'));
        assert_eq!(execution.state(), Some(StateId::new(1)));

        execution.restart(StartId::new(0));
        assert_eq!(execution.state(), Some(StateId::new(0)));

        assert!(execution.step('a'));
        assert_eq!(execution.state(), Some(StateId::new(1)));
    }

    #[test]
    fn a_restart_after_a_step_into_no_state_scans_again() {
        let dfa = dfa();
        let mut execution = execute(&dfa);

        assert!(!execution.step('z'));
        execution.restart(StartId::new(0));

        assert!(execution.step('a'));
    }

    #[test]
    fn each_start_gives_its_own_execution() {
        let dfa = two_starts();

        assert_eq!(dfa.execute(StartId::new(0)).state(), Some(StateId::new(0)));
        assert_eq!(dfa.execute(StartId::new(1)).state(), Some(StateId::new(1)));
    }

    #[test]
    #[should_panic(expected = "start 2 is outside an automaton with 2 start states")]
    fn an_execution_under_a_start_the_automaton_does_not_have_panics() {
        two_starts().execute(StartId::new(2));
    }

    #[test]
    fn the_execution_gives_back_its_automaton() {
        let dfa = dfa();

        assert_eq!(execute(&dfa).dfa(), &dfa);
    }

    #[test]
    fn the_longer_match_wins() {
        let dfa = dfa();

        assert_eq!(scan(&dfa, 0, "ab"), matched(0, 2));
        assert_eq!(scan(&dfa, 0, "ac"), matched(1, 1));
    }

    #[test]
    fn a_scan_that_reaches_no_accept_gives_nothing() {
        let dfa = dfa();

        assert_eq!(scan(&dfa, 0, ""), None);
        assert_eq!(scan(&dfa, 0, "b"), None);
    }

    #[test]
    fn trailing_input_is_left_for_the_next_call() {
        assert_eq!(scan(&dfa(), 0, "abab"), matched(0, 2));
    }

    #[test]
    fn each_start_scans_only_its_own_rules() {
        let dfa = two_starts();

        assert_eq!(scan(&dfa, 0, "a"), matched(0, 1));
        assert_eq!(scan(&dfa, 0, "b"), None);
        assert_eq!(scan(&dfa, 1, "b"), matched(0, 1));
        assert_eq!(scan(&dfa, 1, "a"), None);
    }

    #[test]
    fn one_execution_scans_a_sequence_of_matches() {
        let dfa = dfa();
        let symbols: Vec<char> = "abaab".chars().collect();
        let start = StartId::new(0);
        let mut execution = dfa.execute(start);
        let mut input = &symbols[..];
        let mut accepts = Vec::new();

        while let Some(found) = longest_match(&mut execution, start, input) {
            accepts.push(found.accept);
            input = &input[found.length..];
        }

        assert_eq!(accepts, vec![0, 1, 0]);
        assert_eq!(input, []);
    }
}
