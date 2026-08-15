/// The condition on a transition of an automaton.
///
/// A label says which symbols move the automaton along the transition. The automaton does not know
/// the alphabet. It reads only this trait. Thus one automaton serves a byte alphabet, a character
/// alphabet, or another alphabet.
pub trait Label {
    /// One symbol of the alphabet that the automaton reads.
    type Symbol: Copy;

    /// Returns `true` if `symbol` moves the automaton along the transition.
    fn matches(&self, symbol: Self::Symbol) -> bool;
}
