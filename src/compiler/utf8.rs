//! Lowers a character set to the bytes that encode its characters.
//!
//! An [`Nfa`](crate::automata::nfa::Nfa) reads bytes. A regular expression
//! matches characters. Thus you must lower a
//! [`Class`](crate::regex::Node::Class) leaf to its UTF-8 encodings before
//! Thompson construction makes the states.
//!
//! [`lower`] does that step. It makes an alternation of byte sequences from a
//! [`CharSet`]. Construction reads each byte sequence as a chain of
//! [`Range`](crate::automata::nfa::State::Range) states.
//!
//! This module lowers before determinization, and not after it. Thus the
//! remainder of the pipeline reads only bytes, and the matcher does no
//! decoding. An automaton of this form rejects an overlong encoding, an
//! encoded surrogate, and a truncated character. It rejects them in the same
//! manner as all other incorrect input. It never holds a part of a decoded
//! character.

use crate::regex::CharSet;

/// The maximum number of the bytes that a character encodes to.
const MAX_LENGTH: usize = 4;

/// The number of the payload bits in a continuation byte.
const CONTINUATION_BITS: u32 = 6;

/// The largest character that encodes to one, to two, and to three bytes.
const MAX_BY_LENGTH: [u32; MAX_LENGTH - 1] = [0x7F, 0x7FF, 0xFFFF];

/// A range of bytes that matches one byte of an encoded character.
///
/// Both ends are in the range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteRange {
    /// The lowest byte in the range.
    pub low: u8,
    /// The highest byte in the range.
    pub high: u8,
}

/// The encodings of one range of characters, as one [`ByteRange`] for each
/// byte.
///
/// A sequence of `n` ranges matches only the byte strings of `n` bytes whose
/// byte `i` is in range `i`. Each of those byte strings is the encoding of a
/// character in the range of characters. The two sets are equal, because the
/// [`lower`] function selects the range of characters for that result.
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
        let mut padded = [ByteRange { low: 0, high: 0 }; MAX_LENGTH];
        padded[..ranges.len()].copy_from_slice(ranges);
        Self {
            ranges: padded,
            length: ranges.len(),
        }
    }
}

/// Lowers `set` to the byte sequences that encode its characters.
///
/// The sequences are disjoint and in ascending sequence. Only one sequence
/// matches each character in `set`. The sequences match no other byte string.
///
/// A set that holds no characters lowers to no sequences. That result is the
/// alternation that matches nothing.
pub fn lower(set: &CharSet) -> Vec<ByteSequence> {
    let mut sequences = Vec::new();
    for (low, high) in set.ranges() {
        lower_range(low as u32, high as u32, &mut sequences);
    }
    sequences
}

/// Adds the sequences that encode the characters from `low` to `high` to
/// `out`.
///
/// The function splits the range until the encodings of each part make one
/// sequence. Thus the recursion goes down a maximum of one split for each byte
/// of the encoding.
fn lower_range(low: u32, high: u32, out: &mut Vec<ByteSequence>) {
    if low > high {
        return;
    }

    // Characters of different encoded lengths have no byte string in common.
    // Thus split the range where the encoding gets one more byte. After this
    // loop, both ends encode to the same number of bytes.
    for &max in &MAX_BY_LENGTH {
        if low <= max && max < high {
            lower_range(low, max, out);
            lower_range(max + 1, high, out);
            return;
        }
    }

    let length = encoded_length(low);
    if length == 1 {
        out.push(ByteSequence::new(&[ByteRange {
            low: low as u8,
            high: high as u8,
        }]));
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
            lower_range(low, low | mask, out);
            lower_range((low | mask) + 1, high, out);
            return;
        }
        if high & mask != mask {
            lower_range(low, (high & !mask) - 1, out);
            lower_range(high & !mask, high, out);
            return;
        }
    }

    let low_bytes = encode(low);
    let high_bytes = encode(high);
    let mut ranges = [ByteRange { low: 0, high: 0 }; MAX_LENGTH];
    for (range, (&low_byte, &high_byte)) in ranges.iter_mut().zip(low_bytes.iter().zip(&high_bytes))
    {
        *range = ByteRange {
            low: low_byte,
            high: high_byte,
        };
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
/// # Panics
///
/// This function panics if `codepoint` is a surrogate. It also panics if
/// `codepoint` is above the highest character.
///
/// Neither value comes here. A [`CharSet`] holds neither value, and a split of
/// a range gives only the values that the range already held.
fn encode(codepoint: u32) -> [u8; MAX_LENGTH] {
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

    fn sequence(ranges: &[(u8, u8)]) -> ByteSequence {
        let ranges: Vec<ByteRange> = ranges
            .iter()
            .map(|&(low, high)| ByteRange { low, high })
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
    fn an_empty_set_lowers_to_no_sequences() {
        assert_eq!(lower(&CharSet::empty()), Vec::new());
    }

    #[test]
    fn an_ascii_character_lowers_to_one_byte() {
        assert_eq!(
            lower(&CharSet::single('a')),
            vec![sequence(&[(b'a', b'a')])]
        );
    }

    #[test]
    fn an_ascii_range_lowers_to_one_range() {
        assert_eq!(
            lower(&CharSet::range('a', 'z')),
            vec![sequence(&[(b'a', b'z')])]
        );
    }

    #[test]
    fn a_two_byte_character_lowers_to_its_encoding() {
        assert_eq!(
            lower(&CharSet::single('é')),
            vec![sequence(&[(0xC3, 0xC3), (0xA9, 0xA9)])]
        );
    }

    #[test]
    fn a_three_byte_character_lowers_to_its_encoding() {
        assert_eq!(
            lower(&CharSet::single('☃')),
            vec![sequence(&[(0xE2, 0xE2), (0x98, 0x98), (0x83, 0x83)])]
        );
    }

    #[test]
    fn a_four_byte_character_lowers_to_its_encoding() {
        assert_eq!(
            lower(&CharSet::single('🦀')),
            vec![sequence(&[
                (0xF0, 0xF0),
                (0x9F, 0x9F),
                (0xA6, 0xA6),
                (0x80, 0x80),
            ])]
        );
    }

    #[test]
    fn a_range_of_trailing_bytes_lowers_to_one_sequence() {
        assert_eq!(
            lower(&CharSet::range('\u{80}', '\u{7FF}')),
            vec![sequence(&[(0xC2, 0xDF), (0x80, 0xBF)])]
        );
    }

    #[test]
    fn a_range_crossing_an_encoding_length_splits_there() {
        let sequences = lower(&CharSet::range('\u{7E}', '\u{81}'));
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
            lower(&CharSet::range('\u{100}', '\u{200}')),
            vec![
                sequence(&[(0xC4, 0xC7), (0x80, 0xBF)]),
                sequence(&[(0xC8, 0xC8), (0x80, 0x80)]),
            ]
        );
    }

    #[test]
    fn a_range_starting_mid_block_splits_off_the_partial_block() {
        assert_eq!(
            lower(&CharSet::range('\u{1FF}', '\u{2FF}')),
            vec![
                sequence(&[(0xC7, 0xC7), (0xBF, 0xBF)]),
                sequence(&[(0xC8, 0xCB), (0x80, 0xBF)]),
            ]
        );
    }

    #[test]
    fn every_sequence_holds_one_to_four_non_empty_ranges() {
        let sequences = lower(&CharSet::any());
        for sequence in &sequences {
            assert!((1..=MAX_LENGTH).contains(&sequence.ranges().len()));
            for range in sequence.ranges() {
                assert!(range.low <= range.high, "empty range in {sequence:?}");
            }
        }
    }

    #[test]
    fn every_character_is_matched_by_exactly_one_sequence() {
        let sequences = lower(&CharSet::any());
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
        assert_eq!(matched_strings(&lower(&CharSet::any())), CHARACTERS);
    }

    #[test]
    fn an_overlong_encoding_is_not_matched() {
        let sequences = lower(&CharSet::any());
        assert!(!matches(&sequences, &[0xC0, 0x80]));
        assert!(!matches(&sequences, &[0xC1, 0xBF]));
        assert!(!matches(&sequences, &[0xE0, 0x80, 0x80]));
        assert!(!matches(&sequences, &[0xF0, 0x80, 0x80, 0x80]));
    }

    #[test]
    fn an_encoded_surrogate_is_not_matched() {
        let sequences = lower(&CharSet::any());
        assert!(!matches(&sequences, &[0xED, 0xA0, 0x80]));
        assert!(!matches(&sequences, &[0xED, 0xBF, 0xBF]));
        assert!(matches(&sequences, &[0xED, 0x9F, 0xBF]));
        assert!(matches(&sequences, &[0xEE, 0x80, 0x80]));
    }

    #[test]
    fn a_byte_string_that_is_not_an_encoding_is_not_matched() {
        let sequences = lower(&CharSet::any());
        assert!(!matches(&sequences, &[0x80]));
        assert!(!matches(&sequences, &[0xC2]));
        assert!(!matches(&sequences, &[0xC2, 0xC2]));
        assert!(!matches(&sequences, &[0xF5, 0x80, 0x80, 0x80]));
        assert!(!matches(&sequences, &[0xF4, 0x90, 0x80, 0x80]));
    }

    #[test]
    fn a_hole_in_a_set_is_a_hole_in_its_sequences() {
        let sequences = lower(&CharSet::any().subtract(&CharSet::single('é')));
        assert!(!matches_character(&sequences, 'é'));
        assert!(matches_character(&sequences, 'è'));
        assert!(matches_character(&sequences, 'ê'));
        assert_eq!(matched_strings(&sequences), CHARACTERS - 1);
    }

    #[test]
    fn a_set_of_several_ranges_lowers_to_all_of_them() {
        let sequences = lower(&CharSet::digits().union(&CharSet::range('α', 'ω')));
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
        let sequences = lower(&CharSet::any());
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
