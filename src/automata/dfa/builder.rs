use super::automaton::Dfa;
use crate::automata::arena_builder::ArenaBuilder;
use crate::automata::id::StateId;
use crate::automata::overflow::{Overflow, Part};
use crate::automata::transition::Transition;

/// A [`Dfa`] that is not complete.
///
/// Add a state with [`push`](Self::push), then add its transitions and its accept. A transition
/// can point at a state that comes later. Thus a loop needs no reserved slot.
///
/// The builder does not check that the labels of one state are disjoint. Only lxr calls it, and
/// determinization gives disjoint labels. An execution reads the first label that matches.
///
/// Build the automaton with [`build`](Self::build).
///
/// A push past the capacity records an overflow, and each later call does nothing. Ask
/// [`overflowed`](Self::overflowed) if a long chain of pushes must stop early.
#[derive(Debug)]
pub struct DfaBuilder<L, A> {
    transitions: ArenaBuilder<Transition<L>>,
    accepts: Vec<Option<A>>,
    capacity: usize,
    overflow: bool,
}

impl<L, A> DfaBuilder<L, A> {
    /// Creates a `DfaBuilder` that holds no state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a `DfaBuilder` whose state arena holds at most `capacity` states.
    ///
    /// Determinization gives a limit here. The tests need a capacity below [`StateId::CAPACITY`].
    pub(in crate::automata) fn with_capacity(capacity: usize) -> Self {
        Self {
            transitions: ArenaBuilder::new(),
            accepts: Vec::new(),
            capacity,
            overflow: false,
        }
    }

    /// Adds a state to the end of the state arena, then returns its identifier.
    ///
    /// The state has no transition and no accept.
    ///
    /// A push past the capacity returns an identifier that the caller must not use.
    pub fn push(&mut self) -> StateId {
        if self.accepts.len() >= self.capacity {
            self.overflow = true;
            return StateId::new(0);
        }
        let id = StateId::new(self.accepts.len());
        self.accepts.push(None);
        id
    }

    /// Returns `true` if a push went past the capacity of the state arena.
    ///
    /// A caller that adds the states in a loop reads this to stop the loop.
    /// [`build`](Self::build) then reports the [`Overflow`].
    pub fn overflowed(&self) -> bool {
        self.overflow
    }

    /// Adds a transition from `from` to `to` for each symbol that `label` matches.
    ///
    /// `to` can be a state that you push later. [`build`](Self::build) checks each target.
    ///
    /// # Panics
    ///
    /// This function panics if `from` is not in the state arena.
    pub fn transition(&mut self, from: StateId, label: L, to: StateId) {
        if self.overflow {
            return;
        }
        assert!(
            from.index() < self.accepts.len(),
            "cannot add a transition at {}: no such state",
            from.index()
        );
        self.transitions
            .push(from.index(), Transition { label, target: to });
    }

    /// Makes `state` accept, with `accept` as its accept.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not in the state arena. It also panics if `state`
    /// already has an accept.
    pub fn accept(&mut self, state: StateId, accept: A) {
        if self.overflow {
            return;
        }
        let slot = self
            .accepts
            .get_mut(state.index())
            .unwrap_or_else(|| panic!("cannot accept at {}: no such state", state.index()));
        assert!(
            slot.is_none(),
            "state {} already has an accept",
            state.index()
        );
        *slot = Some(accept);
    }

    /// Builds a [`Dfa`] that has one start state for each identifier in `starts`.
    ///
    /// # Errors
    ///
    /// This function returns an [`Overflow`] if the states or the transitions went past a
    /// capacity.
    ///
    /// # Panics
    ///
    /// This function panics for each of these conditions:
    ///
    /// - `starts` is empty.
    /// - A start state is not in the state arena.
    /// - The target of a transition is not in the state arena.
    pub fn build(self, starts: &[StateId]) -> Result<Dfa<L, A>, Overflow> {
        if self.overflow {
            return Err(Overflow::new(Part::States, self.capacity));
        }

        let count = self.accepts.len();
        assert!(!starts.is_empty(), "a DFA needs at least one start state");
        for (index, start) in starts.iter().enumerate() {
            assert!(
                start.index() < count,
                "start {index} points at {}, outside an arena of {count} states",
                start.index()
            );
        }

        let transitions = self.transitions.build(count)?;

        for index in 0..count {
            for transition in transitions.get(index).into_iter().flatten() {
                assert!(
                    transition.target.index() < count,
                    "state {index} points at {}, outside an arena of {count} states",
                    transition.target.index()
                );
            }
        }

        Ok(Dfa::new(transitions, self.accepts, starts.to_vec()))
    }
}

impl<L, A> Default for DfaBuilder<L, A> {
    /// Creates a `DfaBuilder` that holds no state.
    fn default() -> Self {
        Self::with_capacity(StateId::CAPACITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::overflow::Part;
    use crate::automata::testing::{Symbols, only};

    fn builder() -> DfaBuilder<Symbols, u32> {
        DfaBuilder::new()
    }

    #[test]
    fn push_hands_back_sequential_ids() {
        let mut builder = builder();
        let first = builder.push();
        let second = builder.push();

        assert_eq!(first.index(), 0);
        assert_eq!(second.index(), 1);
    }

    #[test]
    fn building_keeps_the_states_in_push_order() {
        let mut builder = builder();
        let first = builder.push();
        let second = builder.push();
        builder.accept(first, 0);
        builder.accept(second, 9);
        let dfa = builder
            .build(&[second])
            .expect("the builder is below its capacity");

        assert_eq!(dfa.accept(first), Some(&0));
        assert_eq!(dfa.accept(second), Some(&9));
        assert_eq!(dfa.state_count(), 2);
    }

    #[test]
    fn a_state_keeps_the_sequence_of_its_transitions() {
        let mut builder = builder();
        let start = builder.push();
        let first = builder.push();
        let second = builder.push();
        builder.transition(start, only('b'), second);
        builder.transition(start, only('a'), first);
        let dfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        assert_eq!(dfa.transitions(start)[0].target, second);
        assert_eq!(dfa.transitions(start)[1].target, first);
    }

    #[test]
    fn a_state_can_point_back_at_itself() {
        let mut builder = builder();
        let loop_state = builder.push();
        builder.transition(loop_state, only('a'), loop_state);
        let dfa = builder
            .build(&[loop_state])
            .expect("the builder is below its capacity");

        assert_eq!(dfa.transitions(loop_state)[0].target, loop_state);
    }

    #[test]
    fn a_transition_can_point_at_a_state_that_comes_later() {
        let mut builder = builder();
        let start = builder.push();
        let accept = StateId::new(1);
        builder.transition(start, only('a'), accept);
        builder.push();
        let dfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        assert_eq!(dfa.transitions(start)[0].target, accept);
    }

    #[test]
    fn a_state_that_gets_no_accept_does_not_accept() {
        let mut builder = builder();
        let state = builder.push();
        let dfa = builder
            .build(&[state])
            .expect("the builder is below its capacity");

        assert_eq!(dfa.accept(state), None);
        assert_eq!(dfa.transitions(state), &[]);
    }

    #[test]
    #[should_panic(expected = "state 0 already has an accept")]
    fn accepting_at_the_same_state_two_times_panics() {
        let mut builder = builder();
        let state = builder.push();
        builder.accept(state, 0);
        builder.accept(state, 1);
    }

    #[test]
    #[should_panic(expected = "cannot accept at 3: no such state")]
    fn accepting_at_a_state_that_was_never_pushed_panics() {
        builder().accept(StateId::new(3), 0);
    }

    #[test]
    #[should_panic(expected = "cannot add a transition at 3: no such state")]
    fn adding_a_transition_at_a_state_that_was_never_pushed_panics() {
        let mut builder = builder();
        let target = builder.push();
        builder.transition(StateId::new(3), only('a'), target);
    }

    #[test]
    #[should_panic(expected = "state 0 points at 9")]
    fn building_with_a_transition_target_outside_the_arena_panics() {
        let mut builder = builder();
        let start = builder.push();
        builder.transition(start, only('a'), StateId::new(9));
        let _ = builder.build(&[start]);
    }

    #[test]
    #[should_panic(expected = "start 0 points at 9, outside")]
    fn building_with_a_start_outside_the_arena_panics() {
        let mut builder = builder();
        builder.push();
        let _ = builder.build(&[StateId::new(9)]);
    }

    #[test]
    #[should_panic(expected = "at least one start state")]
    fn building_without_a_start_panics() {
        let mut builder = builder();
        builder.push();
        let _ = builder.build(&[]);
    }

    #[test]
    fn a_builder_at_its_capacity_builds() {
        let mut builder: DfaBuilder<Symbols, u32> = DfaBuilder::with_capacity(2);
        let start = builder.push();
        let accept = builder.push();
        builder.transition(start, only('a'), accept);

        assert!(!builder.overflowed());
        assert_eq!(
            builder
                .build(&[start])
                .expect("the builder is below its capacity")
                .state_count(),
            2
        );
    }

    #[test]
    fn a_push_past_the_capacity_reports_an_overflow() {
        let mut builder: DfaBuilder<Symbols, u32> = DfaBuilder::with_capacity(2);
        let start = builder.push();
        builder.push();
        builder.push();

        assert!(builder.overflowed());
        assert_eq!(builder.build(&[start]), Err(Overflow::new(Part::States, 2)));
    }

    #[test]
    fn a_builder_that_overflowed_takes_no_more_transitions() {
        let mut builder: DfaBuilder<Symbols, u32> = DfaBuilder::with_capacity(1);
        let start = builder.push();
        let outside = builder.push();
        builder.transition(start, only('a'), outside);
        builder.accept(outside, 0);

        assert_eq!(builder.build(&[start]), Err(Overflow::new(Part::States, 1)));
    }
}
