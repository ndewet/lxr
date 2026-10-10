/// Identifies a state in one finite automaton.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct StateId(u32);

impl StateId {
    /// The maximum number of states in one automaton.
    pub(crate) const CAPACITY: usize = u32::MAX as usize;

    /// Marks a state allocation that a poisoned builder rejected.
    pub(crate) const REJECTED: Self = Self(u32::MAX);

    /// Creates an identifier for `index`.
    ///
    /// # Panics
    ///
    /// This function panics if `index` is not below [`CAPACITY`](Self::CAPACITY).
    pub(crate) fn new(index: usize) -> Self {
        assert!(
            index < Self::CAPACITY,
            "an automaton holds at most {} states",
            Self::CAPACITY
        );
        Self(index as u32)
    }

    /// Returns the state index.
    pub(crate) fn index(self) -> usize {
        assert!(self != Self::REJECTED, "a rejected state has no index");
        self.0 as usize
    }

    /// Reports an identifier that is outside an automaton.
    ///
    /// # Panics
    ///
    /// This function panics when called.
    pub(crate) fn outside(self, count: usize) -> ! {
        panic!(
            "state {} is outside an automaton of {count} states",
            self.index()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_state_id_round_trips_through_its_index() {
        assert_eq!(StateId::new(0).index(), 0);
        let last = StateId::CAPACITY - 1;
        assert_eq!(StateId::new(last).index(), last);
    }

    #[test]
    #[should_panic(expected = "an automaton holds at most 4294967295 states")]
    fn a_state_id_past_the_last_index_panics() {
        StateId::new(StateId::CAPACITY);
    }

    #[test]
    #[should_panic(expected = "a rejected state has no index")]
    fn a_rejected_state_has_no_index() {
        StateId::REJECTED.index();
    }
}
