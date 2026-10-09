use super::{Nfa, epsilon_closure};
use crate::automata::id::StateId;
use crate::automata::label::Label;
use crate::automata::state_set::StateSet;

/// Tracks the active state set for one [`Nfa`] execution.
///
/// The state set is epsilon-closed. The execution reuses its buffers between
/// steps.
#[derive(Debug)]
pub(crate) struct Execution<'a, L, A = ()> {
    nfa: &'a Nfa<L, A>,
    states: StateSet,
    next: Vec<StateId>,
}

impl<'a, L: Label, A> Execution<'a, L, A> {
    /// Creates an execution with no active state.
    pub(in crate::automata) fn new(nfa: &'a Nfa<L, A>) -> Self {
        Self {
            nfa,
            states: StateSet::new(nfa.state_count()),
            next: Vec::new(),
        }
    }

    fn seed(&mut self, states: &[StateId]) {
        epsilon_closure(self.nfa, states, &mut self.states);
    }

    /// Restarts the execution at start-state index `start`.
    ///
    /// # Panics
    ///
    /// This function panics if `start` is not a start state of the automaton.
    pub(crate) fn restart(&mut self, start: usize) {
        self.seed(&[self.nfa.start_state(start)]);
    }

    /// Reads `symbol` and updates the active state set.
    ///
    /// Returns `false` if no state remains active.
    pub(crate) fn step(&mut self, symbol: L::Symbol) -> bool {
        self.next.clear();
        self.next
            .extend(self.nfa.step(self.states.members(), symbol));
        epsilon_closure(self.nfa, &self.next, &mut self.states);
        !self.states.is_empty()
    }

    /// Returns the active states in ascending sequence.
    ///
    /// The result contains no duplicates.
    pub(crate) fn states(&self) -> &[StateId] {
        self.states.members()
    }

    /// Returns whether an active state accepts.
    pub(crate) fn accepts(&self) -> bool {
        self.states().iter().any(|&state| self.nfa.accepts(state))
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::automata::testing::{Symbols, builder, literal, only};

    fn execution(nfa: &Nfa<Symbols>) -> Execution<'_, Symbols> {
        let mut execution = nfa.execution();
        execution.restart(0);
        execution
    }

    #[test]
    fn an_execution_starts_at_the_closure_of_its_start_state() {
        let mut builder = builder();
        let start = builder.add_state();
        let accept = builder.add_state();
        builder.add_epsilon_transition(start, accept);
        builder.mark_accept(accept);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        let execution = execution(&nfa);

        assert_eq!(execution.states(), &[start, accept]);
        assert!(execution.accepts());
    }

    #[test]
    fn a_step_into_no_state_empties_the_execution() {
        let mut builder = builder();
        let start = builder.add_state();
        let accept = builder.add_state();
        builder.add_transition(start, only('a'), accept);
        builder.mark_accept(accept);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        let mut execution = execution(&nfa);

        assert!(!execution.step('b'));
        assert_eq!(execution.states(), &[]);
        assert!(!execution.accepts());
        assert!(!execution.step('a'));
    }

    #[test]
    fn an_execution_accepts_if_one_state_of_its_set_accepts() {
        let mut builder = builder();
        let start = builder.add_state();
        let first = builder.add_state();
        let second = builder.add_state();
        builder.add_transition(start, only('a'), first);
        builder.add_transition(start, only('a'), second);
        builder.mark_accept(second);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        let mut execution = execution(&nfa);

        assert!(execution.step('a'));
        assert_eq!(execution.states(), &[first, second]);
        assert!(execution.accepts());
    }

    #[test]
    fn a_step_keeps_a_shared_target_one_time() {
        let mut builder = builder();
        let start = builder.add_state();
        let target = builder.add_state();
        builder.add_transition(start, only('a'), target);
        builder.add_transition(start, only('a'), target);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        let mut execution = execution(&nfa);

        assert!(execution.step('a'));
        assert_eq!(execution.states(), &[target]);
    }

    #[test]
    fn a_restart_puts_the_execution_back_at_its_start() {
        let mut builder = builder();
        let start = builder.add_state();
        let accept = builder.add_state();
        builder.add_transition(start, only('a'), accept);
        builder.mark_accept(accept);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        let mut execution = execution(&nfa);

        assert!(execution.step('a'));
        assert_eq!(execution.states(), &[accept]);

        execution.restart(0);
        assert_eq!(execution.states(), &[start]);

        assert!(execution.step('a'));
        assert_eq!(execution.states(), &[accept]);
    }

    #[test]
    fn each_start_seeds_only_its_own_state() {
        let mut builder = builder();
        let code = builder.add_state();
        let string = builder.add_state();
        builder.mark_accept(string);
        let nfa = builder
            .build(&[code, string])
            .expect("the builder is below its capacity");
        let mut execution = nfa.execution();

        execution.restart(0);
        assert!(!execution.accepts());

        execution.restart(1);
        assert!(execution.accepts());
    }

    #[test]
    fn a_new_execution_is_in_no_state() {
        let mut builder = builder();
        let start = builder.add_state();
        builder.mark_accept(start);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        let execution = nfa.execution();

        assert_eq!(execution.states(), &[]);
        assert!(!execution.accepts());
    }

    #[test]
    #[should_panic(expected = "start 2 is outside an automaton with 2 start states")]
    fn an_execution_under_a_start_the_automaton_does_not_have_panics() {
        let mut builder = builder();
        let code = literal(&mut builder, "a");
        let string = literal(&mut builder, "b");
        let nfa = builder
            .build(&[code.entry, string.entry])
            .expect("the builder is below its capacity");

        nfa.execution().restart(2);
    }
}
