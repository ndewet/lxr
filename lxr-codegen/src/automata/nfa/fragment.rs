use crate::automata::StateId;

/// Identifies one incomplete NFA fragment.
///
/// The fragment owns no state. Its exit has no transition or accept value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Fragment {
    entry: StateId,
    exit: StateId,
}

impl Fragment {
    /// Creates a fragment from its boundary states.
    pub(crate) fn new(entry: StateId, exit: StateId) -> Self {
        Self { entry, exit }
    }

    /// Returns the entry state.
    pub(crate) fn entry(&self) -> StateId {
        self.entry
    }

    /// Returns the exit state.
    pub(crate) fn exit(&self) -> StateId {
        self.exit
    }
}
