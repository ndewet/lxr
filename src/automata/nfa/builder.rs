use super::automaton::Nfa;
use super::state::{State, StateId};

/// An arena of [`State`]s under construction.
#[derive(Debug, Default)]
pub struct NfaBuilder {
    states: Vec<Option<State>>,
}

impl NfaBuilder {
    /// Creates an empty `NfaBuilder`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a state to the arena and returns its identifier.
    ///
    /// # Panics
    ///
    /// Panics if the arena already holds `u32::MAX + 1` states.
    pub fn push(&mut self, state: State) -> StateId {
        let id = StateId::new(self.states.len());
        self.states.push(Some(state));
        id
    }

    /// Appends an empty slot to the arena and returns its identifier.
    ///
    /// States pushed before the slot is filled may point at it. Every reserved slot must be filled
    /// with [`fill`](Self::fill) before the arena is built.
    ///
    /// # Panics
    ///
    /// Panics if the arena already holds `u32::MAX + 1` states.
    pub fn reserve(&mut self) -> StateId {
        let id = StateId::new(self.states.len());
        self.states.push(None);
        id
    }

    /// Writes a state into the slot reserved for `id`.
    ///
    /// # Panics
    ///
    /// Panics if `id` is outside the arena, or if its slot is already filled.
    pub fn fill(&mut self, id: StateId, state: State) {
        let slot = self
            .states
            .get_mut(id.index())
            .unwrap_or_else(|| panic!("cannot fill {}: no such state", id.index()));
        assert!(
            slot.is_none(),
            "cannot fill {}: already filled with {:?}",
            id.index(),
            slot
        );
        *slot = Some(state);
    }

    /// Builds an [`Nfa`] that is entered at `starts`.
    ///
    /// # Panics
    ///
    /// Panics if `starts` is empty, if a reserved slot was never filled, if a start state or a
    /// successor points outside the arena, or if a [`State::Range`] has a `low` bound greater than
    /// its `high` bound.
    pub fn build(self, starts: &[StateId]) -> Nfa {
        let count = self.states.len();
        assert!(!starts.is_empty(), "an NFA needs at least one start state");
        for (index, start) in starts.iter().enumerate() {
            assert!(
                start.index() < count,
                "start {index} points at {}, outside an arena of {count} states",
                start.index()
            );
        }

        let states: Vec<State> = self
            .states
            .into_iter()
            .enumerate()
            .map(|(index, state)| {
                state.unwrap_or_else(|| panic!("state {index} was reserved but never filled"))
            })
            .collect();

        for (index, state) in states.iter().enumerate() {
            if let State::Range { low, high, .. } = *state {
                assert!(
                    low <= high,
                    "state {index} has an empty byte range {low:#04x}..={high:#04x}"
                );
            }

            for successor in state.successors() {
                assert!(
                    successor.index() < count,
                    "state {index} points at {}, outside an arena of {count} states",
                    successor.index()
                );
            }
        }

        Nfa::new(states, starts.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_hands_back_sequential_ids() {
        let mut builder = NfaBuilder::new();
        let first = builder.push(State::Match { token: 0 });
        let second = builder.push(State::Match { token: 1 });

        assert_eq!(first.index(), 0);
        assert_eq!(second.index(), 1);
    }

    #[test]
    fn building_keeps_the_states_in_push_order() {
        let mut builder = NfaBuilder::new();
        let first = builder.push(State::Match { token: 0 });
        let accept = builder.push(State::Match { token: 9 });
        let nfa = builder.build(&[accept]);

        assert_eq!(nfa.state(first), State::Match { token: 0 });
        assert_eq!(nfa.state(accept), State::Match { token: 9 });
        assert_eq!(nfa.state_count(), 2);
    }

    #[test]
    fn a_reserved_state_can_point_back_at_itself() {
        let mut builder = NfaBuilder::new();
        let split = builder.reserve();
        let accept = builder.push(State::Match { token: 0 });
        builder.fill(
            split,
            State::Split {
                first: split,
                second: accept,
            },
        );
        let nfa = builder.build(&[split]);

        assert_eq!(
            nfa.state(split),
            State::Split {
                first: split,
                second: accept,
            }
        );
    }

    #[test]
    #[should_panic(expected = "state 0 was reserved but never filled")]
    fn building_with_an_unfilled_reservation_panics() {
        let mut builder = NfaBuilder::new();
        builder.reserve();
        let accept = builder.push(State::Match { token: 0 });
        builder.build(&[accept]);
    }

    #[test]
    #[should_panic(expected = "already filled")]
    fn filling_the_same_state_twice_panics() {
        let mut builder = NfaBuilder::new();
        let slot = builder.reserve();
        builder.fill(slot, State::Match { token: 0 });
        builder.fill(slot, State::Match { token: 1 });
    }

    #[test]
    #[should_panic(expected = "no such state")]
    fn filling_a_state_that_was_never_reserved_panics() {
        let mut builder = NfaBuilder::new();
        builder.fill(StateId::new(3), State::Match { token: 0 });
    }

    #[test]
    #[should_panic(expected = "state 0 points at 9")]
    fn building_with_a_first_split_branch_outside_the_arena_panics() {
        let mut builder = NfaBuilder::new();
        let split = builder.reserve();
        let accept = builder.push(State::Match { token: 0 });
        builder.fill(
            split,
            State::Split {
                first: StateId::new(9),
                second: accept,
            },
        );
        builder.build(&[split]);
    }

    #[test]
    #[should_panic(expected = "state 0 points at 9")]
    fn building_with_a_range_successor_outside_the_arena_panics() {
        let mut builder = NfaBuilder::new();
        let range = builder.push(State::Range {
            low: b'a',
            high: b'z',
            next: StateId::new(9),
        });
        builder.build(&[range]);
    }

    #[test]
    #[should_panic(expected = "state 0 points at 9")]
    fn building_with_a_second_split_branch_outside_the_arena_panics() {
        let mut builder = NfaBuilder::new();
        let split = builder.reserve();
        let accept = builder.push(State::Match { token: 0 });
        builder.fill(
            split,
            State::Split {
                first: accept,
                second: StateId::new(9),
            },
        );
        builder.build(&[split]);
    }

    #[test]
    #[should_panic(expected = "state 1 has an empty byte range 0x7a..=0x61")]
    fn building_with_an_inverted_range_panics() {
        let mut builder = NfaBuilder::new();
        let accept = builder.push(State::Match { token: 0 });
        let range = builder.push(State::Range {
            low: b'z',
            high: b'a',
            next: accept,
        });
        builder.build(&[range]);
    }

    #[test]
    fn a_range_bound_may_touch_itself() {
        let mut builder = NfaBuilder::new();
        let accept = builder.push(State::Match { token: 0 });
        let range = builder.push(State::Range {
            low: b'a',
            high: b'a',
            next: accept,
        });
        let nfa = builder.build(&[range]);

        let mut out = Vec::new();
        nfa.step(&[range], b'a', &mut out);
        assert_eq!(out, vec![accept]);
    }

    #[test]
    #[should_panic(expected = "start 0 points at 9, outside")]
    fn building_with_a_start_outside_the_arena_panics() {
        let mut builder = NfaBuilder::new();
        builder.push(State::Match { token: 0 });
        builder.build(&[StateId::new(9)]);
    }

    #[test]
    #[should_panic(expected = "start 1 points at 9, outside")]
    fn building_with_a_later_start_outside_the_arena_panics() {
        let mut builder = NfaBuilder::new();
        let accept = builder.push(State::Match { token: 0 });
        builder.build(&[accept, StateId::new(9)]);
    }

    #[test]
    #[should_panic(expected = "at least one start state")]
    fn building_without_a_start_panics() {
        let mut builder = NfaBuilder::new();
        builder.push(State::Match { token: 0 });
        builder.build(&[]);
    }
}
