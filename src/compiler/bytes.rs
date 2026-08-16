use super::alphabet::Alphabet;
use super::fragment::Fragment;
use super::utf8::{self, ByteRange, ByteSequence};
use crate::automata::{NfaBuilder, StateId};
use crate::regex::CharSet;

/// The byte alphabet of a lexer that reads UTF-8.
///
/// The alphabet lowers each character set to the byte sequences that encode
/// its characters. Thus the automaton reads bytes, and it does no decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Bytes;

impl Alphabet for Bytes {
    type Label = ByteRange;

    /// Adds one chain of byte transitions for each byte sequence of `set`.
    ///
    /// The function calls [`lower`](super::utf8::lower) to get the sequences.
    /// Each chain starts at the entry of the fragment and stops at its exit.
    /// Thus the chains make an alternation.
    fn lower(&self, set: &CharSet, builder: &mut NfaBuilder<Self::Label>) -> Fragment {
        let entry = builder.push();
        let exit = builder.push();
        let sequences = utf8::lower(set);
        for sequence in sequences {
            chain(sequence, entry, exit, builder);
        }
        Fragment::new(entry, exit)
    }
}

/// Adds a chain of transitions from `entry` to `exit` that matches
/// `sequence`.
///
/// The chain reads one byte for each range of the sequence. The function adds
/// one state between each pair of the ranges. Thus a chain of `n` ranges adds
/// `n - 1` states, and only the last range points at `exit`.
///
/// Each chain of one character set starts at the same entry and stops at the
/// same exit. Two chains that start with the same byte stay separate, because
/// they share no state between the entry and the exit.
fn chain(
    sequence: ByteSequence,
    entry: StateId,
    exit: StateId,
    builder: &mut NfaBuilder<ByteRange>,
) {
    let mut previous = entry;
    let mut iterator = sequence.ranges().iter().peekable();
    while let Some(range) = iterator.next() {
        if iterator.peek().is_some() {
            let next = builder.push();
            builder.transition(previous, *range, next);
            previous = next;
        } else {
            builder.transition(previous, *range, exit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::{Automaton, Execution, NondeterministicFiniteAutomaton, Scanner};

    /// Lowers `set` into an automaton that starts at the entry of the
    /// fragment and accepts at its exit.
    fn lowered(set: &CharSet) -> (NondeterministicFiniteAutomaton<ByteRange>, Fragment) {
        let mut builder: NfaBuilder<ByteRange> = NfaBuilder::new();
        let fragment = Bytes.lower(set, &mut builder);
        builder.accept(fragment.exit());
        let nfa = builder
            .build(&[fragment.entry()])
            .expect("a test stays below the capacity of a builder");
        (nfa, fragment)
    }

    /// Returns the number of the bytes that `set` matches at the start of
    /// `input`.
    fn matched(set: &CharSet, input: &[u8]) -> Option<usize> {
        let (nfa, _) = lowered(set);
        let start = 0;
        let mut execution = nfa.execute();
        execution
            .longest_match(start, input, |_| ())
            .map(|found| found.length)
    }

    /// Returns the number of the transitions in the whole automaton.
    fn transition_count(nfa: &NondeterministicFiniteAutomaton<ByteRange>) -> usize {
        (0..nfa.state_count())
            .map(|index| nfa.transitions(StateId::new(index)).len())
            .sum()
    }

    #[test]
    fn a_one_byte_character_gives_one_transition() {
        let set = CharSet::single('a');
        let (nfa, _) = lowered(&set);

        assert_eq!(nfa.state_count(), 2);
        assert_eq!(transition_count(&nfa), 1);
        assert_eq!(matched(&set, b"a"), Some(1));
        assert_eq!(matched(&set, b"b"), None);
    }

    #[test]
    fn a_two_byte_character_gives_a_chain_of_two() {
        let set = CharSet::single('é');
        let (nfa, _) = lowered(&set);

        assert_eq!(nfa.state_count(), 3);
        assert_eq!(transition_count(&nfa), 2);
        assert_eq!(matched(&set, "é".as_bytes()), Some(2));
    }

    #[test]
    fn a_chain_rejects_the_first_byte_alone() {
        let set = CharSet::single('é');

        assert_eq!(matched(&set, &[0xC3]), None);
        assert_eq!(matched(&set, &[0xC3, 0xC3]), None);
        assert_eq!(matched(&set, &[0xA9]), None);
    }

    #[test]
    fn an_empty_set_matches_nothing() {
        let set = CharSet::empty();
        let (nfa, _) = lowered(&set);

        assert_eq!(nfa.state_count(), 2);
        assert_eq!(transition_count(&nfa), 0);
        assert_eq!(matched(&set, b""), None);
        assert_eq!(matched(&set, b"a"), None);
    }

    #[test]
    fn a_range_that_crosses_an_encoding_length_gives_one_chain_for_each_length() {
        let set = CharSet::range('\u{7F}', '\u{80}');
        let (nfa, _) = lowered(&set);

        assert_eq!(nfa.state_count(), 3);
        assert_eq!(transition_count(&nfa), 3);
        assert_eq!(matched(&set, &[0x7F]), Some(1));
        assert_eq!(matched(&set, &[0xC2, 0x80]), Some(2));
    }

    #[test]
    fn each_chain_starts_at_the_entry_and_stops_at_the_exit() {
        let set = CharSet::single('a').union(&CharSet::single('c'));
        let (nfa, fragment) = lowered(&set);
        let targets: Vec<_> = nfa
            .transitions(fragment.entry())
            .iter()
            .map(|transition| transition.target)
            .collect();

        assert_eq!(targets, vec![fragment.exit(), fragment.exit()]);
        assert_eq!(matched(&set, b"a"), Some(1));
        assert_eq!(matched(&set, b"c"), Some(1));
        assert_eq!(matched(&set, b"b"), None);
    }

    #[test]
    fn two_encodings_that_start_with_the_same_byte_stay_separate() {
        let set = CharSet::single('\u{A9}').union(&CharSet::single('\u{AB}'));
        let (nfa, _) = lowered(&set);

        assert_eq!(nfa.state_count(), 4);
        assert_eq!(matched(&set, &[0xC2, 0xA9]), Some(2));
        assert_eq!(matched(&set, &[0xC2, 0xAB]), Some(2));
        assert_eq!(matched(&set, &[0xC2, 0xAA]), None);
    }

    #[test]
    fn the_set_of_each_character_matches_a_character_of_each_length() {
        let set = CharSet::any();

        assert_eq!(matched(&set, b"a"), Some(1));
        assert_eq!(matched(&set, "é".as_bytes()), Some(2));
        assert_eq!(matched(&set, "€".as_bytes()), Some(3));
        assert_eq!(matched(&set, "🦀".as_bytes()), Some(4));
        assert_eq!(matched(&set, &[0x80]), None);
        assert_eq!(matched(&set, &[0xC3]), None);
    }

    #[test]
    fn the_fragment_keeps_its_entry_and_its_exit_clear() {
        let (nfa, fragment) = lowered(&CharSet::range('a', 'z'));

        assert!(nfa.transitions(fragment.exit()).is_empty());
        assert!(nfa.epsilons(fragment.entry()).is_empty());
        assert!(!nfa.accepts(fragment.entry()));
    }
}
