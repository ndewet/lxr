//! Lowers character sets into UTF-8 byte sequences.
//!
//! Lowering occurs before determinization, so later stages process only bytes.
//! The sequences exclude overlong encodings, surrogates, and truncated
//! characters.

use super::Encoding;
use crate::automata::Label;
use crate::automata::label::LabelClass;
use crate::automata::label::Partitionable;
use crate::regex::CharSet;

const MAX_LENGTH: usize = 4;

const CONTINUATION_BITS: u32 = 6;

const MAX_BY_LENGTH: [u32; MAX_LENGTH - 1] = [0x7F, 0x7FF, 0xFFFF];

/// Lowers character sets for an automaton that reads UTF-8 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Utf8;

impl Encoding for Utf8 {
    type Label = ByteRange;
    type Sequence = ByteSequence;

    fn encode(&self, set: &CharSet) -> Vec<Self::Sequence> {
        encode(set)
    }
}

/// Matches one inclusive range of bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ByteRange {
    /// The inclusive lower bound.
    pub(crate) low: u8,
    /// The inclusive upper bound.
    pub(crate) high: u8,
}

impl ByteRange {
    /// Creates an inclusive range of bytes.
    ///
    /// # Panics
    ///
    /// This function panics if `low` is above `high`.
    pub(crate) fn new(low: u8, high: u8) -> Self {
        assert!(low <= high, "a byte range cannot start above its end");
        Self { low, high }
    }
}

impl Label for ByteRange {
    type Symbol = u8;

    fn matches(&self, byte: u8) -> bool {
        (self.low..=self.high).contains(&byte)
    }
}

impl Partitionable for ByteRange {
    fn partition(labels: &[Self]) -> Vec<LabelClass<Self>> {
        let mut boundaries: Vec<u16> = Vec::new();
        for label in labels {
            boundaries.push(label.low as u16);
            boundaries.push(label.high as u16 + 1);
        }
        boundaries.sort_unstable();
        boundaries.dedup();
        let mut classes: Vec<LabelClass<Self>> = Vec::new();
        for pair in boundaries.windows(2) {
            let low = pair[0] as u8;
            let high = (pair[1] - 1) as u8;
            let matching_labels: Vec<usize> = labels
                .iter()
                .enumerate()
                .filter_map(|(index, label)| label.matches(low).then_some(index))
                .collect();
            if matching_labels.is_empty() {
                continue;
            }
            let label = ByteRange::new(low, high);
            let class = LabelClass::new(label, matching_labels);
            classes.push(class);
        }
        classes
    }
}

/// Matches one set of equal-length UTF-8 encodings.
///
/// Each position has one [`ByteRange`]. Their product contains only valid
/// encodings from the source character set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ByteSequence {
    ranges: [ByteRange; MAX_LENGTH],
    length: usize,
}

impl ByteSequence {
    /// Returns the byte ranges in input sequence.
    pub(crate) fn ranges(&self) -> &[ByteRange] {
        &self.ranges[..self.length]
    }

    fn new(ranges: &[ByteRange]) -> Self {
        assert!(
            (1..=MAX_LENGTH).contains(&ranges.len()),
            "a character encodes to 1 to {MAX_LENGTH} bytes, not {}",
            ranges.len()
        );
        let mut padded = [ByteRange::new(0, 0); MAX_LENGTH];
        padded[..ranges.len()].copy_from_slice(ranges);
        Self {
            ranges: padded,
            length: ranges.len(),
        }
    }
}

impl AsRef<[ByteRange]> for ByteSequence {
    fn as_ref(&self) -> &[ByteRange] {
        self.ranges()
    }
}

/// Lowers `set` into disjoint UTF-8 byte sequences.
///
/// The sequences match exactly the characters in `set`. An empty set produces
/// no sequence.
pub(crate) fn encode(set: &CharSet) -> Vec<ByteSequence> {
    let mut sequences = Vec::new();
    for (low, high) in set.ranges() {
        encode_range(low as u32, high as u32, &mut sequences);
    }
    sequences
}

fn encode_range(low: u32, high: u32, out: &mut Vec<ByteSequence>) {
    if low > high {
        return;
    }

    // Encodings with different lengths are disjoint. Split the range where
    // its encoded length changes.
    for &max in &MAX_BY_LENGTH {
        if low <= max && max < high {
            encode_range(low, max, out);
            encode_range(max + 1, high, out);
            return;
        }
    }

    let length = encoded_length(low);
    if length == 1 {
        out.push(ByteSequence::new(&[ByteRange::new(low as u8, high as u8)]));
        return;
    }

    // A sequence forms the product of its byte ranges. Split partial trailing
    // byte blocks so that the product matches the character range exactly.
    for trailing in 1..length as u32 {
        let mask = (1 << (CONTINUATION_BITS * trailing)) - 1;
        if low & !mask == high & !mask {
            continue;
        }
        if low & mask != 0 {
            encode_range(low, low | mask, out);
            encode_range((low | mask) + 1, high, out);
            return;
        }
        if high & mask != mask {
            encode_range(low, (high & !mask) - 1, out);
            encode_range(high & !mask, high, out);
            return;
        }
    }

    let low_bytes = encode_scalar(low);
    let high_bytes = encode_scalar(high);
    let mut ranges = [ByteRange::new(0, 0); MAX_LENGTH];
    for (range, (&low_byte, &high_byte)) in ranges.iter_mut().zip(low_bytes.iter().zip(&high_bytes))
    {
        *range = ByteRange::new(low_byte, high_byte);
    }
    out.push(ByteSequence::new(&ranges[..length]));
}

fn encoded_length(codepoint: u32) -> usize {
    MAX_BY_LENGTH
        .iter()
        .position(|&max| codepoint <= max)
        .map_or(MAX_LENGTH, |index| index + 1)
}

fn encode_scalar(codepoint: u32) -> [u8; MAX_LENGTH] {
    let character = char::from_u32(codepoint).expect("a character range holds only characters");
    let mut bytes = [0; MAX_LENGTH];
    character.encode_utf8(&mut bytes);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHARACTERS: u64 = 0x11_0000 - 0x800;

    fn byte_range(low: u8, high: u8) -> ByteRange {
        ByteRange::new(low, high)
    }

    fn assert_partition(labels: &[ByteRange], expected: &[(ByteRange, &[usize])]) {
        let actual = ByteRange::partition(labels);

        assert_eq!(actual.len(), expected.len(), "partition of {labels:?}");
        for (class, (label, matching_labels)) in actual.iter().zip(expected) {
            assert_eq!(class.get_label(), label, "partition of {labels:?}");
            assert_eq!(
                class.get_matching_labels(),
                matching_labels,
                "partition of {labels:?}"
            );
        }
    }

    #[test]
    fn partitioning_no_byte_ranges_produces_no_classes() {
        assert_partition(&[], &[]);
    }

    #[test]
    fn partitioning_one_byte_range_preserves_it() {
        assert_partition(&[byte_range(b'a', b'z')], &[(byte_range(b'a', b'z'), &[0])]);
    }

    #[test]
    fn partitioning_disjoint_byte_ranges_omits_the_gap() {
        assert_partition(
            &[byte_range(b'a', b'f'), byte_range(b'm', b'z')],
            &[
                (byte_range(b'a', b'f'), &[0]),
                (byte_range(b'm', b'z'), &[1]),
            ],
        );
    }

    #[test]
    fn partitioning_overlapping_byte_ranges_splits_at_each_boundary() {
        assert_partition(
            &[byte_range(b'a', b'f'), byte_range(b'd', b'z')],
            &[
                (byte_range(b'a', b'c'), &[0]),
                (byte_range(b'd', b'f'), &[0, 1]),
                (byte_range(b'g', b'z'), &[1]),
            ],
        );
    }

    #[test]
    fn partition_indexes_select_the_matching_transition_targets() {
        let transitions = [
            (byte_range(b'a', b'f'), "first target"),
            (byte_range(b'd', b'z'), "second target"),
        ];
        let labels: Vec<_> = transitions.iter().map(|(label, _)| *label).collect();

        let classes = ByteRange::partition(&labels);
        let overlap = &classes[1];
        let targets: Vec<_> = overlap
            .get_matching_labels()
            .iter()
            .map(|&index| transitions[index].1)
            .collect();

        assert_eq!(overlap.get_label(), &byte_range(b'd', b'f'));
        assert_eq!(targets, vec!["first target", "second target"]);
    }

    #[test]
    fn partitioning_a_contained_byte_range_splits_the_containing_range() {
        assert_partition(
            &[byte_range(b'a', b'z'), byte_range(b'd', b'f')],
            &[
                (byte_range(b'a', b'c'), &[0]),
                (byte_range(b'd', b'f'), &[0, 1]),
                (byte_range(b'g', b'z'), &[0]),
            ],
        );
    }

    #[test]
    fn partitioning_equal_byte_ranges_records_both_indexes() {
        assert_partition(
            &[byte_range(b'a', b'z'), byte_range(b'a', b'z')],
            &[(byte_range(b'a', b'z'), &[0, 1])],
        );
    }

    #[test]
    fn partitioning_adjacent_byte_ranges_keeps_their_classes_separate() {
        assert_partition(
            &[byte_range(b'a', b'm'), byte_range(b'n', b'z')],
            &[
                (byte_range(b'a', b'm'), &[0]),
                (byte_range(b'n', b'z'), &[1]),
            ],
        );
    }

    #[test]
    fn partitioning_a_byte_range_can_include_the_largest_byte() {
        assert_partition(
            &[byte_range(254, 255), byte_range(255, 255)],
            &[
                (byte_range(254, 254), &[0]),
                (byte_range(255, 255), &[0, 1]),
            ],
        );
    }

    #[test]
    fn every_pair_of_small_byte_ranges_obeys_the_partition_contract() {
        const ALPHABET_END: u8 = 15;

        let ranges: Vec<_> = (0..=ALPHABET_END)
            .flat_map(|low| (low..=ALPHABET_END).map(move |high| byte_range(low, high)))
            .collect();

        for &first in &ranges {
            for &second in &ranges {
                assert_partition_contract(&[first, second], ALPHABET_END);
            }
        }
    }

    fn assert_partition_contract(labels: &[ByteRange], alphabet_end: u8) {
        let classes = ByteRange::partition(labels);

        for class in &classes {
            let label = class.get_label();
            assert!(
                label.low <= label.high,
                "empty class in partition of {labels:?}"
            );
            assert!(
                label.high <= alphabet_end,
                "class outside the alphabet in partition of {labels:?}"
            );
            assert!(
                !class.get_matching_labels().is_empty(),
                "class without matching labels in partition of {labels:?}"
            );
        }

        for pair in classes.windows(2) {
            assert!(
                pair[0].get_label().high < pair[1].get_label().low,
                "unordered or overlapping classes in partition of {labels:?}"
            );
        }

        for byte in 0..=alphabet_end {
            let expected: Vec<_> = labels
                .iter()
                .enumerate()
                .filter_map(|(index, label)| label.matches(byte).then_some(index))
                .collect();
            let matching_classes: Vec<_> = classes
                .iter()
                .filter(|class| class.get_label().matches(byte))
                .collect();

            if expected.is_empty() {
                assert!(
                    matching_classes.is_empty(),
                    "uncovered byte {byte} has a class in partition of {labels:?}"
                );
            } else {
                assert_eq!(
                    matching_classes.len(),
                    1,
                    "covered byte {byte} is not in exactly one class for {labels:?}"
                );
                assert_eq!(
                    matching_classes[0].get_matching_labels(),
                    &expected,
                    "wrong matching labels for byte {byte} in partition of {labels:?}"
                );
            }
        }
    }

    #[test]
    fn a_byte_range_matches_the_bytes_from_its_low_bound_to_its_high_bound() {
        let range = ByteRange::new(b'a', b'z');

        assert!(range.matches(b'a'));
        assert!(range.matches(b'm'));
        assert!(range.matches(b'z'));
        assert!(!range.matches(b'A'));
        assert!(!range.matches(b'{'));
    }

    #[test]
    fn a_byte_range_of_one_byte_matches_only_that_byte() {
        let range = ByteRange::new(0x80, 0x80);

        assert!(range.matches(0x80));
        assert!(!range.matches(0x7F));
        assert!(!range.matches(0x81));
    }

    #[test]
    #[should_panic(expected = "a byte range cannot start above its end")]
    fn a_byte_range_cannot_start_above_its_end() {
        ByteRange::new(b'z', b'a');
    }

    fn sequence(ranges: &[(u8, u8)]) -> ByteSequence {
        let ranges: Vec<ByteRange> = ranges
            .iter()
            .map(|&(low, high)| ByteRange::new(low, high))
            .collect();
        ByteSequence::new(&ranges)
    }

    fn matching(sequences: &[ByteSequence], bytes: &[u8]) -> usize {
        sequences
            .iter()
            .filter(|sequence| {
                sequence.ranges().len() == bytes.len()
                    && sequence
                        .ranges()
                        .iter()
                        .zip(bytes)
                        .all(|(range, byte)| (range.low..=range.high).contains(byte))
            })
            .count()
    }

    fn matches(sequences: &[ByteSequence], bytes: &[u8]) -> bool {
        matching(sequences, bytes) > 0
    }

    fn matches_character(sequences: &[ByteSequence], character: char) -> bool {
        matches(sequences, character.to_string().as_bytes())
    }

    fn matched_strings(sequences: &[ByteSequence]) -> u64 {
        sequences
            .iter()
            .map(|sequence| {
                sequence
                    .ranges()
                    .iter()
                    .map(|range| u64::from(range.high) - u64::from(range.low) + 1)
                    .product::<u64>()
            })
            .sum()
    }

    #[test]
    fn an_empty_set_encodes_to_no_sequences() {
        assert_eq!(encode(&CharSet::empty()), Vec::new());
    }

    #[test]
    fn an_ascii_character_encodes_to_one_byte() {
        assert_eq!(
            encode(&CharSet::single('a')),
            vec![sequence(&[(b'a', b'a')])]
        );
    }

    #[test]
    fn an_ascii_range_encodes_to_one_range() {
        assert_eq!(
            encode(&CharSet::range('a', 'z')),
            vec![sequence(&[(b'a', b'z')])]
        );
    }

    #[test]
    fn a_two_byte_character_encodes_to_its_encoding() {
        assert_eq!(
            encode(&CharSet::single('é')),
            vec![sequence(&[(0xC3, 0xC3), (0xA9, 0xA9)])]
        );
    }

    #[test]
    fn a_three_byte_character_encodes_to_its_encoding() {
        assert_eq!(
            encode(&CharSet::single('☃')),
            vec![sequence(&[(0xE2, 0xE2), (0x98, 0x98), (0x83, 0x83)])]
        );
    }

    #[test]
    fn a_four_byte_character_encodes_to_its_encoding() {
        assert_eq!(
            encode(&CharSet::single('🦀')),
            vec![sequence(&[
                (0xF0, 0xF0),
                (0x9F, 0x9F),
                (0xA6, 0xA6),
                (0x80, 0x80),
            ])]
        );
    }

    #[test]
    fn a_range_of_trailing_bytes_encodes_to_one_sequence() {
        assert_eq!(
            encode(&CharSet::range('\u{80}', '\u{7FF}')),
            vec![sequence(&[(0xC2, 0xDF), (0x80, 0xBF)])]
        );
    }

    #[test]
    fn a_range_crossing_an_encoding_length_splits_there() {
        let sequences = encode(&CharSet::range('\u{7E}', '\u{81}'));
        assert_eq!(
            sequences,
            vec![
                sequence(&[(0x7E, 0x7F)]),
                sequence(&[(0xC2, 0xC2), (0x80, 0x81)]),
            ]
        );
    }

    #[test]
    fn a_range_ending_mid_block_splits_off_the_partial_block() {
        assert_eq!(
            encode(&CharSet::range('\u{100}', '\u{200}')),
            vec![
                sequence(&[(0xC4, 0xC7), (0x80, 0xBF)]),
                sequence(&[(0xC8, 0xC8), (0x80, 0x80)]),
            ]
        );
    }

    #[test]
    fn a_range_starting_mid_block_splits_off_the_partial_block() {
        assert_eq!(
            encode(&CharSet::range('\u{1FF}', '\u{2FF}')),
            vec![
                sequence(&[(0xC7, 0xC7), (0xBF, 0xBF)]),
                sequence(&[(0xC8, 0xCB), (0x80, 0xBF)]),
            ]
        );
    }

    #[test]
    fn every_sequence_holds_one_to_four_non_empty_ranges() {
        let sequences = encode(&CharSet::any());
        for sequence in &sequences {
            assert!((1..=MAX_LENGTH).contains(&sequence.ranges().len()));
            for range in sequence.ranges() {
                assert!(range.low <= range.high, "empty range in {sequence:?}");
            }
        }
    }

    #[test]
    fn every_character_is_matched_by_exactly_one_sequence() {
        let sequences = encode(&CharSet::any());
        let mut buffer = [0; MAX_LENGTH];
        for character in (0..=0x10FFFF).filter_map(char::from_u32) {
            let encoded = character.encode_utf8(&mut buffer);
            assert_eq!(
                matching(&sequences, encoded.as_bytes()),
                1,
                "{character:?} is not matched exactly once"
            );
        }
    }

    #[test]
    fn nothing_but_a_character_is_matched() {
        // Equal counts prove that the sequences match no extra byte string.
        assert_eq!(matched_strings(&encode(&CharSet::any())), CHARACTERS);
    }

    #[test]
    fn an_overlong_encoding_is_not_matched() {
        let sequences = encode(&CharSet::any());
        assert!(!matches(&sequences, &[0xC0, 0x80]));
        assert!(!matches(&sequences, &[0xC1, 0xBF]));
        assert!(!matches(&sequences, &[0xE0, 0x80, 0x80]));
        assert!(!matches(&sequences, &[0xF0, 0x80, 0x80, 0x80]));
    }

    #[test]
    fn an_encoded_surrogate_is_not_matched() {
        let sequences = encode(&CharSet::any());
        assert!(!matches(&sequences, &[0xED, 0xA0, 0x80]));
        assert!(!matches(&sequences, &[0xED, 0xBF, 0xBF]));
        assert!(matches(&sequences, &[0xED, 0x9F, 0xBF]));
        assert!(matches(&sequences, &[0xEE, 0x80, 0x80]));
    }

    #[test]
    fn a_byte_string_that_is_not_an_encoding_is_not_matched() {
        let sequences = encode(&CharSet::any());
        assert!(!matches(&sequences, &[0x80]));
        assert!(!matches(&sequences, &[0xC2]));
        assert!(!matches(&sequences, &[0xC2, 0xC2]));
        assert!(!matches(&sequences, &[0xF5, 0x80, 0x80, 0x80]));
        assert!(!matches(&sequences, &[0xF4, 0x90, 0x80, 0x80]));
    }

    #[test]
    fn a_hole_in_a_set_is_a_hole_in_its_sequences() {
        let sequences = encode(&CharSet::any().subtract(&CharSet::single('é')));
        assert!(!matches_character(&sequences, 'é'));
        assert!(matches_character(&sequences, 'è'));
        assert!(matches_character(&sequences, 'ê'));
        assert_eq!(matched_strings(&sequences), CHARACTERS - 1);
    }

    #[test]
    fn a_set_of_several_ranges_encodes_to_all_of_them() {
        let sequences = encode(&CharSet::digits().union(&CharSet::range('α', 'ω')));
        assert!(matches_character(&sequences, '0'));
        assert!(matches_character(&sequences, '9'));
        assert!(matches_character(&sequences, 'α'));
        assert!(matches_character(&sequences, 'ω'));
        assert!(!matches_character(&sequences, 'a'));
        assert!(!matches_character(&sequences, 'Ω'));
        assert_eq!(matched_strings(&sequences), 10 + 25);
    }

    #[test]
    fn the_sequences_of_a_set_are_ascending() {
        let sequences = encode(&CharSet::any());
        for pair in sequences.windows(2) {
            let (earlier, later) = (pair[0].ranges(), pair[1].ranges());
            assert!(
                earlier.len() < later.len() || earlier[0].high < later[0].low,
                "{:?} does not come before {:?}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    #[should_panic(expected = "a character encodes to 1 to 4 bytes, not 0")]
    fn a_sequence_of_no_ranges_panics() {
        sequence(&[]);
    }

    #[test]
    #[should_panic(expected = "a character encodes to 1 to 4 bytes, not 5")]
    fn a_sequence_of_more_ranges_than_a_character_has_bytes_panics() {
        sequence(&[(0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]);
    }
}
