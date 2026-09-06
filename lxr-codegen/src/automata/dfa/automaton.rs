use crate::automata::{Label, StateId, Transition, table::StateTable};

/// Stores a deterministic finite automaton.
///
/// Outgoing labels from one state do not overlap. A missing transition moves
/// to an implicit dead state.
pub(crate) struct Dfa<L, A = ()> {
    table: StateTable<L, A>,
}

impl<L, A> Dfa<L, A> {
    /// Creates a DFA from a validated state table.
    pub(super) fn new(table: StateTable<L, A>) -> Self {
        Self { table }
    }

    /// Returns the number of states in the DFA.
    pub(crate) fn state_count(&self) -> usize {
        self.table.state_count()
    }

    /// Returns the transitions from `state`.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not in the DFA.
    pub(crate) fn transitions(&self, state: StateId) -> &[Transition<L>] {
        self.table.transitions(state)
    }

    /// Returns `true` if `state` accepts.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not in the DFA.
    pub(crate) fn accepts(&self, state: StateId) -> bool {
        self.table.accepts(state)
    }

    /// Returns the accept value of `state`, if the state accepts.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not in the DFA.
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

impl<L: Label, A> Dfa<L, A> {
    /// Returns the target that matches `symbol`, or `None` for the dead state.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not in the DFA.
    pub(crate) fn step(&self, state: StateId, symbol: L::Symbol) -> Option<StateId> {
        let transition: Option<&Transition<L>> = self
            .transitions(state)
            .iter()
            .find(|transition| transition.label.matches(symbol));
        transition.map(|transition| transition.target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::table::StateTableBuilder;
    use crate::automata::testing::{Symbols, only, range};
    use std::cell::Cell;
    use std::rc::Rc;

    fn example() -> Dfa<Symbols, &'static str> {
        let mut table = StateTableBuilder::new();
        let start = table.add_state();
        let accept = table.add_state();
        let other_start = table.add_state();
        table.add_transition(start, range('a', 'm'), accept);
        table.add_transition(start, range('n', 'z'), other_start);
        table.set_accept(accept, "identifier");

        Dfa::new(
            table
                .build(&[start, other_start])
                .expect("the table is below its capacity"),
        )
    }

    #[test]
    fn a_dfa_keeps_its_states_and_transitions() {
        let dfa = example();

        assert_eq!(dfa.state_count(), 3);
        assert_eq!(
            dfa.transitions(StateId::new(0)),
            &[
                Transition {
                    label: range('a', 'm'),
                    target: StateId::new(1),
                },
                Transition {
                    label: range('n', 'z'),
                    target: StateId::new(2),
                },
            ]
        );
        assert!(dfa.transitions(StateId::new(1)).is_empty());
    }

    #[test]
    fn a_dfa_keeps_its_accept_values() {
        let dfa = example();

        assert!(!dfa.accepts(StateId::new(0)));
        assert!(dfa.accepts(StateId::new(1)));
        assert_eq!(dfa.accept(StateId::new(1)), Some(&"identifier"));
        assert_eq!(dfa.accept(StateId::new(2)), None);
    }

    #[test]
    fn a_dfa_keeps_its_start_state_sequence() {
        let dfa = example();

        assert_eq!(dfa.start_states(), &[StateId::new(0), StateId::new(2)]);
        assert_eq!(dfa.start_state(0), StateId::new(0));
        assert_eq!(dfa.start_state(1), StateId::new(2));
    }

    #[test]
    fn a_step_follows_the_transition_that_matches_the_symbol() {
        let dfa = example();

        assert_eq!(dfa.step(StateId::new(0), 'a'), Some(StateId::new(1)));
        assert_eq!(dfa.step(StateId::new(0), 'm'), Some(StateId::new(1)));
        assert_eq!(dfa.step(StateId::new(0), 'n'), Some(StateId::new(2)));
        assert_eq!(dfa.step(StateId::new(0), 'z'), Some(StateId::new(2)));
    }

    #[test]
    fn a_step_without_a_matching_transition_returns_nothing() {
        let dfa = example();

        assert_eq!(dfa.step(StateId::new(0), 'A'), None);
        assert_eq!(dfa.step(StateId::new(1), 'a'), None);
    }

    #[derive(Clone)]
    struct CountedLabel {
        inner: Symbols,
        calls: Rc<Cell<usize>>,
    }

    impl Label for CountedLabel {
        type Symbol = char;

        fn matches(&self, symbol: Self::Symbol) -> bool {
            self.calls.set(self.calls.get() + 1);
            self.inner.matches(symbol)
        }
    }

    #[test]
    fn a_step_stops_after_the_matching_transition() {
        let calls = Rc::new(Cell::new(0));
        let mut table: StateTableBuilder<CountedLabel, ()> = StateTableBuilder::new();
        let start = table.add_state();
        let first = table.add_state();
        let second = table.add_state();
        table.add_transition(
            start,
            CountedLabel {
                inner: only('a'),
                calls: Rc::clone(&calls),
            },
            first,
        );
        table.add_transition(
            start,
            CountedLabel {
                inner: only('b'),
                calls: Rc::clone(&calls),
            },
            second,
        );
        let dfa = Dfa::new(
            table
                .build(&[start])
                .expect("the table is below its capacity"),
        );

        assert_eq!(dfa.step(start, 'a'), Some(first));
        assert_eq!(calls.get(), 1);
    }

    #[test]
    #[should_panic(expected = "state 9 is outside an automaton of 3 states")]
    fn reading_transitions_from_a_state_outside_the_dfa_panics() {
        example().transitions(StateId::new(9));
    }

    #[test]
    #[should_panic(expected = "state 9 is outside an automaton of 3 states")]
    fn reading_acceptance_from_a_state_outside_the_dfa_panics() {
        example().accepts(StateId::new(9));
    }

    #[test]
    #[should_panic(expected = "state 9 is outside an automaton of 3 states")]
    fn reading_an_accept_value_from_a_state_outside_the_dfa_panics() {
        example().accept(StateId::new(9));
    }

    #[test]
    #[should_panic(expected = "start 2 is outside an automaton with 2 start states")]
    fn reading_a_start_outside_the_dfa_panics() {
        example().start_state(2);
    }

    #[test]
    #[should_panic(expected = "state 9 is outside an automaton of 3 states")]
    fn stepping_from_a_state_outside_the_dfa_panics() {
        example().step(StateId::new(9), 'a');
    }
}
