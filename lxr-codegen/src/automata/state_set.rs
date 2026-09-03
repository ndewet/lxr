use super::StateId;

/// A set of states from one finite automaton.
///
/// The set keeps its members in a vector and uses a membership table to make
/// insertion constant time. It reuses both allocations after a clear.
#[derive(Debug)]
pub(crate) struct StateSet {
    members: Vec<StateId>,
    membership: Vec<bool>,
}

impl StateSet {
    /// Creates an empty set for an automaton with `state_count` states.
    pub(crate) fn new(state_count: usize) -> Self {
        Self {
            members: Vec::new(),
            membership: vec![false; state_count],
        }
    }

    /// Adds `state` to the set and reports whether it was new.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is outside the automaton of this set.
    pub(crate) fn insert(&mut self, state: StateId) -> bool {
        let state_count = self.membership.len();
        let present = self
            .membership
            .get_mut(state.index())
            .unwrap_or_else(|| state.outside(state_count));
        if *present {
            return false;
        }

        *present = true;
        self.members.push(state);
        true
    }

    /// Removes all states and keeps the allocated storage.
    pub(crate) fn clear(&mut self) {
        for state in self.members.drain(..) {
            self.membership[state.index()] = false;
        }
    }

    /// Returns `true` if the set has no state.
    pub(crate) fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Returns the states in their current sequence.
    pub(crate) fn members(&self) -> &[StateId] {
        &self.members
    }

    /// Returns the member at `index`, if it exists.
    pub(crate) fn get(&self, index: usize) -> Option<StateId> {
        self.members.get(index).copied()
    }

    /// Sorts the states by their identifiers.
    pub(crate) fn sort(&mut self) {
        self.members.sort_unstable();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insertion_keeps_each_state_one_time() {
        let mut states = StateSet::new(3);

        assert!(states.insert(StateId::new(2)));
        assert!(states.insert(StateId::new(0)));
        assert!(!states.insert(StateId::new(2)));
        assert_eq!(states.members(), &[StateId::new(2), StateId::new(0)]);
    }

    #[test]
    fn a_clear_permits_each_state_again() {
        let mut states = StateSet::new(2);
        states.insert(StateId::new(1));

        states.clear();

        assert!(states.is_empty());
        assert!(states.insert(StateId::new(1)));
    }

    #[test]
    #[should_panic(expected = "state 2 is outside an automaton of 2 states")]
    fn insertion_of_a_state_outside_the_automaton_panics() {
        StateSet::new(2).insert(StateId::new(2));
    }
}
