/// An index into the state arena of an [`Automaton`](super::Automaton).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateId(u32);

impl StateId {
    /// The number of the states that a state arena holds.
    pub const CAPACITY: usize = (u32::MAX as usize).saturating_add(1);

    /// Creates a `StateId` from an index into the state arena.
    ///
    /// # Panics
    ///
    /// This function panics if `index` is not below [`CAPACITY`](Self::CAPACITY).
    pub fn new(index: usize) -> Self {
        Self(u32::try_from(index).expect("an automaton holds at most u32::MAX + 1 states"))
    }

    /// Returns the index into the state arena that this identifier refers to.
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
}
