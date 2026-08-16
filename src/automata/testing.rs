//! The alphabet that the tests of this module share.
//!
//! The module compiles only under `cfg(test)`. It ships in no build of the crate.
//!
//! An automaton knows no alphabet, thus each test selects one. [`Symbols`] is a range of
//! characters. A character alphabet holds a gap at the values of the surrogates, and it holds more
//! than a million symbols. Thus a test that reads this alphabet catches code in this module that
//! assumes 256 contiguous symbols.

use super::divisible::Divisible;
use super::label::Label;

/// The test alphabet. An automaton knows no alphabet, thus a test selects one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Symbols {
    pub low: char,
    pub high: char,
}

impl Label for Symbols {
    type Symbol = char;

    fn matches(&self, symbol: char) -> bool {
        (self.low..=self.high).contains(&symbol)
    }
}

impl Divisible for Symbols {
    /// Divides `labels` into the ranges between their ends.
    ///
    /// The result is in ascending sequence.
    #[expect(clippy::todo, unused_variables, reason = "step 1 of the plan")]
    fn divide(labels: &[Self]) -> Vec<Self> {
        todo!()
    }

    /// Returns the lowest character in the range.
    ///
    /// # Panics
    ///
    /// This function panics if the range matches no character.
    #[expect(clippy::todo, reason = "step 1 of the plan")]
    fn any_symbol(&self) -> char {
        todo!()
    }
}

/// A label that matches only `symbol`.
pub(super) fn only(symbol: char) -> Symbols {
    Symbols {
        low: symbol,
        high: symbol,
    }
}

/// A label that matches each symbol from `low` to `high`.
pub(super) fn range(low: char, high: char) -> Symbols {
    Symbols { low, high }
}

/// The first character above the values that the surrogates hold.
const ABOVE_GAP: char = '\u{E000}';

/// The last character below the values that the surrogates hold.
const BELOW_GAP: char = '\u{D7FF}';

/// Returns the character after `symbol`, or `None` if `symbol` is the last character.
///
/// The characters leave out the surrogates. Thus the function steps across that gap.
pub(super) fn after(symbol: char) -> Option<char> {
    if symbol == BELOW_GAP {
        return Some(ABOVE_GAP);
    }
    char::from_u32(symbol as u32 + 1)
}

/// Returns the character before `symbol`, or `None` if `symbol` is the first character.
///
/// The characters leave out the surrogates. Thus the function steps across that gap.
pub(super) fn before(symbol: char) -> Option<char> {
    if symbol == ABOVE_GAP {
        return Some(BELOW_GAP);
    }
    char::from_u32((symbol as u32).checked_sub(1)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn divided(labels: &[Symbols]) -> Vec<Symbols> {
        Symbols::divide(labels)
    }

    #[test]
    fn no_label_gives_no_class() {
        assert_eq!(divided(&[]), Vec::new());
    }

    #[test]
    fn one_label_gives_itself() {
        assert_eq!(divided(&[range('a', 'z')]), vec![range('a', 'z')]);
    }

    #[test]
    fn two_labels_that_share_no_symbol_stay_separate() {
        assert_eq!(
            divided(&[range('a', 'c'), range('e', 'f')]),
            vec![range('a', 'c'), range('e', 'f')]
        );
    }

    #[test]
    fn two_labels_that_touch_stay_separate() {
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
    fn a_label_inside_another_label_gives_three_classes() {
        assert_eq!(
            divided(&[range('a', 'z'), range('c', 'e')]),
            vec![range('a', 'b'), range('c', 'e'), range('f', 'z')]
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
                range('c', 'c'),
                range('d', 'f'),
                range('g', 'z'),
            ]
        );
    }

    #[test]
    fn a_label_that_reaches_the_last_character_gives_one_class() {
        assert_eq!(
            divided(&[range('a', char::MAX)]),
            vec![range('a', char::MAX)]
        );
    }

    #[test]
    fn a_class_steps_across_the_gap_of_the_surrogates() {
        assert_eq!(
            divided(&[range(BELOW_GAP, ABOVE_GAP), only(ABOVE_GAP)]),
            vec![only(BELOW_GAP), only(ABOVE_GAP)]
        );
    }

    #[test]
    fn a_label_gives_a_symbol_that_it_matches() {
        assert!(range('a', 'z').matches(range('a', 'z').any_symbol()));
        assert!(only('\0').matches(only('\0').any_symbol()));
        assert!(only(char::MAX).matches(only(char::MAX).any_symbol()));
    }

    #[test]
    fn the_character_after_the_last_character_below_the_gap_is_above_the_gap() {
        assert_eq!(after(BELOW_GAP), Some(ABOVE_GAP));
        assert_eq!(before(ABOVE_GAP), Some(BELOW_GAP));
    }

    #[test]
    fn the_characters_stop_at_both_ends() {
        assert_eq!(after(char::MAX), None);
        assert_eq!(before('\0'), None);
        assert_eq!(after('a'), Some('b'));
        assert_eq!(before('b'), Some('a'));
    }
}
