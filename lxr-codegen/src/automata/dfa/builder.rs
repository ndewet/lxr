use super::Dfa;
use crate::automata::label::Partitionable;
use crate::automata::table::StateTableBuilder;
use crate::automata::{BuildError, StateId};

/// Collects states for a [`Dfa`].
///
/// Outgoing labels from one state must be disjoint.
#[derive(Debug)]
pub(crate) struct Builder<L, A = ()> {
    table: StateTableBuilder<L, A>,
}

impl<L, A> Builder<L, A> {
    /// Creates an empty builder.
    pub(crate) fn new() -> Self {
        Self {
            table: StateTableBuilder::new(),
        }
    }

    /// Creates a builder with the given state capacity.
    #[cfg(test)]
    pub(super) fn with_capacity(capacity: usize) -> Self {
        Self {
            table: StateTableBuilder::with_capacity(capacity),
        }
    }

    /// Returns the current state count.
    pub(crate) fn state_count(&self) -> usize {
        self.table.state_count()
    }

    /// Adds a state and returns its identifier.
    pub(crate) fn add_state(&mut self) -> StateId {
        self.table.add_state()
    }

    /// Adds a transition from `from` to `to`.
    ///
    /// # Panics
    ///
    /// This function panics if `from` is not in the builder.
    pub(crate) fn add_transition(&mut self, from: StateId, label: L, to: StateId) {
        self.table.add_transition(from, label, to);
    }

    /// Sets the accept value and returns the prior value.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not in the builder.
    pub(crate) fn set_accept(&mut self, state: StateId, accept: A) -> Option<A> {
        self.table.set_accept(state, accept)
    }

    /// Builds a DFA with `starts` as its start states.
    ///
    /// # Errors
    ///
    /// This function returns a [`BuildError`] if a capacity was exceeded.
    ///
    /// # Panics
    ///
    /// This function panics if `starts` is empty or an identifier is invalid.
    /// It also panics if two outgoing labels overlap.
    pub(crate) fn build(self, starts: &[StateId]) -> Result<Dfa<L, A>, BuildError>
    where
        L: Partitionable,
    {
        let table = self.table.build(starts)?;
        for index in 0..table.state_count() {
            let state = StateId::new(index);
            let labels: Vec<_> = table
                .transitions(state)
                .iter()
                .map(|transition| transition.label.clone())
                .collect();
            let overlaps = L::partition(&labels)
                .iter()
                .any(|class| class.get_matching_labels().len() > 1);
            assert!(!overlaps, "state {index} has overlapping transition labels");
        }
        Ok(Dfa::new(table))
    }
}

impl<L, A> Default for Builder<L, A> {
    fn default() -> Self {
        Self {
            table: StateTableBuilder::default(),
        }
    }
}

impl<L> Builder<L> {
    /// Marks `state` as an accept state.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not in the builder.
    pub(crate) fn mark_accept(&mut self, state: StateId) {
        self.table.set_accept(state, ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::Transition;
    use crate::automata::encoding::ByteRange;
    use crate::automata::testing::{Symbols, only};

    fn builder() -> Builder<Symbols> {
        Builder {
            table: StateTableBuilder::new(),
        }
    }

    fn with_states(count: usize) -> (Builder<Symbols>, Vec<StateId>) {
        let mut builder = builder();
        let states = (0..count).map(|_| builder.table.add_state()).collect();
        (builder, states)
    }

    #[test]
    fn a_new_builder_holds_no_state() {
        let builder = Builder::<Symbols>::new();

        assert_eq!(builder.table.state_count(), 0);
    }

    #[test]
    fn a_default_builder_holds_no_state() {
        let builder = Builder::<Symbols>::default();

        assert_eq!(builder.table.state_count(), 0);
    }

    #[test]
    fn the_state_count_reports_the_shared_table_count() {
        let (builder, _) = with_states(2);

        assert_eq!(builder.state_count(), 2);
    }

    #[test]
    fn adding_states_returns_sequential_identifiers() {
        let mut builder = builder();

        assert_eq!(builder.add_state(), StateId::new(0));
        assert_eq!(builder.add_state(), StateId::new(1));
    }

    #[test]
    fn adding_a_transition_keeps_its_label_and_target() {
        let (mut builder, states) = with_states(2);
        builder.add_transition(states[0], only('a'), states[1]);

        let table = builder
            .table
            .build(&[states[0]])
            .expect("the table is below its capacity");
        assert_eq!(
            table.transitions(states[0]),
            &[Transition {
                label: only('a'),
                target: states[1],
            }]
        );
    }

    #[test]
    fn a_state_can_point_back_at_itself() {
        let (mut builder, states) = with_states(1);
        builder.add_transition(states[0], only('a'), states[0]);

        let table = builder
            .table
            .build(&states)
            .expect("the table is below its capacity");
        assert_eq!(table.transitions(states[0])[0].target, states[0]);
    }

    #[test]
    fn a_transition_can_point_at_a_state_that_comes_later() {
        let (mut builder, states) = with_states(1);
        let later = StateId::new(1);
        builder.add_transition(states[0], only('a'), later);
        builder.table.add_state();

        let table = builder
            .table
            .build(&states)
            .expect("the table is below its capacity");
        assert_eq!(table.transitions(states[0])[0].target, later);
    }

    #[test]
    fn setting_an_accept_returns_and_replaces_its_prior_value() {
        let mut table = StateTableBuilder::new();
        let state = table.add_state();
        let mut builder = Builder::<Symbols, &str> { table };

        assert_eq!(builder.set_accept(state, "identifier"), None);
        assert_eq!(builder.set_accept(state, "keyword"), Some("identifier"));

        let table = builder
            .table
            .build(&[state])
            .expect("the table is below its capacity");
        assert_eq!(table.accept(state), Some(&"keyword"));
    }

    #[test]
    fn marking_accept_at_the_same_state_two_times_changes_nothing() {
        let (mut builder, states) = with_states(1);
        builder.mark_accept(states[0]);
        builder.mark_accept(states[0]);

        let table = builder
            .table
            .build(&states)
            .expect("the table is below its capacity");
        assert!(table.accepts(states[0]));
    }

    #[test]
    fn building_keeps_states_transitions_accepts_and_starts() {
        let mut table = StateTableBuilder::<ByteRange, &str>::new();
        let first = table.add_state();
        let second = table.add_state();
        table.add_transition(first, ByteRange::new(b'a', b'a'), second);
        table.set_accept(second, "identifier");
        let builder = Builder { table };

        let dfa = builder
            .build(&[second, first])
            .expect("the builder is below its capacity");

        assert_eq!(dfa.state_count(), 2);
        assert_eq!(dfa.transitions(first)[0].target, second);
        assert_eq!(dfa.accept(second), Some(&"identifier"));
        assert_eq!(dfa.start_states(), &[second, first]);
    }

    #[test]
    #[should_panic(expected = "cannot add a transition at 3: no such state")]
    fn adding_a_transition_at_a_state_that_was_never_added_panics() {
        let (mut builder, states) = with_states(1);

        builder.add_transition(StateId::new(3), only('a'), states[0]);
    }

    #[test]
    #[should_panic(expected = "cannot accept at 3: no such state")]
    fn accepting_at_a_state_that_was_never_added_panics() {
        let mut builder = builder();

        builder.mark_accept(StateId::new(3));
    }

    #[test]
    #[should_panic(expected = "state 0 points at 9")]
    fn building_with_a_transition_target_outside_the_dfa_panics() {
        let mut table: StateTableBuilder<ByteRange, ()> = StateTableBuilder::new();
        let start = table.add_state();
        table.add_transition(start, ByteRange::new(b'a', b'a'), StateId::new(9));

        let _ = Builder { table }.build(&[start]);
    }

    #[test]
    #[should_panic(expected = "start 1 points at 9, outside")]
    fn building_with_a_start_outside_the_dfa_panics() {
        let mut table: StateTableBuilder<ByteRange, ()> = StateTableBuilder::new();
        let start = table.add_state();

        let _ = Builder { table }.build(&[start, StateId::new(9)]);
    }

    #[test]
    #[should_panic(expected = "at least one start state")]
    fn building_without_a_start_panics() {
        let mut table: StateTableBuilder<ByteRange, ()> = StateTableBuilder::new();
        table.add_state();

        let _ = Builder { table }.build(&[]);
    }

    #[test]
    #[should_panic(expected = "state 0 has overlapping transition labels")]
    fn building_with_overlapping_transition_labels_panics() {
        let mut builder = Builder::<ByteRange>::new();
        let start = builder.add_state();
        let first = builder.add_state();
        let second = builder.add_state();
        builder.add_transition(start, ByteRange::new(b'a', b'z'), first);
        builder.add_transition(start, ByteRange::new(b'm', b'z'), second);

        let _ = builder.build(&[start]);
    }

    #[test]
    fn a_builder_can_have_a_test_capacity() {
        let mut builder = Builder::<Symbols>::with_capacity(2);
        builder.table.add_state();
        builder.table.add_state();
        builder.table.add_state();

        assert!(matches!(
            builder.table.build(&[StateId::new(0)]),
            Err(BuildError::TooManyStates { capacity: 2 })
        ));
    }

    #[test]
    fn a_builder_at_its_capacity_builds() {
        let mut table = StateTableBuilder::<ByteRange, ()>::with_capacity(2);
        let start = table.add_state();
        table.add_state();

        let dfa = Builder { table }
            .build(&[start])
            .expect("the builder is at its capacity");
        assert_eq!(dfa.state_count(), 2);
    }

    #[test]
    fn adding_a_state_past_the_capacity_records_an_error() {
        let mut builder = Builder {
            table: StateTableBuilder::<Symbols, ()>::with_capacity(2),
        };
        let start = builder.add_state();
        builder.add_state();
        builder.add_state();

        assert!(matches!(
            builder.table.build(&[start]),
            Err(BuildError::TooManyStates { capacity: 2 })
        ));
    }

    #[test]
    fn building_propagates_a_recorded_capacity_error() {
        let mut table = StateTableBuilder::<ByteRange, ()>::with_capacity(1);
        let start = table.add_state();
        table.add_state();

        assert!(matches!(
            Builder { table }.build(&[start]),
            Err(BuildError::TooManyStates { capacity: 1 })
        ));
    }

    #[test]
    fn a_builder_past_its_capacity_takes_no_more_mutations() {
        let mut builder = Builder {
            table: StateTableBuilder::<ByteRange, ()>::with_capacity(1),
        };
        let start = builder.add_state();
        let outside = builder.add_state();
        builder.add_transition(start, ByteRange::new(b'a', b'a'), outside);
        builder.mark_accept(outside);

        assert!(matches!(
            builder.build(&[start]),
            Err(BuildError::TooManyStates { capacity: 1 })
        ));
    }
}
