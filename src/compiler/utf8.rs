//! Lowering of character sets to the bytes that encode them.
//!
//! An [`Nfa`](crate::automata::nfa::Nfa) reads bytes, but a regular expression
//! matches characters, so a [`Class`](crate::regex::Node::Class) leaf has to be
//! rewritten in terms of UTF-8 encodings before Thompson construction can turn
//! it into states. [`lower`] does that rewrite: it turns a [`CharSet`] into an
//! alternation of byte sequences, each of which construction reads as a chain
//! of [`Range`](crate::automata::nfa::State::Range) states.
//!
//! Lowering here rather than after determinization keeps the rest of the
//! pipeline byte-oriented, and it keeps the decoding out of the matcher: an
//! automaton built this way rejects an overlong encoding, an encoded surrogate,
//! and a truncated character the same way it rejects any other input, without
//! ever holding a partially decoded character.

use crate::regex::CharSet;

/// The largest number of bytes a character encodes to.
const MAX_LENGTH: usize = 4;

/// The number of payload bits a continuation byte carries.
const CONTINUATION_BITS: u32 = 6;

/// The largest character that encodes to one, to two, and to three bytes.
const MAX_BY_LENGTH: [u32; MAX_LENGTH - 1] = [0x7F, 0x7FF, 0xFFFF];

/// An inclusive range of bytes, matching one byte of an encoded character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteRange {
    pub low: u8,
    pub high: u8,
}

/// The encodings of one run of characters, as one [`ByteRange`] per byte.
///
/// A sequence of `n` ranges matches exactly those `n` byte strings whose `i`-th
/// byte lies in the `i`-th range, and every one of them is the encoding of a
/// character in the run — the run is chosen so that the two say the same thing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteSequence {
    ranges: [ByteRange; MAX_LENGTH],
    length: usize,
}

impl ByteSequence {
    /// Returns one byte range per byte of the encoding, first byte first.
    pub fn ranges(&self) -> &[ByteRange] {
        &self.ranges[..self.length]
    }

    /// Creates a `ByteSequence` from one byte range per byte of the encoding.
    ///
    /// # Panics
    ///
    /// Panics if `ranges` is empty or holds more than [`MAX_LENGTH`] ranges.
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
/// The sequences are disjoint and ascending: every character in `set` is
/// matched by exactly one of them, and nothing else is matched by any of them.
/// A set holding no characters lowers to no sequences, which is the alternation
/// that matches nothing.
pub fn lower(set: &CharSet) -> Vec<ByteSequence> {
    let mut sequences = Vec::new();
    for (low, high) in set.ranges() {
        lower_range(low as u32, high as u32, &mut sequences);
    }
    sequences
}

/// Appends the sequences encoding the characters from `low` to `high` to `out`.
///
/// The range is split until each piece is one whose encodings form a sequence,
/// so the recursion is at most one split per byte of the encoding deep.
fn lower_range(low: u32, high: u32, out: &mut Vec<ByteSequence>) {
    if low > high {
        return;
    }

    // Characters of different encoded lengths share no byte string, so split
    // wherever the encoding grows a byte. Past this point both ends encode to
    // the same number of bytes.
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

    // A sequence lets every byte vary over its own range independently, which
    // only says the same thing as the character range if the trailing bytes run
    // over all of their values. `mask` covers the payload bits of the last
    // `trailing` bytes, so the range qualifies when it starts with those bits
    // clear and ends with them set. Where it does not, split at the first value
    // that does: a low end that starts mid-block is cut off at the end of its
    // block, and a high end that stops mid-block is cut off at the start of
    // its own.
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

/// Returns the number of bytes `codepoint` encodes to.
fn encoded_length(codepoint: u32) -> usize {
    MAX_BY_LENGTH
        .iter()
        .position(|&max| codepoint <= max)
        .map_or(MAX_LENGTH, |index| index + 1)
}

/// Returns the encoding of `codepoint`, padded with zeroes.
///
/// # Panics
///
/// Panics if `codepoint` is a surrogate or above the highest character. Neither
/// reaches here: a [`CharSet`] holds neither, and splitting a range only ever
/// yields values it already held.
fn encode(codepoint: u32) -> [u8; MAX_LENGTH] {
    let character = char::from_u32(codepoint).expect("a character range holds only characters");
    let mut bytes = [0; MAX_LENGTH];
    character.encode_utf8(&mut bytes);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The number of characters, which is every codepoint but the surrogates.
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

    /// Returns the number of byte strings `sequences` matches.
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
        // U+0100 to U+01FF fills the trailing byte of every leading byte it
        // reaches, but U+0200 leaves it at its first value.
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
        // Every character is matched exactly once, so matching no more strings
        // than there are characters leaves room for nothing else: no overlong
        // encoding, no encoded surrogate, no truncated or stray byte.
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
