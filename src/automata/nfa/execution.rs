use super::automaton::Nfa;
use crate::automata::execution::Execution;
use crate::automata::id::{StartId, StateId};
use crate::automata::label::Label;

/// One scan of an [`Nfa`], in progress.
///
/// The execution holds the set of the states that the scan is in, and the scratch space of the
/// epsilon closure. Thus a step makes no allocation. Use the same execution for each token of the
/// input.
///
/// To make an `NfaExecution`, use [`Automaton::execute`](crate::automata::Automaton::execute).
#[derive(Debug)]
pub struct NfaExecution<'a, L, A> {
    nfa: &'a Nfa<L, A>,
    current: Vec<StateId>,
    next: Vec<StateId>,
    reached: Vec<bool>,
    pending: Vec<StateId>,
}

impl<'a, L, A> NfaExecution<'a, L, A> {
    /// Creates an execution of `nfa` that is in no state.
    pub(super) fn new(nfa: &'a Nfa<L, A>) -> Self {
        Self {
            nfa,
            current: Vec::new(),
            next: Vec::new(),
            reached: vec![false; nfa.state_count()],
            pending: Vec::new(),
        }
    }

    /// Returns the automaton that this execution scans.
    pub fn nfa(&self) -> &'a Nfa<L, A> {
        self.nfa
    }

    /// Returns the states that the execution is in.
    ///
    /// The states are in ascending sequence, and the result holds no duplicate.
    pub fn states(&self) -> &[StateId] {
        &self.current
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

impl<L: Label, A> Execution for NfaExecution<'_, L, A> {
    type Symbol = L::Symbol;
    type Accept = A;

    fn restart(&mut self, start: StartId) {
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

        nfa.step(current, symbol, next);
        if next.is_empty() {
            current.clear();
            return false;
        }

        closure(nfa, reached, pending, next, current);
        true
    }

    fn accepts(&self) -> impl Iterator<Item = &Self::Accept> {
        self.current.iter().filter_map(|&id| self.nfa.accept(id))
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
fn closure<L, A>(
    nfa: &Nfa<L, A>,
    reached: &mut [bool],
    pending: &mut Vec<StateId>,
    seeds: &[StateId],
    out: &mut Vec<StateId>,
) {
    for &seed in seeds {
        debug_assert!(
            seed.index() < nfa.state_count(),
            "state {} is outside an arena of {} states",
            seed.index(),
            nfa.state_count()
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
    use super::super::builder::NfaBuilder;
    use super::super::reference::built;
    use super::*;
    use crate::automata::automaton::Automaton;
    use crate::automata::reference::{Symbols, only};

    fn builder() -> NfaBuilder<Symbols, u32> {
        NfaBuilder::new()
    }

    fn seeded(nfa: &Nfa<Symbols, u32>, seeds: &[StateId]) -> Vec<usize> {
        let mut execution = NfaExecution::new(nfa);
        execution.seed(seeds);
        execution.states().iter().map(|id| id.index()).collect()
    }

    fn execute(nfa: &Nfa<Symbols, u32>) -> NfaExecution<'_, Symbols, u32> {
        nfa.execute(StartId::new(0))
    }

    #[test]
    fn the_closure_of_a_state_without_an_epsilon_transition_is_just_itself() {
        let mut builder = builder();
        let accept = builder.push();
        builder.accept(accept, 0);
        let nfa = built(builder, &[accept]);

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
        let nfa = built(builder, &[start]);

        assert_eq!(seeded(&nfa, &[start]), vec![0, 1, 2, 3]);
    }

    #[test]
    fn the_closure_stops_at_a_transition_with_a_label() {
        let mut builder = builder();
        let start = builder.push();
        let accept = builder.push();
        builder.transition(start, only('a'), accept);
        let nfa = built(builder, &[start]);

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
        let nfa = built(builder, &[left]);

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
        let nfa = built(builder, &[left]);

        assert_eq!(seeded(&nfa, &[right, left, right]), vec![0, 1, 2, 3]);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "state 9 is outside an arena of 2 states")]
    fn a_closure_over_a_seed_outside_the_arena_panics() {
        let mut builder = builder();
        let start = builder.push();
        builder.push();
        let nfa = built(builder, &[start]);

        seeded(&nfa, &[StateId::new(9)]);
    }

    #[test]
    fn an_execution_starts_at_the_closure_of_its_start_state() {
        let mut builder = builder();
        let start = builder.push();
        let accept = builder.push();
        builder.epsilon(start, accept);
        builder.accept(accept, 0);
        let nfa = built(builder, &[start]);

        let execution = execute(&nfa);

        assert_eq!(execution.states(), &[start, accept]);
        assert_eq!(execution.accepts().collect::<Vec<_>>(), vec![&0]);
    }

    #[test]
    fn a_step_into_no_state_empties_the_execution() {
        let mut builder = builder();
        let start = builder.push();
        let accept = builder.push();
        builder.transition(start, only('a'), accept);
        builder.accept(accept, 0);
        let nfa = built(builder, &[start]);

        let mut execution = execute(&nfa);

        assert!(!execution.step('b'));
        assert_eq!(execution.states(), &[]);
        assert_eq!(execution.accepts().count(), 0);
        assert!(!execution.step('a'));
    }

    #[test]
    fn an_execution_reports_each_accept_that_it_reached() {
        let mut builder = builder();
        let start = builder.push();
        let first = builder.push();
        let second = builder.push();
        builder.transition(start, only('a'), first);
        builder.transition(start, only('a'), second);
        builder.accept(first, 7);
        builder.accept(second, 3);
        let nfa = built(builder, &[start]);

        let mut execution = execute(&nfa);

        assert!(execution.step('a'));
        assert_eq!(execution.accepts().collect::<Vec<_>>(), vec![&7, &3]);
    }

    #[test]
    fn a_restart_puts_the_execution_back_at_its_start() {
        let mut builder = builder();
        let start = builder.push();
        let accept = builder.push();
        builder.transition(start, only('a'), accept);
        builder.accept(accept, 0);
        let nfa = built(builder, &[start]);

        let mut execution = execute(&nfa);

        assert!(execution.step('a'));
        assert_eq!(execution.states(), &[accept]);

        execution.restart(StartId::new(0));
        assert_eq!(execution.states(), &[start]);

        assert!(execution.step('a'));
        assert_eq!(execution.states(), &[accept]);
    }

    #[test]
    fn each_start_gives_its_own_execution() {
        let mut builder = builder();
        let code = builder.push();
        let string = builder.push();
        let nfa = built(builder, &[code, string]);

        assert_eq!(nfa.execute(StartId::new(0)).states(), &[code]);
        assert_eq!(nfa.execute(StartId::new(1)).states(), &[string]);
    }

    #[test]
    #[should_panic(expected = "start 2 is outside an automaton with 2 start states")]
    fn an_execution_under_a_start_the_automaton_does_not_have_panics() {
        let mut builder = builder();
        let code = builder.push();
        let string = builder.push();
        let nfa = built(builder, &[code, string]);

        nfa.execute(StartId::new(2));
    }

    #[test]
    fn the_execution_gives_back_its_automaton() {
        let mut builder = builder();
        let state = builder.push();
        let nfa = built(builder, &[state]);

        assert_eq!(execute(&nfa).nfa(), &nfa);
    }
}
