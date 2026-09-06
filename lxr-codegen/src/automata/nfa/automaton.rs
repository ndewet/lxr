use super::{Execution, Matcher};
use crate::automata::Transition;
use crate::automata::adjacency::AdjacencyList;
use crate::automata::id::StateId;
use crate::automata::label::Label;
use crate::automata::table::StateTable;

/// Stores a nondeterministic finite automaton.
///
/// An NFA permits overlapping labels and epsilon transitions. The accept type
/// identifies lexer rules without defining their precedence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Nfa<L, A = ()> {
    table: StateTable<L, A>,
    epsilons: AdjacencyList<StateId>,
}

impl<L, A> Nfa<L, A> {
    /// Creates an NFA from its validated storage.
    ///
    /// # Panics
    ///
    /// This function panics if the storage sizes differ or an epsilon target is invalid.
    pub(super) fn new(table: StateTable<L, A>, epsilons: AdjacencyList<StateId>) -> Self {
        let count = table.state_count();
        assert_eq!(
            epsilons.state_count(),
            count,
            "an automaton needs one group of epsilon transitions for each of its {count} states"
        );
        for index in 0..count {
            let state = StateId::new(index);
            for &target in epsilons.get(state).into_iter().flatten() {
                table.check_target(state, target);
            }
        }

        Self { table, epsilons }
    }

    /// Returns the epsilon-transition targets from `state`.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not in the automaton.
    pub(crate) fn epsilon_targets(&self, state: StateId) -> &[StateId] {
        self.epsilons
            .get(state)
            .unwrap_or_else(|| state.outside(self.state_count()))
    }

    /// Returns the number of states in the automaton.
    pub(crate) fn state_count(&self) -> usize {
        self.table.state_count()
    }

    /// Returns the transitions from `state`.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not in the automaton.
    pub(crate) fn transitions(&self, state: StateId) -> &[Transition<L>] {
        self.table.transitions(state)
    }

    /// Returns `true` if `state` accepts.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not in the automaton.
    pub(crate) fn accepts(&self, state: StateId) -> bool {
        self.table.accepts(state)
    }

    /// Returns the accept value of `state`, if the state accepts.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not in the automaton.
    pub(crate) fn accept(&self, state: StateId) -> Option<&A> {
        self.table.accept(state)
    }

    /// Returns the start states in declaration sequence.
    pub(crate) fn start_states(&self) -> &[StateId] {
        self.table.start_states()
    }

    /// Returns the start state at `index`.
    ///
    /// # Panics
    ///
    /// This function panics if `index` is outside the start states.
    pub(crate) fn start_state(&self, index: usize) -> StateId {
        self.table.start_state(index)
    }
}

impl<L: Label, A> Nfa<L, A> {
    /// Returns each target reached from `states` with `symbol`.
    ///
    /// This method does not follow epsilon transitions or remove duplicates.
    ///
    /// # Panics
    ///
    /// This function panics if `states` contains an invalid identifier.
    /// The panic occurs when the caller reads the invalid state from the iterator.
    pub(crate) fn step<'a>(
        &'a self,
        states: &'a [StateId],
        symbol: L::Symbol,
    ) -> impl Iterator<Item = StateId> + 'a {
        states.iter().flat_map(move |&id| {
            self.transitions(id)
                .iter()
                .filter(move |transition| transition.label.matches(symbol))
                .map(|transition| transition.target)
        })
    }

    /// Creates an empty [`Execution`] for this NFA.
    pub(crate) fn execution(&self) -> Execution<'_, L, A> {
        Execution::new(self)
    }

    /// Creates a reusable longest-match [`Matcher`].
    pub(crate) fn matcher(&self) -> Matcher<'_, L, A> {
        Matcher::new(self)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::automata::testing::{Symbols, builder, only, range};

    fn stepped(nfa: &Nfa<Symbols>, states: &[StateId], symbol: char) -> Vec<StateId> {
        nfa.step(states, symbol).collect()
    }

    #[test]
    fn a_step_follows_a_transition_that_matches_the_symbol() {
        let mut builder = builder();
        let start = builder.add_state();
        let accept = builder.add_state();
        builder.add_transition(start, range('a', 'z'), accept);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        assert_eq!(stepped(&nfa, &[start], 'm'), vec![accept]);
        assert_eq!(stepped(&nfa, &[start], 'A'), Vec::new());
    }

    #[test]
    fn a_step_does_not_follow_an_epsilon_transition() {
        let mut builder = builder();
        let start = builder.add_state();
        let accept = builder.add_state();
        builder.add_epsilon_transition(start, accept);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        assert_eq!(stepped(&nfa, &[start], 'a'), Vec::new());
    }

    #[test]
    fn a_step_yields_a_shared_target_one_time_for_each_transition() {
        let mut builder = builder();
        let start = builder.add_state();
        let accept = builder.add_state();
        builder.add_transition(start, range('a', 'z'), accept);
        builder.add_transition(start, only('e'), accept);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        assert_eq!(stepped(&nfa, &[start], 'e'), vec![accept, accept]);
    }

    #[test]
    fn a_state_keeps_its_transitions_in_the_sequence_in_which_they_arrived() {
        let mut builder = builder();
        let start = builder.add_state();
        let first = builder.add_state();
        let second = builder.add_state();
        builder.add_transition(start, only('b'), second);
        builder.add_transition(start, only('a'), first);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        assert_eq!(
            nfa.transitions(start),
            &[
                Transition {
                    label: only('b'),
                    target: second
                },
                Transition {
                    label: only('a'),
                    target: first
                },
            ]
        );
        assert_eq!(nfa.transitions(first), &[]);
    }

    #[test]
    fn a_state_keeps_its_epsilon_transitions() {
        let mut builder = builder();
        let start = builder.add_state();
        let first = builder.add_state();
        let second = builder.add_state();
        builder.add_epsilon_transition(start, second);
        builder.add_epsilon_transition(start, first);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        assert_eq!(nfa.epsilon_targets(start), &[second, first]);
        assert_eq!(nfa.epsilon_targets(first), &[]);
    }

    #[test]
    fn only_a_state_that_the_builder_marked_accepts() {
        let mut builder = builder();
        let start = builder.add_state();
        let accept = builder.add_state();
        builder.mark_accept(accept);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        assert!(!nfa.accepts(start));
        assert!(nfa.accepts(accept));
        assert_eq!(nfa.state_count(), 2);
    }

    #[test]
    fn each_start_names_its_own_state() {
        let mut builder = builder();
        let code = builder.add_state();
        let string = builder.add_state();
        let nfa = builder
            .build(&[code, string])
            .expect("the builder is below its capacity");

        assert_eq!(nfa.start_states(), &[code, string]);
        assert_eq!(nfa.start_state(0), code);
        assert_eq!(nfa.start_state(1), string);
    }

    #[test]
    #[should_panic(expected = "start 2 is outside an automaton with 2 start states")]
    fn reading_a_start_outside_the_automaton_panics() {
        let mut builder = builder();
        let code = builder.add_state();
        let string = builder.add_state();
        let nfa = builder
            .build(&[code, string])
            .expect("the builder is below its capacity");

        nfa.start_state(2);
    }

    #[test]
    #[should_panic(expected = "state 9 is outside an automaton of 2 states")]
    fn reading_the_transitions_of_a_state_outside_the_arena_panics() {
        let mut builder = builder();
        let start = builder.add_state();
        builder.add_state();
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        nfa.transitions(StateId::new(9));
    }

    #[test]
    #[should_panic(expected = "state 9 is outside an automaton of 2 states")]
    fn reading_the_accept_of_a_state_outside_the_arena_panics() {
        let mut builder = builder();
        let start = builder.add_state();
        builder.add_state();
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        nfa.accepts(StateId::new(9));
    }
}
