use super::automaton::DeterministicFiniteAutomaton;
use crate::automata::automaton::Automaton;
use crate::automata::execution::Execution;
use crate::automata::id::StateId;
use crate::automata::label::Label;

/// One scan of a [`DeterministicFiniteAutomaton`], in progress.
///
/// The execution is in one state, or in no state. Thus a step makes no allocation and reads no
/// buffer. Use the same execution for each token of the input.
///
/// To make a `DeterministicExecution`, use [`Scanner::execute`](crate::automata::Scanner::execute).
#[derive(Debug)]
pub struct DeterministicExecution<'a, L> {
    dfa: &'a DeterministicFiniteAutomaton<L>,
    state: Option<StateId>,
}

impl<'a, L> DeterministicExecution<'a, L> {
    /// Creates an execution of `dfa` that is in no state.
    pub(super) fn new(dfa: &'a DeterministicFiniteAutomaton<L>) -> Self {
        Self { dfa, state: None }
    }
}

impl<L: Label> Execution for DeterministicExecution<'_, L> {
    type Symbol = L::Symbol;

    fn restart(&mut self, start: usize) {
        self.state = Some(self.dfa.start_state(start));
    }

    fn step(&mut self, symbol: Self::Symbol) -> bool {
        self.state = self.state.and_then(|id| self.dfa.step(id, symbol));
        self.state.is_some()
    }

    fn states(&self) -> &[StateId] {
        self.state.as_slice()
    }

    fn accepts(&self) -> bool {
        self.state.is_some_and(|id| self.dfa.accepts(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::scanner::Scanner;
    use crate::automata::testing::{Symbols, chain, dfa, only};

    /// Builds an automaton that has one start for `"a"` and one start for `"b"`.
    fn two_starts() -> DeterministicFiniteAutomaton<Symbols> {
        dfa(
            &[&[(only('a'), 2)], &[(only('b'), 2)], &[]],
            &[false, false, true],
            &[0, 1],
        )
    }

    /// Returns an execution of `dfa` at its first start state.
    fn execute(dfa: &DeterministicFiniteAutomaton<Symbols>) -> DeterministicExecution<'_, Symbols> {
        let mut execution = dfa.execute();
        execution.restart(0);
        execution
    }

    #[test]
    fn a_new_execution_is_in_no_state() {
        let dfa = chain();
        let execution = dfa.execute();

        assert_eq!(execution.states(), &[]);
        assert!(!execution.accepts());
    }

    #[test]
    fn an_execution_starts_at_its_start_state() {
        let dfa = chain();
        let execution = execute(&dfa);

        assert_eq!(execution.states(), &[StateId::new(0)]);
        assert!(!execution.accepts());
    }

    #[test]
    fn a_step_moves_the_execution_to_the_target_of_the_label() {
        let dfa = chain();
        let mut execution = execute(&dfa);

        assert!(execution.step('a'));
        assert_eq!(execution.states(), &[StateId::new(1)]);
        assert!(execution.accepts());
    }

    #[test]
    fn a_step_into_no_state_empties_the_execution() {
        let dfa = chain();
        let mut execution = execute(&dfa);

        assert!(!execution.step('b'));
        assert_eq!(execution.states(), &[]);
        assert!(!execution.accepts());
        assert!(!execution.step('a'));
    }

    #[test]
    fn a_restart_puts_the_execution_back_at_its_start() {
        let dfa = chain();
        let mut execution = execute(&dfa);

        assert!(!execution.step('z'));

        execution.restart(0);
        assert_eq!(execution.states(), &[StateId::new(0)]);

        assert!(execution.step('a'));
        assert_eq!(execution.states(), &[StateId::new(1)]);
    }

    #[test]
    fn each_start_gives_its_own_execution() {
        let dfa = two_starts();
        let mut execution = dfa.execute();

        execution.restart(0);
        assert_eq!(execution.states(), &[StateId::new(0)][..]);

        execution.restart(1);
        assert_eq!(execution.states(), &[StateId::new(1)][..]);
    }

    #[test]
    #[should_panic(expected = "start 2 is outside an automaton with 2 start states")]
    fn an_execution_under_a_start_the_automaton_does_not_have_panics() {
        two_starts().execute().restart(2);
    }
}
