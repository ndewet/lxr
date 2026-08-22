use super::id::StateId;

/// One transition of an [`Automaton`]. The automaton reads one symbol, then it goes to `target`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition<L> {
    /// The condition that a symbol obeys to move the automaton along this transition.
    pub label: L,
    /// The state that this transition goes to.
    pub target: StateId,
}

/// A finite automaton.
///
/// The automaton holds the states, the transitions, the states that accept, and one or more start
/// states. This trait reads that structure, and it starts no scan. To scan the automaton, use
/// [`Scanner`](super::Scanner).
///
/// The automaton knows which states accept. It does not know what an accept means. A lexer, for
/// example, holds the token of each state that accepts, and the automaton does not.
///
/// The trait puts no condition on the label. A pass that reads only the structure, for example
/// minimization, thus needs no alphabet.
///
/// A nondeterministic automaton and a deterministic automaton implement this trait. Thus one pass
/// reads either of them with the same code.
pub trait Automaton {
    /// The condition on a transition of this automaton.
    type Label;

    /// Returns the number of the states in the state arena.
    fn state_count(&self) -> usize;

    /// Returns the transitions that leave the state that `id` refers to.
    ///
    /// # Panics
    ///
    /// This function panics if `id` is not in the state arena.
    fn transitions(&self, id: StateId) -> &[Transition<Self::Label>];

    /// Returns `true` if the state that `id` refers to accepts.
    ///
    /// # Panics
    ///
    /// This function panics if `id` is not in the state arena.
    fn accepts(&self, id: StateId) -> bool;

    /// Returns the state at which each start begins a scan, the first start first.
    fn start_states(&self) -> &[StateId];

    /// Returns the number of the start states of the automaton.
    fn start_count(&self) -> usize {
        self.start_states().len()
    }

    /// Returns the state at which the automaton starts a scan under `start`.
    ///
    /// # Panics
    ///
    /// This function panics if `start` is not a start state of this automaton.
    #[allow(dead_code, reason = "the tests scan an automaton with this API")]
    fn start_state(&self, start: usize) -> StateId {
        let starts = self.start_states();
        *starts.get(start).unwrap_or_else(|| {
            panic!(
                "start {start} is outside an automaton with {} start states",
                starts.len()
            )
        })
    }
}
