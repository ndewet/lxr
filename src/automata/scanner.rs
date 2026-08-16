use super::automaton::Automaton;
use super::execution::Execution;

/// An automaton that scans a sequence of symbols.
///
/// The automaton holds no state of a scan. It is read only, thus two scans can read the same
/// automaton at the same time. One [`Execution`] holds the state of one scan, and
/// [`execute`](Self::execute) makes it.
///
/// [`Automaton`] gives the structure, and this trait gives the scan. A pass that reads only the
/// structure, for example minimization, thus needs no alphabet.
///
/// A nondeterministic automaton and a deterministic automaton implement this trait. Thus a lexer
/// scans either of them with the same code. The two differ in the cost of one step, and not in the
/// input that they accept.
pub trait Scanner: Automaton {
    /// One symbol of the alphabet that the automaton reads.
    type Symbol: Copy;

    /// The execution of this automaton.
    type Execution<'a>: Execution<Symbol = Self::Symbol>
    where
        Self: 'a;

    /// Starts an execution at the start state that `start` refers to.
    ///
    /// One execution scans a sequence of tokens. Make it one time, then call
    /// [`longest_match`](Execution::longest_match) on it for each token. Thus the scan makes the
    /// buffers one time, and not one time for each token.
    ///
    /// # Panics
    ///
    /// This function panics if `start` is not a start state of this automaton.
    fn execute(&self, start: usize) -> Self::Execution<'_>;
}
