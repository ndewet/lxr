use super::automaton::NondeterministicFiniteAutomaton;
use crate::automata::automaton::Automaton;
use crate::automata::execution::Execution;
use crate::automata::id::StateId;
use crate::automata::label::Label;

/// One scan of an [`NondeterministicFiniteAutomaton`], in progress.
///
/// The execution holds the set of the states that the scan is in, and the scratch space of the
/// epsilon closure. Thus a step makes no allocation. Use the same execution for each token of the
/// input.
///
/// To make an `NfaExecution`, use [`Automaton::execute`](crate::automata::Automaton::execute).
#[derive(Debug)]
pub struct NondeterministicExecution<'a, L> {
    nfa: &'a NondeterministicFiniteAutomaton<L>,
    current: Vec<StateId>,
    next: Vec<StateId>,
    reached: Vec<bool>,
    pending: Vec<StateId>,
}

impl<'a, L: Label> NondeterministicExecution<'a, L> {
    /// Creates an execution of `nfa` that is in no state.
    ///
    /// Determinization makes one execution, then it seeds that execution for each set of states.
    pub(in crate::automata) fn new(nfa: &'a NondeterministicFiniteAutomaton<L>) -> Self {
        Self {
            nfa,
            current: Vec::new(),
            next: Vec::new(),
            reached: vec![false; nfa.state_count()],
            pending: Vec::new(),
        }
    }

    /// Puts the execution in `states`, and in each state that `states` goes to without a symbol.
    ///
    /// Determinization seeds an execution with the states of one subset, then it steps.
    ///
    /// # Panics
    ///
    /// This function panics if a state in `states` is not in the state arena.
    pub fn seed(&mut self, states: &[StateId]) {
        let Self {
            nfa,
            current,
            reached,
            pending,
            ..
        } = self;
        closure(nfa, reached, pending, states, current);
    }
}

impl<L: Label> Execution for NondeterministicExecution<'_, L> {
    type Symbol = L::Symbol;

    fn restart(&mut self, start: usize) {
        self.seed(&[self.nfa.start_state(start)]);
    }

    fn step(&mut self, symbol: Self::Symbol) -> bool {
        let Self {
            nfa,
            current,
            next,
            reached,
            pending,
        } = self;

        next.clear();
        next.extend(nfa.step(current, symbol));
        if next.is_empty() {
            current.clear();
            return false;
        }

        closure(nfa, reached, pending, next, current);
        true
    }

    fn states(&self) -> &[StateId] {
        &self.current
    }

    fn accepts(&self) -> bool {
        self.current.iter().any(|&id| self.nfa.accepts(id))
    }
}

/// Finds each state that `seeds` goes to without a symbol, then writes the states into `out`.
///
/// The function clears `out` first. The result holds the seeds. The result is in ascending
/// sequence and holds no duplicate.
///
/// `reached` and `pending` are the scratch space. The function gives them back empty, thus the
/// next call reuses them.
///
/// # Panics
///
/// This function panics if a state in `seeds` is not in the state arena. The check is a
/// `debug_assert!`, because `step` calls this function one time for each symbol.
fn closure<L>(
    nfa: &NondeterministicFiniteAutomaton<L>,
    reached: &mut [bool],
    pending: &mut Vec<StateId>,
    seeds: &[StateId],
    out: &mut Vec<StateId>,
) {
    for &seed in seeds {
        debug_assert!(
            seed.index() < reached.len(),
            "state {} is outside an arena of {} states",
            seed.index(),
            reached.len()
        );
    }

    out.clear();
    pending.clear();

    for &seed in seeds {
        if !reached[seed.index()] {
            reached[seed.index()] = true;
            pending.push(seed);
        }
    }

    while let Some(id) = pending.pop() {
        out.push(id);
        for &target in nfa.epsilons(id) {
            if !reached[target.index()] {
                reached[target.index()] = true;
                pending.push(target);
            }
        }
    }

    for &id in out.iter() {
        reached[id.index()] = false;
    }

    out.sort_unstable();
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::automata::scanner::Scanner;
    use crate::automata::testing::{Symbols, builder, literal, only};

    fn seeded(nfa: &NondeterministicFiniteAutomaton<Symbols>, seeds: &[StateId]) -> Vec<usize> {
        let mut execution = NondeterministicExecution::new(nfa);
        execution.seed(seeds);
        execution.states().iter().map(|id| id.index()).collect()
    }

    fn execute(
        nfa: &NondeterministicFiniteAutomaton<Symbols>,
    ) -> NondeterministicExecution<'_, Symbols> {
        nfa.execute(0)
    }

    #[test]
    fn the_closure_of_a_state_without_an_epsilon_transition_is_just_itself() {
        let mut builder = builder();
        let accept = builder.push();
        let nfa = builder
            .build(&[accept])
            .expect("the builder is below its capacity");

        assert_eq!(seeded(&nfa, &[accept]), vec![0]);
    }

    #[test]
    fn the_closure_follows_each_epsilon_transition_of_a_state() {
        let mut builder = builder();
        let start = builder.push();
        let left = builder.push();
        let middle = builder.push();
        let right = builder.push();
        builder.epsilon(start, left);
        builder.epsilon(start, middle);
        builder.epsilon(start, right);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        assert_eq!(seeded(&nfa, &[start]), vec![0, 1, 2, 3]);
    }

    #[test]
    fn the_closure_stops_at_a_transition_with_a_label() {
        let mut builder = builder();
        let start = builder.push();
        let accept = builder.push();
        builder.transition(start, only('a'), accept);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        assert_eq!(seeded(&nfa, &[start]), vec![0]);
    }

    #[test]
    fn the_closure_terminates_on_an_epsilon_cycle() {
        let mut builder = builder();
        let left = builder.push();
        let right = builder.push();
        let accept = builder.push();
        builder.epsilon(left, right);
        builder.epsilon(left, accept);
        builder.epsilon(right, left);
        builder.epsilon(right, accept);
        let nfa = builder
            .build(&[left])
            .expect("the builder is below its capacity");

        assert_eq!(seeded(&nfa, &[left]), vec![0, 1, 2]);
    }

    #[test]
    fn the_closure_is_sorted_and_deduplicated() {
        let mut builder = builder();
        let first = builder.push();
        let second = builder.push();
        let left = builder.push();
        let right = builder.push();
        builder.epsilon(left, first);
        builder.epsilon(left, second);
        builder.epsilon(right, second);
        builder.epsilon(right, first);
        let nfa = builder
            .build(&[left])
            .expect("the builder is below its capacity");

        assert_eq!(seeded(&nfa, &[right, left, right]), vec![0, 1, 2, 3]);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "state 9 is outside an arena of 2 states")]
    fn a_closure_over_a_seed_outside_the_arena_panics() {
        let mut builder = builder();
        let start = builder.push();
        builder.push();
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        seeded(&nfa, &[StateId::new(9)]);
    }

    #[test]
    fn an_execution_starts_at_the_closure_of_its_start_state() {
        let mut builder = builder();
        let start = builder.push();
        let accept = builder.push();
        builder.epsilon(start, accept);
        builder.accept(accept);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        let execution = execute(&nfa);

        assert_eq!(execution.states(), &[start, accept]);
        assert!(execution.accepts());
    }

    #[test]
    fn a_step_into_no_state_empties_the_execution() {
        let mut builder = builder();
        let start = builder.push();
        let accept = builder.push();
        builder.transition(start, only('a'), accept);
        builder.accept(accept);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        let mut execution = execute(&nfa);

        assert!(!execution.step('b'));
        assert_eq!(execution.states(), &[]);
        assert!(!execution.accepts());
        assert!(!execution.step('a'));
    }

    #[test]
    fn an_execution_accepts_if_one_state_of_its_set_accepts() {
        let mut builder = builder();
        let start = builder.push();
        let first = builder.push();
        let second = builder.push();
        builder.transition(start, only('a'), first);
        builder.transition(start, only('a'), second);
        builder.accept(second);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        let mut execution = execute(&nfa);

        assert!(execution.step('a'));
        assert_eq!(execution.states(), &[first, second]);
        assert!(execution.accepts());
    }

    #[test]
    fn a_restart_puts_the_execution_back_at_its_start() {
        let mut builder = builder();
        let start = builder.push();
        let accept = builder.push();
        builder.transition(start, only('a'), accept);
        builder.accept(accept);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        let mut execution = execute(&nfa);

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
        let code = builder.push();
        let string = builder.push();
        builder.accept(string);
        let nfa = builder
            .build(&[code, string])
            .expect("the builder is below its capacity");

        assert!(!nfa.execute(0).accepts());
        assert!(nfa.execute(1).accepts());
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

        nfa.execute(2);
    }
}
