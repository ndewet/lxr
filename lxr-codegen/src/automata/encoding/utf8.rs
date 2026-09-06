//! Represents a character set as UTF-8 byte sequences.
//!
//! The lexer reads bytes. A regular expression matches characters. Thus you
//! must encode a [`Class`](crate::regex::Expression::Class) leaf as UTF-8
//! encodings before Thompson construction makes the states.
//!
//! [`encode`] does that step. It makes an alternation of byte sequences from a
//! [`CharSet`]. Construction reads each byte sequence as a chain of
//! [`Transition`](crate::automata::Transition)s. A [`ByteRange`] is the label
//! of one of them.
//!
//! This module lowers before determinization, and not after it. Thus the
//! remainder of the pipeline reads only bytes, and the matcher does no
//! decoding. An automaton of this form rejects an overlong encoding, an
//! encoded surrogate, and a truncated character. It rejects them in the same
//! manner as all other incorrect input. It never holds a part of a decoded
//! character.

use super::Encoding;
use crate::automata::Label;
use crate::automata::label::LabelClass;
use crate::automata::label::Partitionable;
use crate::regex::CharSet;

/// The maximum number of the bytes that a character encodes to.
const MAX_LENGTH: usize = 4;

/// The number of the payload bits in a continuation byte.
const CONTINUATION_BITS: u32 = 6;

/// The largest character that encodes to one, to two, and to three bytes.
const MAX_BY_LENGTH: [u32; MAX_LENGTH - 1] = [0x7F, 0x7FF, 0xFFFF];

/// UTF-8 encoding for an automaton that reads bytes.
///
/// The encoding maps each character set to the byte sequences for its
/// characters. The automaton can then read bytes without decoding them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Utf8;

impl Encoding for Utf8 {
    type Label = ByteRange;
    type Sequence = ByteSequence;

    fn encode(&self, set: &CharSet) -> Vec<Self::Sequence> {
        encode(set)
    }
}

/// A range of bytes that matches one byte of an encoded character.
///
/// Both ends are in the range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ByteRange {
    /// The lowest byte in the range.
    pub low: u8,
    /// The highest byte in the range.
    pub high: u8,
}

impl ByteRange {
    /// Creates an inclusive range of bytes.
    ///
    /// # Panics
    ///
    /// This function panics if `low` is above `high`.
    pub fn new(low: u8, high: u8) -> Self {
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

/// The encodings of one range of characters, as one [`ByteRange`] for each
/// byte.
///
/// A sequence of `n` ranges matches only the byte strings of `n` bytes whose
/// byte `i` is in range `i`. Each of those byte strings is the encoding of a
/// character in the range of characters. The two sets are equal, because the
/// [`encode`] function selects the range of characters for that result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteSequence {
    ranges: [ByteRange; MAX_LENGTH],
    length: usize,
}

impl ByteSequence {
    /// Returns one byte range for each byte of the encoding, the first byte
    /// first.
    pub fn ranges(&self) -> &[ByteRange] {
        &self.ranges[..self.length]
    }

    /// Creates a `ByteSequence` from one byte range for each byte of the
    /// encoding.
    ///
    /// # Panics
    ///
    /// This function panics if `ranges` is empty. It also panics if `ranges`
    /// holds more than [`MAX_LENGTH`] ranges.
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

/// Encodes `set` to the byte sequences that encode its characters.
///
/// The sequences are disjoint and in ascending sequence. Only one sequence
/// matches each character in `set`. The sequences match no other byte string.
///
/// A set that holds no characters encodes to no sequences. That result is the
/// alternation that matches nothing.
pub fn encode(set: &CharSet) -> Vec<ByteSequence> {
    let mut sequences = Vec::new();
    for (low, high) in set.ranges() {
        encode_range(low as u32, high as u32, &mut sequences);
    }
    sequences
}

/// Adds the sequences that encode the characters from `low` to `high` to
/// `out`.
///
/// The function splits the range until the encodings of each part make one
/// sequence. Thus the recursion goes down a maximum of one split for each byte
/// of the encoding.
fn encode_range(low: u32, high: u32, out: &mut Vec<ByteSequence>) {
    if low > high {
        return;
    }

    // Characters of different encoded lengths have no byte string in common.
    // Thus split the range where the encoding gets one more byte. After this
    // loop, both ends encode to the same number of bytes.
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

    // In a sequence, each byte moves through its own range independently. Thus
    // a sequence is equal to the character range only if the trailing bytes
    // move through all of their values. `mask` covers the payload bits of the
    // last `trailing` bytes. The range obeys this condition if it starts with
    // those bits clear and ends with those bits set. If the range does not
    // obey the condition, split it at the first value that does. Cut a low end
    // that starts in the middle of a block at the end of that block. Cut a
    // high end that stops in the middle of a block at the start of that block.
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

/// Returns the number of the bytes that `codepoint` encodes to.
fn encoded_length(codepoint: u32) -> usize {
    MAX_BY_LENGTH
        .iter()
        .position(|&max| codepoint <= max)
        .map_or(MAX_LENGTH, |index| index + 1)
}

/// Returns the encoding of `codepoint`, with zeroes after the last byte.
///
/// `codepoint` is always a character, because a [`CharSet`] holds only
/// characters and a split gives only the values that the range held.
fn encode_scalar(codepoint: u32) -> [u8; MAX_LENGTH] {
    let character = char::from_u32(codepoint).expect("a character range holds only characters");
    let mut bytes = [0; MAX_LENGTH];
    character.encode_utf8(&mut bytes);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The number of the characters. This is each codepoint but the
    /// surrogates.
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

    /// Returns the number of the byte strings that `sequences` matches.
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
        // U+0100 to U+01FF fills the trailing byte of each of its leading
        // bytes. U+0200 leaves the trailing byte at its first value.
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
        // The sequences match each character one time. Thus, if they match no
        // more strings than the number of the characters, they match nothing
        // else. They match no overlong encoding, no encoded surrogate, no
        // truncated character, and no unwanted byte.
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
