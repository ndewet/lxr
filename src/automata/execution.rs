use super::id::StartId;

/// One scan of an automaton, in progress.
///
/// An automaton holds no state of a scan. It is read only. An execution holds where the scan is,
/// and the buffers that the scan needs. Thus a step makes no allocation, and one execution scans a
/// sequence of tokens.
///
/// To make an execution, use [`Automaton::execute`](super::Automaton::execute).
pub trait Execution {
    /// One symbol of the alphabet that the automaton reads.
    type Symbol: Copy;

    /// The meaning of an accept. The automaton does not read it.
    type Accept;

    /// Puts the execution back at the start state that `start` refers to.
    ///
    /// # Panics
    ///
    /// This function panics if `start` is not a start state of the automaton.
    fn restart(&mut self, start: StartId);

    /// Reads `symbol`, then moves the execution.
    ///
    /// Returns `false` if the execution reaches no state. The execution then accepts nothing, and
    /// each later step also gives `false`. To scan again, use [`restart`](Self::restart).
    fn step(&mut self, symbol: Self::Symbol) -> bool;

    /// Returns each accept that the execution reached.
    ///
    /// A deterministic automaton gives no accept or one accept. A nondeterministic automaton gives
    /// the accept of each state that it is in. The caller selects one of them, thus the automaton
    /// holds no rule of precedence.
    fn accepts(&self) -> impl Iterator<Item = &Self::Accept>;
}
