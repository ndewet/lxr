use super::{Nfa, epsilon_closure};
use crate::automata::id::StateId;
use crate::automata::label::Label;
use crate::automata::state_set::StateSet;

/// Holds the state set for one [`Nfa`] execution.
///
/// An execution holds the epsilon-closed set of states that the NFA is in.
/// [`restart`](Self::restart) selects a start state and [`step`](Self::step)
/// applies the NFA transition relation to one symbol. The execution reuses its
/// buffers. Thus a step makes no allocation after the buffers grow to the
/// necessary size.
///
/// To make an `Execution`, use [`execution`](Nfa::execution).
#[derive(Debug)]
pub struct Execution<'a, L, A = ()> {
    nfa: &'a Nfa<L, A>,
    states: StateSet,
    next: Vec<StateId>,
}

impl<'a, L: Label, A> Execution<'a, L, A> {
    /// Creates an execution of `nfa` that is in no state.
    ///
    /// A scan puts the execution at a start state with [`restart`](Self::restart).
    pub(in crate::automata) fn new(nfa: &'a Nfa<L, A>) -> Self {
        Self {
            nfa,
            states: StateSet::new(nfa.state_count()),
            next: Vec::new(),
        }
    }

    /// Puts the execution in `states`, and in each state that `states` goes to without a symbol.
    ///
    /// # Panics
    ///
    /// This function panics if a state in `states` is not in the automaton.
    fn seed(&mut self, states: &[StateId]) {
        epsilon_closure(self.nfa, states, &mut self.states);
    }

    /// Puts the execution back at the start state that `start` refers to.
    ///
    /// # Panics
    ///
    /// This function panics if `start` is not a start state of the automaton.
    pub fn restart(&mut self, start: usize) {
        self.seed(&[self.nfa.start_state(start)]);
    }

    /// Reads `symbol`, then moves the execution.
    ///
    /// Returns `false` if the execution reaches no state. The execution then accepts nothing, and
    /// each later step also gives `false`. To scan again, use [`restart`](Self::restart).
    pub fn step(&mut self, symbol: L::Symbol) -> bool {
        self.next.clear();
        self.next
            .extend(self.nfa.step(self.states.members(), symbol));
        epsilon_closure(self.nfa, &self.next, &mut self.states);
        !self.states.is_empty()
    }

    /// Returns the states that the execution is in.
    ///
    /// The states are in ascending sequence, and the result holds no duplicate.
    pub fn states(&self) -> &[StateId] {
        self.states.members()
    }

    /// Returns `true` if a state that the execution is in accepts.
    ///
    /// The caller can read [`states`](Self::states), then get each accept value
    /// with [`Nfa::accept`].
    pub fn accepts(&self) -> bool {
        self.states().iter().any(|&state| self.nfa.accepts(state))
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::automata::testing::{Symbols, builder, literal, only};

    /// Returns an execution of `nfa` at its first start state.
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
