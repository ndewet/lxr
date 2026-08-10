use super::state::{State, StateId};

/// An entry point of an automaton — one start condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StartId(u32);

impl StartId {
    /// Creates a `StartId` from a start condition index.
    ///
    /// # Panics
    ///
    /// Panics if `index` is greater than `u32::MAX`.
    pub fn new(index: usize) -> Self {
        Self(u32::try_from(index).expect("an automaton has at most u32::MAX + 1 start conditions"))
    }

    /// Returns the start condition this identifier refers to.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// A nondeterministic finite automaton over bytes.
///
/// Built with [`NfaBuilder`](super::NfaBuilder).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nfa {
    states: Vec<State>,
    starts: Vec<StateId>,
}

impl Nfa {
    /// Creates an `Nfa` from an arena of states and the states it is entered at.
    pub(super) fn new(states: Vec<State>, starts: Vec<StateId>) -> Self {
        Self { states, starts }
    }

    /// Returns the state `id` refers to.
    ///
    /// # Panics
    ///
    /// Panics if `id` is outside the arena.
    pub fn state(&self, id: StateId) -> State {
        *self.states.get(id.index()).unwrap_or_else(|| {
            panic!(
                "state {} is outside an arena of {} states",
                id.index(),
                self.states.len()
            )
        })
    }

    /// Returns the number of states in the arena.
    pub fn state_count(&self) -> usize {
        self.states.len()
    }

    /// Returns the number of start conditions the automaton has.
    pub fn start_count(&self) -> usize {
        self.starts.len()
    }

    /// Returns the state the automaton is entered at under `start`.
    ///
    /// # Panics
    ///
    /// Panics if `start` is not a start condition of this automaton.
    pub fn start_state(&self, start: StartId) -> StateId {
        *self.starts.get(start.index()).unwrap_or_else(|| {
            panic!(
                "start {} is outside an automaton with {} start conditions",
                start.index(),
                self.starts.len()
            )
        })
    }

    /// Returns an iterator over the start conditions and the states they are entered at.
    pub fn starts(&self) -> impl Iterator<Item = (StartId, StateId)> + '_ {
        self.starts
            .iter()
            .enumerate()
            .map(|(index, &state)| (StartId::new(index), state))
    }

    /// Writes the states reached from `states` by consuming `byte` into `out`, clearing it first.
    ///
    /// Epsilon edges are not followed; see
    /// [`Simulator::epsilon_closure`](super::Simulator::epsilon_closure).
    ///
    /// # Panics
    ///
    /// Panics if any of `states` is outside the arena.
    pub fn step(&self, states: &[StateId], byte: u8, out: &mut Vec<StateId>) {
        out.clear();
        out.extend(states.iter().filter_map(|&id| match self.state(id) {
            State::Range { low, high, next } if (low..=high).contains(&byte) => Some(next),
            _ => None,
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::super::builder::NfaBuilder;
    use super::super::state::AcceptId;
    use super::*;

    fn stepped(nfa: &Nfa, states: &[StateId], byte: u8) -> Vec<StateId> {
        let mut out = Vec::new();
        nfa.step(states, byte, &mut out);
        out
    }

    #[test]
    fn a_step_follows_a_range_that_contains_the_byte() {
        let mut builder = NfaBuilder::new();
        let accept = builder.push(State::Match {
            accept: AcceptId::new(0),
        });
        let range = builder.push(State::Range {
            low: b'a',
            high: b'z',
            next: accept,
        });
        let nfa = builder.build(&[range]);

        assert_eq!(stepped(&nfa, &[range], b'm'), vec![accept]);
    }

    #[test]
    fn a_step_ignores_a_range_that_excludes_the_byte() {
        let mut builder = NfaBuilder::new();
        let accept = builder.push(State::Match {
            accept: AcceptId::new(0),
        });
        let range = builder.push(State::Range {
            low: b'a',
            high: b'z',
            next: accept,
        });
        let nfa = builder.build(&[range]);

        assert_eq!(stepped(&nfa, &[range], b'A'), Vec::new());
    }

    #[test]
    fn a_step_does_not_follow_epsilon_edges() {
        let mut builder = NfaBuilder::new();
        let accept = builder.push(State::Match {
            accept: AcceptId::new(0),
        });
        let other = builder.push(State::Match {
            accept: AcceptId::new(1),
        });
        let split = builder.push(State::Split {
            first: accept,
            second: other,
        });
        let nfa = builder.build(&[split]);

        assert_eq!(stepped(&nfa, &[split], b'a'), Vec::new());
    }

    #[test]
    fn a_step_yields_a_shared_successor_once_per_range() {
        let mut builder = NfaBuilder::new();
        let accept = builder.push(State::Match {
            accept: AcceptId::new(0),
        });
        let lower = builder.push(State::Range {
            low: b'a',
            high: b'z',
            next: accept,
        });
        let vowel = builder.push(State::Range {
            low: b'e',
            high: b'e',
            next: accept,
        });
        let nfa = builder.build(&[lower, vowel]);

        assert_eq!(stepped(&nfa, &[lower, vowel], b'e'), vec![accept, accept]);
    }

    #[test]
    fn stepping_into_a_buffer_replaces_what_it_held() {
        let mut builder = NfaBuilder::new();
        let accept = builder.push(State::Match {
            accept: AcceptId::new(0),
        });
        let range = builder.push(State::Range {
            low: b'a',
            high: b'z',
            next: accept,
        });
        let nfa = builder.build(&[range]);

        let mut out = vec![range, range, range];
        nfa.step(&[range], b'm', &mut out);
        assert_eq!(out, vec![accept]);

        nfa.step(&[range], b'A', &mut out);
        assert_eq!(out, Vec::new());
    }

    #[test]
    fn a_start_id_round_trips_through_its_index() {
        assert_eq!(StartId::new(0).index(), 0);
        let last = u32::MAX as usize;
        assert_eq!(StartId::new(last).index(), last);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    #[should_panic(expected = "at most u32::MAX + 1 start conditions")]
    fn a_start_id_past_the_last_index_panics() {
        StartId::new(u32::MAX as usize + 1);
    }

    #[test]
    fn each_start_condition_names_its_own_entry_state() {
        let mut builder = NfaBuilder::new();
        let code = builder.push(State::Match {
            accept: AcceptId::new(0),
        });
        let string = builder.push(State::Match {
            accept: AcceptId::new(1),
        });
        let nfa = builder.build(&[code, string]);

        assert_eq!(nfa.start_count(), 2);
        assert_eq!(nfa.start_state(StartId::new(0)), code);
        assert_eq!(nfa.start_state(StartId::new(1)), string);
        assert_eq!(
            nfa.starts().collect::<Vec<_>>(),
            vec![(StartId::new(0), code), (StartId::new(1), string)]
        );
    }

    #[test]
    #[should_panic(expected = "start 2 is outside an automaton with 2 start conditions")]
    fn reading_a_start_outside_the_automaton_panics() {
        let mut builder = NfaBuilder::new();
        let code = builder.push(State::Match {
            accept: AcceptId::new(0),
        });
        let string = builder.push(State::Match {
            accept: AcceptId::new(1),
        });
        let nfa = builder.build(&[code, string]);

        nfa.start_state(StartId::new(2));
    }

    #[test]
    #[should_panic(expected = "state 9 is outside an arena of 2 states")]
    fn reading_a_state_outside_the_arena_panics() {
        let mut builder = NfaBuilder::new();
        let accept = builder.push(State::Match {
            accept: AcceptId::new(0),
        });
        builder.push(State::Match {
            accept: AcceptId::new(1),
        });
        let nfa = builder.build(&[accept]);

        nfa.state(StateId::new(9));
    }
}
