use super::label::Label;

/// A label that one can divide into the classes of its alphabet.
///
/// Determinization reads the labels that leave one set of states. Two symbols that each of those
/// labels treats in the same manner give the same result, thus they belong to one class. This
/// trait computes those classes. One class gives one transition of the deterministic automaton.
///
/// The trait keeps the alphabet outside the automata module. A byte label divides into byte
/// ranges, and a character label divides into character ranges. Determinization reads only this
/// trait, thus it knows neither alphabet.
pub trait Divisible: Label + Sized {
    /// Divides `labels` into disjoint classes, then returns one label for each class.
    ///
    /// The result obeys four conditions:
    ///
    /// 1. Two labels of the result match no symbol in common.
    /// 2. Each label of the result matches at least one symbol.
    /// 3. The result matches each symbol that a label of `labels` matches, and no other symbol.
    /// 4. A label of `labels` matches each symbol of a label of the result, or no symbol of it.
    ///
    /// Condition 4 is the reason for the trait. It lets determinization read one symbol of a class
    /// and get the answer for the whole class.
    ///
    /// An empty `labels` gives an empty result.
    fn divide(labels: &[Self]) -> Vec<Self>;

    /// Returns any symbol that this label matches.
    ///
    /// Determinization reads this symbol to find the transitions of a class. Condition 2 of
    /// [`divide`](Self::divide) gives each class at least one symbol, thus determinization never
    /// reaches the panic.
    ///
    /// # Panics
    ///
    /// This function panics if the label matches no symbol.
    fn any_symbol(&self) -> Self::Symbol;
}
