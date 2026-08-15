use crate::automata::StateId;

/// A part of an automaton that is not complete.
///
/// A fragment has one entry state and one exit state. Each path through the
/// fragment starts at the entry and stops at the exit.
///
/// The exit has no transition and no accept. Thus the caller can join the
/// fragment to another fragment, or repeat it, and the paths of the two
/// fragments stay separate.
///
/// A fragment holds no state. The states are in the
/// [`NfaBuilder`](crate::automata::NfaBuilder) that made them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fragment {
    entry: StateId,
    exit: StateId,
}

impl Fragment {
    /// Creates a fragment that starts at `entry` and stops at `exit`.
    pub fn new(entry: StateId, exit: StateId) -> Self {
        Self { entry, exit }
    }

    /// Returns the state at which the fragment starts.
    pub fn entry(&self) -> StateId {
        self.entry
    }

    /// Returns the state at which the fragment stops.
    pub fn exit(&self) -> StateId {
        self.exit
    }
}
