use super::arena::Arena;
use super::automaton::Transition;
use super::id::StateId;

/// The states of an automaton, and the transitions that leave each one.
///
/// The table holds the transitions with a label, the states that accept, and the start states. A
/// nondeterministic automaton holds one of these and its epsilon transitions. A deterministic
/// automaton holds one of these and nothing else. Thus the two automata share their storage, their
/// bounds checks, and the text of each panic.
///
/// The table does not know if the labels of one state are disjoint. Only a deterministic automaton
/// obeys that condition, and only [`Dfa::step`](super::DeterministicFiniteAutomaton::step) reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateTable<L> {
    transitions: Arena<Transition<L>>,
    accepts: Vec<bool>,
    starts: Vec<StateId>,
}

impl<L> StateTable<L> {
    /// Creates a `Table` from the transitions, the accepts, and the start states.
    ///
    /// # Panics
    ///
    /// This function panics for each of these conditions:
    ///
    /// - The arena holds a group for a state that `accepts` does not hold.
    /// - `starts` is empty, or a start state is not in the state arena.
    /// - The target of a transition is not in the state arena.
    pub(super) fn new(
        transitions: Arena<Transition<L>>,
        accepts: Vec<bool>,
        starts: Vec<StateId>,
    ) -> Self {
        let count = accepts.len();
        assert_eq!(
            transitions.group_count(),
            count,
            "an automaton needs one group of transitions for each of its {count} states"
        );
        assert!(
            !starts.is_empty(),
            "an automaton needs at least one start state"
        );
        for (index, start) in starts.iter().enumerate() {
            assert!(
                start.index() < count,
                "start {index} points at {}, outside an arena of {count} states",
                start.index()
            );
        }

        let table = Self {
            transitions,
            accepts,
            starts,
        };
        for index in 0..count {
            for transition in table.transitions.get(index).into_iter().flatten() {
                table.check_target(index, transition.target);
            }
        }
        table
    }

    /// Returns the number of the states in the state arena.
    pub fn state_count(&self) -> usize {
        self.accepts.len()
    }

    /// Returns the transitions that leave the state that `id` refers to.
    ///
    /// # Panics
    ///
    /// This function panics if `id` is not in the state arena.
    pub fn transitions(&self, id: StateId) -> &[Transition<L>] {
        self.transitions
            .get(id.index())
            .unwrap_or_else(|| self.outside(id))
    }

    /// Returns `true` if the state that `id` refers to accepts.
    ///
    /// # Panics
    ///
    /// This function panics if `id` is not in the state arena.
    pub fn accepts(&self, id: StateId) -> bool {
        *self
            .accepts
            .get(id.index())
            .unwrap_or_else(|| self.outside(id))
    }

    /// Returns the state at which each start begins a scan, the first start first.
    pub fn start_states(&self) -> &[StateId] {
        &self.starts
    }

    /// Panics if `target` is not in the state arena.
    ///
    /// `from` is the state that the transition leaves. An automaton that holds another arena of
    /// transitions checks that arena with this function.
    ///
    /// # Panics
    ///
    /// This function panics if `target` is not in the state arena.
    pub(super) fn check_target(&self, from: usize, target: StateId) {
        assert!(
            target.index() < self.state_count(),
            "state {from} points at {}, outside an arena of {} states",
            target.index(),
            self.state_count()
        );
    }

    /// Panics for a state that the state arena does not hold.
    ///
    /// Each automaton reports a state outside its arena with the same message.
    pub(super) fn outside(&self, id: StateId) -> ! {
        panic!(
            "state {} is outside an arena of {} states",
            id.index(),
            self.state_count()
        )
    }
}
