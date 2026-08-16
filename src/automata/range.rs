use super::label::Label;

/// A [`Label`] that matches each symbol between two ends.
///
/// An alphabet of an ordered and countable symbol gives a label of this shape. A byte range and a
/// character range are both one. The two ends are in the range.
///
/// The trait supplies [`divide`], thus each range label divides in the same manner. Only the walk
/// from one symbol to the next belongs to the alphabet. A character alphabet steps across the gap
/// at the values of the surrogates, and a byte alphabet steps by one.
pub trait Range: Label<Symbol: Ord> {
    /// The highest symbol of the alphabet.
    const LAST: Self::Symbol;

    /// Creates a label that matches each symbol from `low` to `high`.
    fn new(low: Self::Symbol, high: Self::Symbol) -> Self;

    /// Returns the lowest symbol that the label matches.
    fn low(&self) -> Self::Symbol;

    /// Returns the highest symbol that the label matches.
    fn high(&self) -> Self::Symbol;

    /// Returns the symbol after `symbol`, or `None` if `symbol` is [`LAST`](Self::LAST).
    fn after(symbol: Self::Symbol) -> Option<Self::Symbol>;

    /// Returns the symbol before `symbol`, or `None` if `symbol` is the first symbol.
    fn before(symbol: Self::Symbol) -> Option<Self::Symbol>;

    /// Returns the classes of `labels`, which are the ranges between their ends.
    ///
    /// The ends of the labels are the only symbols at which a label changes its answer. Thus the
    /// range from one end to the next end is one class. A range that no label matches is a gap
    /// between two labels, and the result leaves it out.
    ///
    /// The result is in ascending sequence, and it obeys each condition of [`Label::divide`]. Give
    /// this function to `divide` of a range label.
    fn classes(labels: &[Self]) -> Vec<(Self, Self::Symbol)> {
        let mut starts = Vec::with_capacity(labels.len() * 2);
        for label in labels {
            starts.push(label.low());
            if let Some(above) = Self::after(label.high()) {
                starts.push(above);
            }
        }
        starts.sort_unstable();
        starts.dedup();

        let mut classes = Vec::with_capacity(starts.len());
        for (index, &low) in starts.iter().enumerate() {
            let high = match starts.get(index + 1) {
                Some(&next) => Self::before(next)
                    .expect("a start is above the start before it, thus one is below it"),
                None => Self::LAST,
            };
            if labels.iter().any(|label| label.matches(low)) {
                classes.push((Self::new(low, high), low));
            }
        }
        classes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::testing::{ABOVE_GAP, BELOW_GAP, Symbols, only, range};

    /// Returns the classes of `labels`, without the symbol of each class.
    fn divided(labels: &[Symbols]) -> Vec<Symbols> {
        Symbols::classes(labels)
            .into_iter()
            .map(|(class, _)| class)
            .collect()
    }

    #[test]
    fn no_label_gives_no_class() {
        assert_eq!(divided(&[]), Vec::new());
    }

    #[test]
    fn two_labels_that_share_no_symbol_stay_separate() {
        assert_eq!(
            divided(&[range('a', 'c'), range('d', 'f')]),
            vec![range('a', 'c'), range('d', 'f')]
        );
    }

    #[test]
    fn two_labels_that_share_symbols_give_three_classes() {
        assert_eq!(
            divided(&[range('a', 'f'), range('d', 'z')]),
            vec![range('a', 'c'), range('d', 'f'), range('g', 'z')]
        );
    }

    #[test]
    fn two_equal_labels_give_one_class() {
        assert_eq!(divided(&[only('a'), only('a')]), vec![only('a')]);
    }

    #[test]
    fn the_classes_are_ascending() {
        assert_eq!(
            divided(&[range('d', 'z'), only('b'), range('a', 'f')]),
            vec![
                only('a'),
                only('b'),
                only('c'),
                range('d', 'f'),
                range('g', 'z'),
            ]
        );
    }

    #[test]
    fn a_label_that_reaches_the_last_symbol_gives_one_class() {
        assert_eq!(
            divided(&[range('a', char::MAX)]),
            vec![range('a', char::MAX)]
        );
    }

    #[test]
    fn a_class_steps_across_the_gap_of_the_alphabet() {
        assert_eq!(
            divided(&[range(BELOW_GAP, ABOVE_GAP), only(ABOVE_GAP)]),
            vec![only(BELOW_GAP), only(ABOVE_GAP)]
        );
    }

    #[test]
    fn each_class_arrives_with_a_symbol_that_it_matches() {
        let labels = [
            range('a', 'f'),
            range('d', 'z'),
            only('\0'),
            range(BELOW_GAP, ABOVE_GAP),
            only(char::MAX),
        ];

        for (class, symbol) in Symbols::classes(&labels) {
            assert!(class.matches(symbol), "{class:?} does not match {symbol:?}");
        }
    }
}
