/// The condition on a transition of an automaton.
///
/// A label says which symbols move the automaton along the transition. The automaton does not know
/// the alphabet. It reads only this trait. Thus one automaton serves a byte alphabet, a character
/// alphabet, or another alphabet.
///
/// A label matches at least one symbol. A transition that no symbol takes is a transition that the
/// automaton does not need, thus a builder adds no such transition.
///
/// [`divide`](Self::divide) keeps the alphabet outside the automata module. A byte label divides
/// into byte ranges, and a character label divides into character ranges. Determinization reads
/// only this trait, thus it knows neither alphabet.
pub trait Label: Clone {
    /// One symbol of the alphabet that the automaton reads.
    type Symbol: Copy;

    /// Returns `true` if `symbol` moves the automaton along the transition.
    fn matches(&self, symbol: Self::Symbol) -> bool;

    /// Returns `true` if each symbol that this label matches is below `symbol`.
    ///
    /// A deterministic automaton holds the transitions of one state in ascending sequence. Thus
    /// [`step`](super::DeterministicFiniteAutomaton::step) finds the transition of a symbol with a
    /// binary search, and it reads no other transition.
    #[allow(dead_code, reason = "the tests scan an automaton with this API")]
    fn below(&self, symbol: Self::Symbol) -> bool;

    /// Divides `labels` into disjoint classes, then returns each class with one of its symbols.
    ///
    /// Determinization reads the labels that leave one set of states. Two symbols that each of
    /// those labels treats in the same manner give the same result, thus they belong to one class.
    /// One class gives one transition of the deterministic automaton.
    ///
    /// The classes obey five conditions:
    ///
    /// 1. Two classes match no symbol in common.
    /// 2. Each class matches at least one symbol.
    /// 3. The classes match each symbol that a label of `labels` matches, and no other symbol.
    /// 4. A label of `labels` matches each symbol of a class, or no symbol of that class.
    /// 5. Each class is [`below`](Self::below) the symbol of the class after it.
    ///
    /// Condition 4 is the reason for the function. Determinization reads the symbol of a class and
    /// gets the answer for the whole class. The symbol arrives with the class, thus condition 2
    /// supplies it and no caller asks an empty label for a symbol.
    ///
    /// Condition 5 puts the transitions of one state of the deterministic automaton in ascending
    /// sequence. [`step`](super::DeterministicFiniteAutomaton::step) reads that sequence.
    ///
    /// An empty `labels` gives an empty result.
    fn divide(labels: &[Self]) -> Vec<(Self, Self::Symbol)>;
}
