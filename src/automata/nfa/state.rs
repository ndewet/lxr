/// An index into the state arena of an [`Nfa`](super::Nfa).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateId(u32);

impl StateId {
    /// Creates a `StateId` from an arena index.
    ///
    /// # Panics
    ///
    /// Panics if `index` is greater than `u32::MAX`.
    pub fn new(index: usize) -> Self {
        Self(u32::try_from(index).expect("an NFA holds at most u32::MAX + 1 states"))
    }

    /// Returns the arena index this identifier refers to.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// A state of an [`Nfa`](super::Nfa).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Consumes one byte in `low..=high` and moves to `next`.
    Range { low: u8, high: u8, next: StateId },
    /// Moves to both `first` and `second` without consuming a byte.
    Split { first: StateId, second: StateId },
    /// Accepts `token` and leads nowhere.
    Match { token: u32 },
}

impl State {
    /// Returns an iterator over the states this state leads to.
    pub fn successors(self) -> impl Iterator<Item = StateId> {
        let (first, second) = match self {
            Self::Range { next, .. } => (Some(next), None),
            Self::Split { first, second } => (Some(first), Some(second)),
            Self::Match { .. } => (None, None),
        };
        first.into_iter().chain(second)
    }

    /// Returns an iterator over the states this state leads to without consuming a byte.
    pub fn epsilon_successors(self) -> impl Iterator<Item = StateId> {
        match self {
            Self::Range { .. } => None,
            free => Some(free.successors()),
        }
        .into_iter()
        .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(states: impl Iterator<Item = StateId>) -> Vec<usize> {
        states.map(StateId::index).collect()
    }

    #[test]
    fn a_state_id_round_trips_through_its_index() {
        assert_eq!(StateId::new(0).index(), 0);
        let last = u32::MAX as usize;
        assert_eq!(StateId::new(last).index(), last);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    #[should_panic(expected = "at most u32::MAX + 1 states")]
    fn a_state_id_past_the_last_index_panics() {
        StateId::new(u32::MAX as usize + 1);
    }

    #[test]
    fn a_range_is_not_an_epsilon_edge() {
        let state = State::Range {
            low: b'a',
            high: b'z',
            next: StateId::new(7),
        };
        assert_eq!(ids(state.epsilon_successors()), Vec::<usize>::new());
        assert_eq!(ids(state.successors()), vec![7]);
    }

    #[test]
    fn a_split_leads_to_both_branches() {
        let state = State::Split {
            first: StateId::new(1),
            second: StateId::new(2),
        };
        assert_eq!(ids(state.epsilon_successors()), vec![1, 2]);
        assert_eq!(ids(state.successors()), vec![1, 2]);
    }

    #[test]
    fn a_match_leads_nowhere() {
        let state = State::Match { token: 0 };
        assert_eq!(ids(state.epsilon_successors()), Vec::<usize>::new());
        assert_eq!(ids(state.successors()), Vec::<usize>::new());
    }
}
