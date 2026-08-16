use super::automaton::NondeterministicFiniteAutomaton;
use crate::automata::arena::ArenaBuilder;
use crate::automata::automaton::Transition;
use crate::automata::id::StateId;
use crate::automata::overflow::{Overflow, Part};

/// An [`NondeterministicFiniteAutomaton`] that is not complete.
///
/// Add a state with [`push`](Self::push), then add its transitions, its epsilon transitions, and
/// its accept. A transition can point at a state that comes later. Thus a loop needs no reserved
/// slot.
///
/// Build the automaton with [`build`](Self::build).
///
/// A push past [`StateId::CAPACITY`] records an [`Overflow`], and each later call does nothing.
/// Thus a long chain of pushes needs no check at each step.
#[derive(Debug)]
pub struct NfaBuilder<L> {
    transitions: ArenaBuilder<Transition<L>>,
    epsilons: ArenaBuilder<StateId>,
    accepts: Vec<bool>,
    capacity: usize,
    overflow: bool,
}

impl<L> NfaBuilder<L> {
    /// Creates an `NfaBuilder` that holds no state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an `NfaBuilder` whose state arena holds at most `capacity` states.
    ///
    /// The tests need a capacity below [`StateId::CAPACITY`].
    pub(super) fn with_capacity(capacity: usize) -> Self {
        Self {
            transitions: ArenaBuilder::new(),
            epsilons: ArenaBuilder::new(),
            accepts: Vec::new(),
            capacity,
            overflow: false,
        }
    }

    /// Adds a state to the end of the state arena, then returns its identifier.
    ///
    /// The state has no transition, no epsilon transition, and no accept.
    ///
    /// A push past the capacity returns an identifier that the caller must not use.
    pub fn push(&mut self) -> StateId {
        if self.accepts.len() >= self.capacity {
            self.overflow = true;
            return StateId::new(0);
        }
        let id = StateId::new(self.accepts.len());
        self.accepts.push(false);
        id
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

    /// Adds a transition from `from` to `to` that reads no symbol.
    ///
    /// `to` can be a state that you push later. [`build`](Self::build) checks each target.
    ///
    /// # Panics
    ///
    /// This function panics if `from` is not in the state arena.
    pub fn epsilon(&mut self, from: StateId, to: StateId) {
        if self.overflow {
            return;
        }
        assert!(
            from.index() < self.accepts.len(),
            "cannot add an epsilon transition at {}: no such state",
            from.index()
        );
        self.epsilons.push(from.index(), to);
    }

    /// Makes `state` accept.
    ///
    /// A second call at the same state changes nothing. The automaton holds no meaning of an
    /// accept, thus one mark is the whole of it.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not in the state arena.
    pub fn accept(&mut self, state: StateId) {
        if self.overflow {
            return;
        }
        let slot = self
            .accepts
            .get_mut(state.index())
            .unwrap_or_else(|| panic!("cannot accept at {}: no such state", state.index()));
        *slot = true;
    }

    /// Builds an [`NondeterministicFiniteAutomaton`] that has one start state for each identifier in `starts`.
    ///
    /// # Errors
    ///
    /// This function returns an [`Overflow`] if the states, the transitions, or the epsilon
    /// transitions went past a capacity.
    ///
    /// # Panics
    ///
    /// This function panics if `starts` is empty, if a start state is not in the state arena, or
    /// if the target of a transition is not in the state arena.
    pub fn build(self, starts: &[StateId]) -> Result<NondeterministicFiniteAutomaton<L>, Overflow> {
        if self.overflow {
            return Err(Overflow::new(Part::States, self.capacity));
        }

        let count = self.accepts.len();
        let transitions = self.transitions.build(count)?;
        let epsilons = self.epsilons.build(count)?;

        Ok(NondeterministicFiniteAutomaton::new(
            transitions,
            epsilons,
            self.accepts,
            starts.to_vec(),
        ))
    }
}

impl<L> Default for NfaBuilder<L> {
    /// Creates an `NfaBuilder` that holds no state.
    fn default() -> Self {
        Self::with_capacity(StateId::CAPACITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::automaton::Automaton;
    use crate::automata::testing::{Symbols, builder, only};

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
        builder.accept(second);
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
        let loop_state = builder.push();
        builder.transition(loop_state, only('a'), loop_state);
        builder.epsilon(loop_state, loop_state);
        let nfa = builder
            .build(&[loop_state])
            .expect("the builder is below its capacity");

        assert_eq!(nfa.transitions(loop_state)[0].target, loop_state);
        assert_eq!(nfa.epsilons(loop_state), &[loop_state]);
    }

    #[test]
    fn a_transition_can_point_at_a_state_that_comes_later() {
        let mut builder = builder();
        let start = builder.push();
        let accept = StateId::new(1);
        builder.transition(start, only('a'), accept);
        builder.push();
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        assert_eq!(nfa.transitions(start)[0].target, accept);
    }

    #[test]
    fn accepting_at_the_same_state_two_times_changes_nothing() {
        let mut builder = builder();
        let state = builder.push();
        builder.accept(state);
        builder.accept(state);
        let nfa = builder
            .build(&[state])
            .expect("the builder is below its capacity");

        assert!(nfa.accepts(state));
    }

    #[test]
    #[should_panic(expected = "cannot accept at 3: no such state")]
    fn accepting_at_a_state_that_was_never_pushed_panics() {
        builder().accept(StateId::new(3));
    }

    #[test]
    #[should_panic(expected = "cannot add a transition at 3: no such state")]
    fn adding_a_transition_at_a_state_that_was_never_pushed_panics() {
        let mut builder = builder();
        let target = builder.push();
        builder.transition(StateId::new(3), only('a'), target);
    }

    #[test]
    #[should_panic(expected = "cannot add an epsilon transition at 3: no such state")]
    fn adding_an_epsilon_transition_at_a_state_that_was_never_pushed_panics() {
        let mut builder = builder();
        let target = builder.push();
        builder.epsilon(StateId::new(3), target);
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
    #[should_panic(expected = "state 0 points at 9")]
    fn building_with_an_epsilon_target_outside_the_arena_panics() {
        let mut builder = builder();
        let start = builder.push();
        builder.epsilon(start, StateId::new(9));
        let _ = builder.build(&[start]);
    }

    #[test]
    #[should_panic(expected = "start 1 points at 9, outside")]
    fn building_with_a_start_outside_the_arena_panics() {
        let mut builder = builder();
        let start = builder.push();
        let _ = builder.build(&[start, StateId::new(9)]);
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
        let mut builder: NfaBuilder<Symbols> = NfaBuilder::with_capacity(2);
        let start = builder.push();
        let accept = builder.push();
        builder.transition(start, only('a'), accept);

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
        let mut builder: NfaBuilder<Symbols> = NfaBuilder::with_capacity(2);
        let start = builder.push();
        builder.push();
        builder.push();

        assert_eq!(builder.build(&[start]), Err(Overflow::new(Part::States, 2)));
    }

    #[test]
    fn a_builder_that_overflowed_takes_no_more_transitions() {
        let mut builder: NfaBuilder<Symbols> = NfaBuilder::with_capacity(1);
        let start = builder.push();
        let outside = builder.push();
        builder.transition(start, only('a'), outside);
        builder.epsilon(start, outside);
        builder.accept(outside);

        assert_eq!(builder.build(&[start]), Err(Overflow::new(Part::States, 1)));
    }
}
