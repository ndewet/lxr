/// The condition on a transition of an automaton.
///
/// A label says which symbols move the automaton along the transition. The automaton does not know
/// the alphabet. It reads only this trait. Thus one automaton serves a byte alphabet, a character
/// alphabet, or another alphabet.
///
/// A label matches at least one symbol. A transition that no symbol takes is a transition that the
/// automaton does not need, thus a builder adds no such transition.
pub trait Label: Clone {
    /// One symbol of the alphabet that the automaton reads.
    type Symbol: Copy;

    /// Returns `true` if `symbol` moves the automaton along the transition.
    fn matches(&self, symbol: Self::Symbol) -> bool;
}

pub(crate) struct LabelClass<L> {
    label: L,
    matching_labels: Vec<usize>,
}

impl<L> LabelClass<L> {
    pub(crate) fn new(label: L, matching_labels: Vec<usize>) -> Self {
        assert!(
            !matching_labels.is_empty(),
            "a label class must match at least one input label"
        );
        Self {
            label,
            matching_labels,
        }
    }

    pub(crate) fn get_label(&self) -> &L {
        &self.label
    }

    pub(crate) fn get_matching_labels(&self) -> &Vec<usize> {
        &self.matching_labels
    }
}

/// A label that can be split with other labels into disjoint classes.
pub(crate) trait Partitionable: Label + Sized {
    /// Splits `labels` into classes with these rules:
    ///
    /// - Each class matches at least one symbol and one input label.
    /// - Classes do not overlap.
    /// - The classes cover exactly the symbols matched by the input labels.
    /// - Each input label is made of whole classes.
    /// - Each class records every matching input label.
    /// - Class order does not depend on input order.
    ///
    /// Each recorded index is the position of a matching label in `labels`.
    fn partition(labels: &[Self]) -> Vec<LabelClass<Self>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "a label class must match at least one input label")]
    fn a_label_class_must_match_at_least_one_input_label() {
        LabelClass::new((), Vec::new());
    }
}
