/// An index into the state arena of an [`Nfa`](super::Nfa).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateId(u32);

impl StateId {
    /// Creates a `StateId` from an index into the state arena.
    ///
    /// # Panics
    ///
    /// This function panics if `index` is above `u32::MAX`.
    pub fn new(index: usize) -> Self {
        Self(u32::try_from(index).expect("an NFA holds at most u32::MAX + 1 states"))
    }

    /// Returns the index into the state arena that this identifier refers to.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// An accept of an [`Nfa`](super::Nfa). A scan does an accept when a match is complete.
///
/// The automaton does not know the meaning of an accept. It holds only the identifier. The
/// component that builds the automaton holds the table for that identifier.
///
/// Two accepts are equal only if their identifiers are equal. Thus determinization and
/// minimization can divide the accept states correctly. They do not have to know the meaning of
/// an accept.
///
/// A low identifier has precedence. If a scan reaches more than one accept at the same length,
/// the lowest identifier is the result. Thus give the identifiers in the sequence of precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AcceptId(u32);

impl AcceptId {
    /// Creates an `AcceptId` from an index of an accept.
    ///
    /// # Panics
    ///
    /// This function panics if `index` is above `u32::MAX`.
    pub fn new(index: usize) -> Self {
        Self(u32::try_from(index).expect("an automaton has at most u32::MAX + 1 accepts"))
    }

    /// Returns the index of the accept that this identifier refers to.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// A state of an [`Nfa`](super::Nfa).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Reads one byte from `low` to `high`, then goes to `next`.
    Range { low: u8, high: u8, next: StateId },
    /// Goes to `first` and to `second`. It reads no byte.
    Split { first: StateId, second: StateId },
    /// Does the accept `accept`. It goes to no other state.
    Match { accept: AcceptId },
}

impl State {
    /// Returns an iterator over the states that this state goes to.
    pub fn successors(self) -> impl Iterator<Item = StateId> {
        let (first, second) = match self {
            Self::Range { next, .. } => (Some(next), None),
            Self::Split { first, second } => (Some(first), Some(second)),
            Self::Match { .. } => (None, None),
        };
        first.into_iter().chain(second)
    }

    /// Returns an iterator over the states that this state goes to without a byte.
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
    fn an_accept_id_round_trips_through_its_index() {
        assert_eq!(AcceptId::new(0).index(), 0);
        let last = u32::MAX as usize;
        assert_eq!(AcceptId::new(last).index(), last);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    #[should_panic(expected = "at most u32::MAX + 1 accepts")]
    fn an_accept_id_past_the_last_index_panics() {
        AcceptId::new(u32::MAX as usize + 1);
    }

    #[test]
    fn a_lower_accept_id_takes_precedence() {
        assert!(AcceptId::new(0) < AcceptId::new(1));
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
        let state = State::Match {
            accept: AcceptId::new(0),
        };
        assert_eq!(ids(state.epsilon_successors()), Vec::<usize>::new());
        assert_eq!(ids(state.successors()), Vec::<usize>::new());
    }
}
