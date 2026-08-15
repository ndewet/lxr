/// An index into the state arena of an [`Automaton`](super::Automaton).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateId(u32);

impl StateId {
    /// Creates a `StateId` from an index into the state arena.
    ///
    /// # Panics
    ///
    /// This function panics if `index` is above `u32::MAX`.
    pub fn new(index: usize) -> Self {
        Self(u32::try_from(index).expect("an automaton holds at most u32::MAX + 1 states"))
    }

    /// Returns the index into the state arena that this identifier refers to.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// An index of a start state of an [`Automaton`](super::Automaton).
///
/// An automaton has one or more start states. A scan starts at one of them. The automaton does not
/// know why the caller selects one start state. A lexer selects the start state of its start
/// condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StartId(u32);

impl StartId {
    /// Creates a `StartId` from an index of a start state.
    ///
    /// # Panics
    ///
    /// This function panics if `index` is above `u32::MAX`.
    pub fn new(index: usize) -> Self {
        Self(u32::try_from(index).expect("an automaton has at most u32::MAX + 1 start states"))
    }

    /// Returns the index of the start state that this identifier refers to.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_state_id_round_trips_through_its_index() {
        assert_eq!(StateId::new(0).index(), 0);
        let last = u32::MAX as usize;
        assert_eq!(StateId::new(last).index(), last);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    #[should_panic(expected = "an automaton holds at most u32::MAX + 1 states")]
    fn a_state_id_past_the_last_index_panics() {
        StateId::new(u32::MAX as usize + 1);
    }

    #[test]
    fn a_start_id_round_trips_through_its_index() {
        assert_eq!(StartId::new(0).index(), 0);
        let last = u32::MAX as usize;
        assert_eq!(StartId::new(last).index(), last);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    #[should_panic(expected = "at most u32::MAX + 1 start states")]
    fn a_start_id_past_the_last_index_panics() {
        StartId::new(u32::MAX as usize + 1);
    }
}
