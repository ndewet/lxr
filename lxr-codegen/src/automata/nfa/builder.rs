use super::automaton::Nfa;
use crate::automata::adjacency::AdjacencyListBuilder;
use crate::automata::error::BuildError;
use crate::automata::id::StateId;
use crate::automata::table::StateTableBuilder;

/// Collects states for an [`Nfa`].
///
/// A transition target can refer to a state that the caller adds later.
#[derive(Debug)]
pub(crate) struct Builder<L, A = ()> {
    table: StateTableBuilder<L, A>,
    epsilons: AdjacencyListBuilder<StateId>,
}

impl<L, A> Builder<L, A> {
    /// Creates an empty builder.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Creates a builder with the given state capacity.
    #[cfg(test)]
    pub(super) fn with_capacity(capacity: usize) -> Self {
        Self {
            table: StateTableBuilder::with_capacity(capacity),
            epsilons: AdjacencyListBuilder::new(),
        }
    }

    /// Returns the current state count.
    pub(crate) fn state_count(&self) -> usize {
        self.table.state_count()
    }

    /// Adds a state and returns its identifier.
    ///
    /// An addition past the capacity records an error and returns a placeholder.
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

    /// Adds an epsilon transition from `from` to `to`.
    ///
    /// # Panics
    ///
    /// This function panics if `from` is not in the builder.
    pub(crate) fn add_epsilon_transition(&mut self, from: StateId, to: StateId) {
        if self.table.has_error() {
            return;
        }
        assert!(
            self.table.contains(from),
            "cannot add an epsilon transition at {}: no such state",
            from.index()
        );
        self.epsilons.add(from, to);
    }

    /// Sets the accept value and returns the prior value.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not in the builder.
    pub(crate) fn set_accept(&mut self, state: StateId, accept: A) -> Option<A> {
        self.table.set_accept(state, accept)
    }

    /// Builds an NFA with `starts` as its start states.
    ///
    /// # Errors
    ///
    /// This function returns a [`BuildError`] if a capacity was exceeded.
    ///
    /// # Panics
    ///
    /// This function panics if `starts` is empty or an identifier is invalid.
    pub(crate) fn build(self, starts: &[StateId]) -> Result<Nfa<L, A>, BuildError> {
        let count = self.table.state_count();
        let table = self.table.build(starts)?;
        let epsilons = self.epsilons.build(count)?;

        Ok(Nfa::new(table, epsilons))
    }
}

impl<L, A> Default for Builder<L, A> {
    fn default() -> Self {
        Self {
            table: StateTableBuilder::new(),
            epsilons: AdjacencyListBuilder::new(),
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
        let _ = self.set_accept(state, ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::testing::{Symbols, builder, only};

    #[test]
    fn adding_states_returns_sequential_ids() {
        let mut builder = builder();
        let first = builder.add_state();
        let second = builder.add_state();

        assert_eq!(first.index(), 0);
        assert_eq!(second.index(), 1);
    }

    #[test]
    fn building_keeps_the_states_in_insertion_order() {
        let mut builder = builder();
        let first = builder.add_state();
        let second = builder.add_state();
        builder.mark_accept(second);
        let nfa = builder
            .build(&[second])
            .expect("the builder is below its capacity");

        assert!(!nfa.accepts(first));
        assert!(nfa.accepts(second));
        assert_eq!(nfa.state_count(), 2);
    }

    #[test]
    fn a_state_can_point_back_at_itself() {
        let mut builder = builder();
        let loop_state = builder.add_state();
        builder.add_transition(loop_state, only('a'), loop_state);
        builder.add_epsilon_transition(loop_state, loop_state);
        let nfa = builder
            .build(&[loop_state])
            .expect("the builder is below its capacity");

        assert_eq!(nfa.transitions(loop_state)[0].target, loop_state);
        assert_eq!(nfa.epsilon_targets(loop_state), &[loop_state]);
    }

    #[test]
    fn a_transition_can_point_at_a_state_that_comes_later() {
        let mut builder = builder();
        let start = builder.add_state();
        let accept = StateId::new(1);
        builder.add_transition(start, only('a'), accept);
        builder.add_state();
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        assert_eq!(nfa.transitions(start)[0].target, accept);
    }

    #[test]
    fn accepting_at_the_same_state_two_times_changes_nothing() {
        let mut builder = builder();
        let state = builder.add_state();
        builder.mark_accept(state);
        builder.mark_accept(state);
        let nfa = builder
            .build(&[state])
            .expect("the builder is below its capacity");

        assert!(nfa.accepts(state));
    }

    #[test]
    fn setting_an_accept_returns_and_replaces_its_prior_value() {
        let mut builder = Builder::<Symbols, &str>::new();
        let state = builder.add_state();

        assert_eq!(builder.set_accept(state, "identifier"), None);
        assert_eq!(builder.set_accept(state, "keyword"), Some("identifier"));

        let nfa = builder
            .build(&[state])
            .expect("the builder is below its capacity");
        assert_eq!(nfa.accept(state), Some(&"keyword"));
    }

    #[test]
    #[should_panic(expected = "cannot accept at 3: no such state")]
    fn accepting_at_a_state_that_was_never_added_panics() {
        builder().mark_accept(StateId::new(3));
    }

    #[test]
    #[should_panic(expected = "cannot add a transition at 3: no such state")]
    fn adding_a_transition_at_a_state_that_was_never_added_panics() {
        let mut builder = builder();
        let target = builder.add_state();
        builder.add_transition(StateId::new(3), only('a'), target);
    }

    #[test]
    #[should_panic(expected = "cannot add an epsilon transition at 3: no such state")]
    fn adding_an_epsilon_transition_at_a_state_that_was_never_added_panics() {
        let mut builder = builder();
        let target = builder.add_state();
        builder.add_epsilon_transition(StateId::new(3), target);
    }

    #[test]
    #[should_panic(expected = "state 0 points at 9")]
    fn building_with_a_transition_target_outside_the_arena_panics() {
        let mut builder = builder();
        let start = builder.add_state();
        builder.add_transition(start, only('a'), StateId::new(9));
        let _ = builder.build(&[start]);
    }

    #[test]
    #[should_panic(expected = "state 0 points at 9")]
    fn building_with_an_epsilon_target_outside_the_arena_panics() {
        let mut builder = builder();
        let start = builder.add_state();
        builder.add_epsilon_transition(start, StateId::new(9));
        let _ = builder.build(&[start]);
    }

    #[test]
    #[should_panic(expected = "start 1 points at 9, outside")]
    fn building_with_a_start_outside_the_arena_panics() {
        let mut builder = builder();
        let start = builder.add_state();
        let _ = builder.build(&[start, StateId::new(9)]);
    }

    #[test]
    #[should_panic(expected = "at least one start state")]
    fn building_without_a_start_panics() {
        let mut builder = builder();
        builder.add_state();
        let _ = builder.build(&[]);
    }

    #[test]
    fn a_builder_at_its_capacity_builds() {
        let mut builder: Builder<Symbols> = Builder::with_capacity(2);
        let start = builder.add_state();
        let accept = builder.add_state();
        builder.add_transition(start, only('a'), accept);

        assert_eq!(
            builder
                .build(&[start])
                .expect("the builder is below its capacity")
                .state_count(),
            2
        );
    }

    #[test]
    fn adding_a_state_past_the_capacity_reports_an_error() {
        let mut builder: Builder<Symbols> = Builder::with_capacity(2);
        let start = builder.add_state();
        builder.add_state();
        builder.add_state();

        assert_eq!(
            builder.build(&[start]),
            Err(BuildError::TooManyStates { capacity: 2 })
        );
    }

    #[test]
    fn a_builder_past_its_capacity_takes_no_more_transitions() {
        let mut builder: Builder<Symbols> = Builder::with_capacity(1);
        let start = builder.add_state();
        let outside = builder.add_state();
        builder.add_transition(start, only('a'), outside);
        builder.add_epsilon_transition(start, outside);
        builder.mark_accept(outside);

        assert_eq!(
            builder.build(&[start]),
            Err(BuildError::TooManyStates { capacity: 1 })
        );
    }
}
