use super::id::StateId;

/// One labeled transition of a finite automaton.
///
/// The automaton reads a symbol that matches `label`, then it moves to
/// `target`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition<L> {
    /// The condition that a symbol must obey to take this transition.
    pub label: L,
    /// The state that this transition enters.
    pub target: StateId,
}
