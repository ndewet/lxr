use super::execution::Execution;
use super::id::StartId;

/// An automaton that scans a sequence of symbols.
///
/// An automaton holds the states, the transitions, and the accepts. It holds no state of a scan,
/// thus two scans can read the same automaton at the same time. One [`Execution`] holds the state
/// of one scan.
///
/// A nondeterministic automaton and a deterministic automaton implement this trait. Thus a scanner
/// reads either of them with the same code.
pub trait Automaton {
    /// One symbol of the alphabet that the automaton reads.
    type Symbol: Copy;

    /// The meaning of an accept. The automaton does not read it.
    type Accept;

    /// The execution of this automaton.
    type Execution<'a>: Execution<Symbol = Self::Symbol, Accept = Self::Accept>
    where
        Self: 'a;

    /// Starts an execution at the start state that `start` refers to.
    ///
    /// # Panics
    ///
    /// This function panics if `start` is not a start state of this automaton.
    fn execute(&self, start: StartId) -> Self::Execution<'_>;
}
