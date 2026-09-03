/// An index into the states of a finite automaton.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateId(u32);

impl StateId {
    /// The number of states that an automaton can hold.
    pub const CAPACITY: usize = (u32::MAX as usize).saturating_add(1);

    /// Creates a `StateId` from a state index.
    ///
    /// # Panics
    ///
    /// This function panics if `index` is not below [`CAPACITY`](Self::CAPACITY).
    pub(crate) fn new(index: usize) -> Self {
        Self(u32::try_from(index).expect("an automaton holds at most u32::MAX + 1 states"))
    }

    /// Returns the state index that this identifier refers to.
    pub fn index(self) -> usize {
        self.0 as usize
    }

    /// Reports this identifier as outside an automaton of `count` states.
    ///
    /// # Panics
    ///
    /// This function panics each time. Call it only for an identifier that the automaton does not
    /// hold.
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
