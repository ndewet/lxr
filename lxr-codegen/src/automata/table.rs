use super::adjacency::{AdjacencyList, AdjacencyListBuilder};
use super::{BuildError, StateId, Transition};

/// The shared states of a finite automaton.
///
/// The table holds labeled transitions, accept values, and start states. An
/// NFA adds its epsilon transitions. A DFA needs no other state storage.
///
/// The table does not require disjoint transition labels. An NFA permits
/// overlapping labels. A DFA must establish that stronger invariant itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StateTable<L, A> {
    transitions: AdjacencyList<Transition<L>>,
    accepts: Vec<Option<A>>,
    starts: Vec<StateId>,
}

impl<L, A> StateTable<L, A> {
    /// Creates a state table from its parallel storage.
    ///
    /// # Panics
    ///
    /// This function panics for each of these conditions:
    ///
    /// - The adjacency list does not have one group for each state.
    /// - There is no start state, or a start state is outside the table.
    /// - A transition target is outside the table.
    pub(crate) fn new(
        transitions: AdjacencyList<Transition<L>>,
        accepts: Vec<Option<A>>,
        starts: Vec<StateId>,
    ) -> Self {
        let state_count = accepts.len();
        assert_eq!(
            transitions.state_count(),
            state_count,
            "an automaton needs one group of transitions for each of its {state_count} states"
        );
        assert!(
            !starts.is_empty(),
            "an automaton needs at least one start state"
        );
        for (index, start) in starts.iter().enumerate() {
            assert!(
                start.index() < state_count,
                "start {index} points at {}, outside an automaton of {state_count} states",
                start.index()
            );
        }

        let table = Self {
            transitions,
            accepts,
            starts,
        };
        for index in 0..state_count {
            let state = StateId::new(index);
            for transition in table.transitions(state) {
                table.check_target(state, transition.target);
            }
        }
        table
    }

    /// Returns the number of states in the table.
    pub(crate) fn state_count(&self) -> usize {
        self.accepts.len()
    }

    /// Returns the labeled transitions from `state`.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is outside the table.
    pub(crate) fn transitions(&self, state: StateId) -> &[Transition<L>] {
        self.transitions
            .get(state)
            .unwrap_or_else(|| state.outside(self.state_count()))
    }

    /// Returns `true` if `state` accepts.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is outside the table.
    pub(crate) fn accepts(&self, state: StateId) -> bool {
        self.accept(state).is_some()
    }

    /// Returns the accept value of `state`, if the state accepts.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is outside the table.
    pub(crate) fn accept(&self, state: StateId) -> Option<&A> {
        self.accepts
            .get(state.index())
            .unwrap_or_else(|| state.outside(self.state_count()))
            .as_ref()
    }

    /// Returns the start states in their declared sequence.
    pub(crate) fn start_states(&self) -> &[StateId] {
        &self.starts
    }

    /// Returns the start state at `index`.
    ///
    /// # Panics
    ///
    /// This function panics if `index` is outside the start states.
    pub(crate) fn start_state(&self, index: usize) -> StateId {
        *self.starts.get(index).unwrap_or_else(|| {
            panic!(
                "start {index} is outside an automaton with {} start states",
                self.starts.len()
            )
        })
    }

    /// Verifies that `target` is in the table.
    ///
    /// An automaton can use this function to verify a separate group of
    /// transitions.
    ///
    /// # Panics
    ///
    /// This function panics if `target` is outside the table.
    pub(crate) fn check_target(&self, from: StateId, target: StateId) {
        assert!(
            target.index() < self.state_count(),
            "state {} points at {}, outside an automaton of {} states",
            from.index(),
            target.index(),
            self.state_count()
        );
    }
}

/// A shared state table that is not complete.
///
/// The builder owns the parts that NFA and DFA construction share. Each
/// automaton builder adds the storage and invariants specific to its kind.
#[derive(Debug)]
pub(crate) struct StateTableBuilder<L, A> {
    transitions: AdjacencyListBuilder<Transition<L>>,
    accepts: Vec<Option<A>>,
    capacity: usize,
    error: Option<BuildError>,
}

impl<L, A> StateTableBuilder<L, A> {
    /// Creates a builder that holds no state.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Creates a builder that holds at most `capacity` states.
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            transitions: AdjacencyListBuilder::new(),
            accepts: Vec::new(),
            capacity,
            error: None,
        }
    }

    /// Returns the number of states that the builder holds.
    pub(crate) fn state_count(&self) -> usize {
        self.accepts.len()
    }

    /// Returns `true` if the builder recorded an error.
    pub(crate) fn has_error(&self) -> bool {
        self.error.is_some()
    }

    /// Returns `true` if the builder holds `state`.
    pub(crate) fn contains(&self, state: StateId) -> bool {
        state.index() < self.state_count()
    }

    /// Adds a state, then returns its identifier.
    ///
    /// An addition past the capacity records a [`BuildError`] and returns a
    /// placeholder identifier.
    pub(crate) fn add_state(&mut self) -> StateId {
        if self.state_count() >= self.capacity {
            self.error = Some(BuildError::TooManyStates {
                capacity: self.capacity,
            });
            return StateId::new(0);
        }

        let state = StateId::new(self.state_count());
        self.accepts.push(None);
        state
    }

    /// Adds a labeled transition from `from` to `to`.
    ///
    /// `to` can refer to a state that the caller adds later.
    ///
    /// # Panics
    ///
    /// This function panics if `from` is not in the builder.
    pub(crate) fn add_transition(&mut self, from: StateId, label: L, to: StateId) {
        if self.has_error() {
            return;
        }
        assert!(
            self.contains(from),
            "cannot add a transition at {}: no such state",
            from.index()
        );
        self.transitions.add(from, Transition { label, target: to });
    }

    /// Sets the accept value of `state`, then returns its prior value.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not in the builder.
    pub(crate) fn set_accept(&mut self, state: StateId, accept: A) -> Option<A> {
        if self.has_error() {
            return None;
        }
        let slot = self
            .accepts
            .get_mut(state.index())
            .unwrap_or_else(|| panic!("cannot accept at {}: no such state", state.index()));
        slot.replace(accept)
    }

    /// Builds a table with the given start states.
    ///
    /// # Errors
    ///
    /// This function returns a [`BuildError`] if the state or transition
    /// storage went past its capacity.
    ///
    /// # Panics
    ///
    /// This function panics if a start state or transition target is invalid.
    pub(crate) fn build(self, starts: &[StateId]) -> Result<StateTable<L, A>, BuildError> {
        if let Some(error) = self.error {
            return Err(error);
        }

        let transitions = self.transitions.build(self.accepts.len())?;
        Ok(StateTable::new(transitions, self.accepts, starts.to_vec()))
    }
}

impl<L, A> Default for StateTableBuilder<L, A> {
    fn default() -> Self {
        Self::with_capacity(StateId::CAPACITY)
    }
}
