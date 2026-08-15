use super::id::StateId;

/// One transition of an [`Automaton`](super::Automaton). The automaton reads one symbol, then it
/// goes to `target`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition<L> {
    /// The condition that a symbol obeys to move the automaton along this transition.
    pub label: L,
    /// The state that this transition goes to.
    pub target: StateId,
}
