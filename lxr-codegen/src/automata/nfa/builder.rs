use super::automaton::Nfa;
use crate::automata::adjacency::AdjacencyListBuilder;
use crate::automata::error::BuildError;
use crate::automata::id::StateId;
use crate::automata::table::StateTableBuilder;

/// A [`Nfa`] that is not complete.
///
/// Add a state with [`add_state`](Self::add_state), then add its transitions,
/// epsilon transitions, and accept value. A transition can point at a state
/// that comes later.
///
/// Build the automaton with [`build`](Self::build).
///
/// An added state past [`StateId::CAPACITY`] records a [`BuildError`]. Each
/// later mutation does nothing, so a caller needs no check after each state.
#[derive(Debug)]
pub struct Builder<L, A = ()> {
    table: StateTableBuilder<L, A>,
    epsilons: AdjacencyListBuilder<StateId>,
}

impl<L, A> Builder<L, A> {
    /// Creates a `Builder` that holds no state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a `Builder` that holds at most `capacity` states.
    ///
    /// The tests need a capacity below [`StateId::CAPACITY`].
    #[cfg(test)]
    pub(super) fn with_capacity(capacity: usize) -> Self {
        Self {
            table: StateTableBuilder::with_capacity(capacity),
            epsilons: AdjacencyListBuilder::new(),
        }
    }

    /// Returns the number of the states that the builder holds.
    ///
    /// A caller that builds one part after another reads this before the part and after it. The
    /// two numbers then give the states of that part, because
    /// [`add_state`](Self::add_state) adds each state at the end.
    pub fn state_count(&self) -> usize {
        self.table.state_count()
    }

    /// Adds a state to the automaton, then returns its identifier.
    ///
    /// The state has no transition, no epsilon transition, and no accept.
    ///
    /// An addition past the capacity returns a placeholder identifier.
    pub fn add_state(&mut self) -> StateId {
        self.table.add_state()
    }

    /// Adds a transition from `from` to `to` for each symbol that `label` matches.
    ///
    /// `to` can be a state that you add later. [`build`](Self::build) checks each target.
    ///
    /// # Panics
    ///
    /// This function panics if `from` is not in the builder.
    pub fn add_transition(&mut self, from: StateId, label: L, to: StateId) {
        self.table.add_transition(from, label, to);
    }

    /// Adds a transition from `from` to `to` that reads no symbol.
    ///
    /// `to` can be a state that you add later. [`build`](Self::build) checks each target.
    ///
    /// # Panics
    ///
    /// This function panics if `from` is not in the builder.
    pub fn add_epsilon_transition(&mut self, from: StateId, to: StateId) {
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

    /// Sets the accept value of `state`, then returns its prior value.
    ///
    /// The builder stores the value without interpreting it.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not in the builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr_codegen::automata::encoding::ByteRange;
    /// use lxr_codegen::automata::nfa::Builder;
    ///
    /// let mut builder = Builder::<ByteRange, &str>::new();
    /// let state = builder.add_state();
    ///
    /// assert_eq!(builder.set_accept(state, "identifier"), None);
    /// assert_eq!(builder.set_accept(state, "keyword"), Some("identifier"));
    /// ```
    pub fn set_accept(&mut self, state: StateId, accept: A) -> Option<A> {
        self.table.set_accept(state, accept)
    }

    /// Builds a [`Nfa`] that has one start state for each identifier
    /// in `starts`.
    ///
    /// # Errors
    ///
    /// This function returns a [`BuildError`] if the states, the transitions, or the epsilon
    /// transitions went past a capacity.
    ///
    /// # Panics
    ///
    /// This function panics if `starts` is empty, if a start state is not in the builder, or if
    /// the target of a transition is not in the builder.
    pub fn build(self, starts: &[StateId]) -> Result<Nfa<L, A>, BuildError> {
        let count = self.table.state_count();
        let table = self.table.build(starts)?;
        let epsilons = self.epsilons.build(count)?;

        Ok(Nfa::new(table, epsilons))
    }
}

impl<L, A> Default for Builder<L, A> {
    /// Creates a `Builder` that holds no state.
    fn default() -> Self {
        Self {
            table: StateTableBuilder::new(),
            epsilons: AdjacencyListBuilder::new(),
        }
    }
}

impl<L> Builder<L> {
    /// Makes `state` accept with the unit value.
    ///
    /// A second call at the same state changes nothing.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not in the builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr_codegen::automata::encoding::ByteRange;
    /// use lxr_codegen::automata::nfa::Builder;
    ///
    /// let mut builder = Builder::<ByteRange>::new();
    /// let state = builder.add_state();
    /// builder.mark_accept(state);
    /// let nfa = builder.build(&[state]).unwrap();
    ///
    /// assert!(nfa.accepts(state));
    /// ```
    pub fn mark_accept(&mut self, state: StateId) {
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
