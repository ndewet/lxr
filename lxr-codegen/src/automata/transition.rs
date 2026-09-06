use super::id::StateId;

/// Moves to `target` when `label` matches the input symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Transition<L> {
    /// The transition condition.
    pub(crate) label: L,
    /// The destination state.
    pub(crate) target: StateId,
}
