use super::automaton::Nfa;
use crate::automata::arena_builder::ArenaBuilder;
use crate::automata::id::StateId;
use crate::automata::transition::Transition;

/// An [`Nfa`] that is not complete.
///
/// Add a state with [`push`](Self::push), then add its transitions, its epsilon transitions, and
/// its accept. A transition can point at a state that comes later. Thus a loop needs no reserved
/// slot.
///
/// Build the automaton with [`build`](Self::build).
#[derive(Debug)]
pub struct NfaBuilder<L, A> {
    transitions: ArenaBuilder<Transition<L>>,
    epsilons: ArenaBuilder<StateId>,
    accepts: Vec<Option<A>>,
}

impl<L, A> NfaBuilder<L, A> {
    /// Creates an `NfaBuilder` that holds no state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a state to the end of the state arena, then returns its identifier.
    ///
    /// The state has no transition, no epsilon transition, and no accept.
    ///
    /// # Panics
    ///
    /// This function panics if the state arena already holds `u32::MAX + 1` states.
    pub fn push(&mut self) -> StateId {
        let id = StateId::new(self.accepts.len());
        self.accepts.push(None);
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
        assert!(
            from.index() < self.accepts.len(),
            "cannot add an epsilon transition at {}: no such state",
            from.index()
        );
        self.epsilons.push(from.index(), to);
    }

    /// Makes `state` accept, with `accept` as its accept.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not in the state arena. It also panics if `state`
    /// already has an accept.
    pub fn accept(&mut self, state: StateId, accept: A) {
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

    /// Builds an [`Nfa`] that has one start state for each identifier in `starts`.
    ///
    /// # Panics
    ///
    /// This function panics for each of these conditions:
    ///
    /// - `starts` is empty.
    /// - A start state is not in the state arena.
    /// - The target of a transition is not in the state arena.
    /// - The target of an epsilon transition is not in the state arena.
    pub fn build(self, starts: &[StateId]) -> Nfa<L, A> {
        let count = self.accepts.len();
        assert!(!starts.is_empty(), "an NFA needs at least one start state");
        for (index, start) in starts.iter().enumerate() {
            assert!(
                start.index() < count,
                "start {index} points at {}, outside an arena of {count} states",
                start.index()
            );
        }

        let transitions = self.transitions.build(count);
        let epsilons = self.epsilons.build(count);

        for index in 0..count {
            let targets = transitions
                .get(index)
                .into_iter()
                .flatten()
                .map(|transition| transition.target)
                .chain(epsilons.get(index).into_iter().flatten().copied());
            for target in targets {
                assert!(
                    target.index() < count,
                    "state {index} points at {}, outside an arena of {count} states",
                    target.index()
                );
            }
        }

        Nfa::new(transitions, epsilons, self.accepts, starts.to_vec())
    }
}

impl<L, A> Default for NfaBuilder<L, A> {
    /// Creates an `NfaBuilder` that holds no state.
    fn default() -> Self {
        Self {
            transitions: ArenaBuilder::new(),
            epsilons: ArenaBuilder::new(),
            accepts: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::reference::{Symbols, only};

    fn builder() -> NfaBuilder<Symbols, u32> {
        NfaBuilder::new()
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
        let nfa = builder.build(&[second]);

        assert_eq!(nfa.accept(first), Some(&0));
        assert_eq!(nfa.accept(second), Some(&9));
        assert_eq!(nfa.state_count(), 2);
    }

    #[test]
    fn a_state_can_point_back_at_itself() {
        let mut builder = builder();
        let loop_state = builder.push();
        builder.transition(loop_state, only('a'), loop_state);
        builder.epsilon(loop_state, loop_state);
        let nfa = builder.build(&[loop_state]);

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
        let nfa = builder.build(&[start]);

        assert_eq!(nfa.transitions(start)[0].target, accept);
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
        builder.build(&[start]);
    }

    #[test]
    #[should_panic(expected = "state 0 points at 9")]
    fn building_with_an_epsilon_target_outside_the_arena_panics() {
        let mut builder = builder();
        let start = builder.push();
        builder.epsilon(start, StateId::new(9));
        builder.build(&[start]);
    }

    #[test]
    #[should_panic(expected = "start 0 points at 9, outside")]
    fn building_with_a_start_outside_the_arena_panics() {
        let mut builder = builder();
        builder.push();
        builder.build(&[StateId::new(9)]);
    }

    #[test]
    #[should_panic(expected = "start 1 points at 9, outside")]
    fn building_with_a_later_start_outside_the_arena_panics() {
        let mut builder = builder();
        let start = builder.push();
        builder.build(&[start, StateId::new(9)]);
    }

    #[test]
    #[should_panic(expected = "at least one start state")]
    fn building_without_a_start_panics() {
        let mut builder = builder();
        builder.push();
        builder.build(&[]);
    }
}
