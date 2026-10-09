/// Defines the symbols that take a transition.
///
/// Each label must match at least one symbol.
pub(crate) trait Label: Clone {
    /// A symbol in the input alphabet.
    type Symbol: Copy;

    /// Returns whether this label matches `symbol`.
    fn matches(&self, symbol: Self::Symbol) -> bool;
}

/// Holds one disjoint label and the input labels that contain it.
pub(crate) struct LabelClass<L> {
    label: L,
    matching_labels: Vec<usize>,
}

impl<L> LabelClass<L> {
    /// Creates a label class.
    ///
    /// # Panics
    ///
    /// This function panics if `matching_labels` is empty.
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

    /// Returns the disjoint label.
    pub(crate) fn get_label(&self) -> &L {
        &self.label
    }

    /// Returns the indexes of the matching input labels.
    pub(crate) fn get_matching_labels(&self) -> &Vec<usize> {
        &self.matching_labels
    }
}

/// Splits labels into disjoint classes.
pub(crate) trait Partitionable: Label + Sized {
    /// Partitions `labels` by their matching input indexes.
    ///
    /// The classes are disjoint and cover the exact union of `labels`.
    /// Their sequence does not depend on the input sequence.
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
