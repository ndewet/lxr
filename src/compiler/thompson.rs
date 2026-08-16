//! Thompson construction: makes the states of one pattern.
//!
//! [`fragment`] walks a [`Node`] tree and makes one [`Fragment`] for each
//! node. A fragment of an operator holds the fragments of its operands. Thus
//! the number of the states grows with the size of the pattern, and not with
//! the size of the alphabet.
//!
//! The construction knows no alphabet. It gives each
//! [`Class`](Node::Class) leaf to an [`Alphabet`].
//!
//! The construction makes no start state and no accept.
//! [`compile`](super::compile) owns the builder. It joins each pattern to the
//! start states of the rule.

use super::alphabet::Alphabet;
use super::fragment::Fragment;
use crate::automata::NfaBuilder;
use crate::regex::{Node, Repetitions};

/// Adds the states of `node` to `builder`, then returns them as a fragment.
///
/// The function lowers each character set with `alphabet`.
///
/// The fragment has no accept. The caller makes the exit accept, or joins the
/// fragment to another fragment.
pub fn fragment<A: Alphabet>(
    node: &Node,
    alphabet: &A,
    builder: &mut NfaBuilder<A::Label>,
) -> Fragment {
    match node {
        Node::Epsilon => epsilon(builder),
        Node::Class(set) => alphabet.lower(set, builder),
        Node::Concatenation(parts) => concatenation(parts, alphabet, builder),
        Node::Alternation(branches) => alternation(branches, alphabet, builder),
        Node::Star(inner) => star(inner, alphabet, builder),
        Node::Plus(inner) => plus(inner, alphabet, builder),
        Node::Optional(inner) => optional(inner, alphabet, builder),
        Node::Repetition(inner, repetitions) => repetition(inner, *repetitions, alphabet, builder),
    }
}

/// Returns a fragment that matches the empty string.
///
/// The function needs no alphabet, because the fragment reads no symbol.
fn epsilon<L>(builder: &mut NfaBuilder<L>) -> Fragment {
    let entry = builder.push();
    let exit = builder.push();
    builder.epsilon(entry, exit);
    Fragment::new(entry, exit)
}

/// Returns a fragment that matches each part in sequence.
///
/// The function joins the exit of each part to the entry of the next part.
fn concatenation<A: Alphabet>(
    parts: &[Node],
    alphabet: &A,
    builder: &mut NfaBuilder<A::Label>,
) -> Fragment {
    let Some((first, rest)) = parts.split_first() else {
        return epsilon(builder);
    };
    let head = fragment(first, alphabet, builder);
    let mut exit = head.exit();
    for part in rest {
        let next = fragment(part, alphabet, builder);
        builder.epsilon(exit, next.entry());
        exit = next.exit();
    }
    Fragment::new(head.entry(), exit)
}

/// Returns a fragment that matches one of the branches.
///
/// The function adds one entry state and one exit state. The entry goes to
/// each branch, and each branch goes to the exit.
fn alternation<A: Alphabet>(
    branches: &[Node],
    alphabet: &A,
    builder: &mut NfaBuilder<A::Label>,
) -> Fragment {
    let entry = builder.push();
    let exit = builder.push();
    for branch in branches {
        let inner = fragment(branch, alphabet, builder);
        builder.epsilon(entry, inner.entry());
        builder.epsilon(inner.exit(), exit);
    }
    Fragment::new(entry, exit)
}

/// Returns a fragment that matches `node` zero or more times.
fn star<A: Alphabet>(node: &Node, alphabet: &A, builder: &mut NfaBuilder<A::Label>) -> Fragment {
    let inner = fragment(node, alphabet, builder);
    let entry = builder.push();
    let exit = builder.push();
    builder.epsilon(entry, exit);
    builder.epsilon(entry, inner.entry());
    builder.epsilon(inner.exit(), exit);
    builder.epsilon(inner.exit(), inner.entry());
    Fragment::new(entry, exit)
}

/// Returns a fragment that matches `node` one or more times.
fn plus<A: Alphabet>(node: &Node, alphabet: &A, builder: &mut NfaBuilder<A::Label>) -> Fragment {
    let inner = fragment(node, alphabet, builder);
    let entry = builder.push();
    let exit = builder.push();
    builder.epsilon(entry, inner.entry());
    builder.epsilon(inner.exit(), exit);
    builder.epsilon(inner.exit(), inner.entry());
    Fragment::new(entry, exit)
}

/// Returns a fragment that matches `node` zero times or one time.
fn optional<A: Alphabet>(
    node: &Node,
    alphabet: &A,
    builder: &mut NfaBuilder<A::Label>,
) -> Fragment {
    let inner = fragment(node, alphabet, builder);
    let entry = builder.push();
    let exit = builder.push();
    builder.epsilon(entry, exit);
    builder.epsilon(entry, inner.entry());
    builder.epsilon(inner.exit(), exit);
    Fragment::new(entry, exit)
}

/// Returns a fragment that matches `node` as many times as `repetitions`
/// permits.
///
/// The construction has no counter. Thus the function makes one copy of `node`
/// for each permitted repetition, then it builds the copies as a
/// concatenation. A count of `n` costs `n` copies of the states of `node`.
///
/// The minimum gives that many copies. A maximum gives one
/// [`Optional`](Node::Optional) copy for each repetition above the minimum. No
/// maximum gives one [`Star`](Node::Star) copy.
fn repetition<A: Alphabet>(
    node: &Node,
    repetitions: Repetitions,
    alphabet: &A,
    builder: &mut NfaBuilder<A::Label>,
) -> Fragment {
    let (minimum, maximum) = bounds(repetitions);
    let mut parts = vec![node.clone(); minimum];
    match maximum {
        None => parts.push(node.clone().star()),
        Some(maximum) => parts.extend((minimum..maximum).map(|_| node.clone().optional())),
    }
    concatenation(&parts, alphabet, builder)
}

/// Returns the minimum of `repetitions`, and its maximum. A maximum of `None`
/// gives no limit.
///
/// [`Lexicon::rule`](super::Lexicon::rule) rejects an inverted
/// [`Range`](Repetitions::Range), thus the check here is a `debug_assert!`.
fn bounds(repetitions: Repetitions) -> (usize, Option<usize>) {
    match repetitions {
        Repetitions::AtLeast(minimum) => (minimum, None),
        Repetitions::Range(minimum, maximum) => {
            debug_assert!(
                minimum <= maximum,
                "a repetition of {minimum} to {maximum} times has no maximum at or above its minimum"
            );
            (minimum, Some(maximum))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::{
        Automaton, Execution, NondeterministicFiniteAutomaton, Scanner, StateId,
    };
    use crate::compiler::{ByteRange, Bytes};

    /// Builds an automaton that starts at the entry of the fragment of `node`
    /// and accepts at its exit.
    fn built(node: &Node) -> (NondeterministicFiniteAutomaton<ByteRange>, Fragment) {
        let mut builder: NfaBuilder<ByteRange> = NfaBuilder::new();
        let part = fragment(node, &Bytes, &mut builder);
        builder.accept(part.exit());
        let nfa = builder
            .build(&[part.entry()])
            .expect("a test stays below the capacity of a builder");
        (nfa, part)
    }

    /// Returns the number of the bytes that `node` matches at the start of
    /// `input`.
    fn matched(node: &Node, input: &[u8]) -> Option<usize> {
        let (nfa, _) = built(node);
        let start = 0;
        let mut execution = nfa.execute(start);
        execution
            .longest_match(start, input, |_| ())
            .map(|found| found.length)
    }

    /// Returns the number of the bytes that `pattern` matches at the start of
    /// `input`.
    fn scan(pattern: &str, input: &str) -> Option<usize> {
        let node: Node = pattern.parse().expect("the pattern is valid");
        matched(&node, input.as_bytes())
    }

    /// Returns the number of the epsilon transitions in the whole automaton.
    fn epsilon_count(nfa: &NondeterministicFiniteAutomaton<ByteRange>) -> usize {
        (0..nfa.state_count())
            .map(|index| nfa.epsilons(StateId::new(index)).len())
            .sum()
    }

    #[test]
    fn a_class_matches_one_character() {
        assert_eq!(scan("a", "abc"), Some(1));
        assert_eq!(scan("a", "b"), None);
        assert_eq!(scan("a", ""), None);
    }

    #[test]
    fn a_class_matches_a_character_of_more_than_one_byte() {
        assert_eq!(scan("é", "é"), Some(2));
        assert_eq!(matched(&"é".parse::<Node>().unwrap(), &[0xC3]), None);
    }

    #[test]
    fn a_class_of_a_range_matches_each_character_in_it() {
        assert_eq!(scan("[a-z]", "m"), Some(1));
        assert_eq!(scan("[a-z]", "a"), Some(1));
        assert_eq!(scan("[a-z]", "z"), Some(1));
        assert_eq!(scan("[a-z]", "A"), None);
    }

    #[test]
    fn a_class_of_many_encodings_reaches_the_alphabet() {
        assert_eq!(scan(".", "a"), Some(1));
        assert_eq!(scan(".", "é"), Some(2));
        assert_eq!(scan(".", "€"), Some(3));
        assert_eq!(scan(".", "🦀"), Some(4));
        assert_eq!(scan(".", "\n"), None);
    }

    #[test]
    fn an_epsilon_matches_the_empty_string() {
        assert_eq!(matched(&Node::Epsilon, b""), Some(0));
        assert_eq!(matched(&Node::Epsilon, b"a"), Some(0));
        assert_eq!(scan("", "a"), Some(0));
    }

    #[test]
    fn an_epsilon_gives_two_states_and_one_epsilon_transition() {
        let (nfa, part) = built(&Node::Epsilon);

        assert_eq!(nfa.state_count(), 2);
        assert_eq!(nfa.epsilons(part.entry()), &[part.exit()]);
        assert!(nfa.transitions(part.entry()).is_empty());
    }

    #[test]
    fn a_concatenation_matches_each_part_in_sequence() {
        assert_eq!(scan("abc", "abc"), Some(3));
        assert_eq!(scan("abc", "abcd"), Some(3));
        assert_eq!(scan("a[0-9]c", "a7c"), Some(3));
    }

    #[test]
    fn a_concatenation_needs_each_of_its_parts() {
        assert_eq!(scan("ab", "a"), None);
        assert_eq!(scan("abc", "ab"), None);
        assert_eq!(scan("abc", "abd"), None);
        assert_eq!(scan("abc", "bc"), None);
        assert_eq!(scan("abc", ""), None);
    }

    #[test]
    fn a_concatenation_counts_the_bytes_of_each_character() {
        assert_eq!(scan("héllo", "héllo"), Some(6));
        assert_eq!(scan("é.", "é!"), Some(3));
        assert_eq!(scan("é.", "e!"), None);
    }

    #[test]
    fn a_concatenation_of_no_parts_matches_the_empty_string() {
        let node = Node::Concatenation(Vec::new());

        assert_eq!(matched(&node, b""), Some(0));
        assert_eq!(matched(&node, b"a"), Some(0));
    }

    #[test]
    fn a_concatenation_of_one_part_matches_that_part() {
        let node = Node::Concatenation(vec!["a".parse().unwrap()]);

        assert_eq!(matched(&node, b"a"), Some(1));
        assert_eq!(matched(&node, b"b"), None);
    }

    #[test]
    fn a_concatenation_joins_each_pair_of_parts_with_one_epsilon_transition() {
        let (nfa, _) = built(&"ab".parse::<Node>().unwrap());

        assert_eq!(nfa.state_count(), 4);
        assert_eq!(epsilon_count(&nfa), 1);
    }

    #[test]
    fn a_long_concatenation_matches_each_of_its_parts() {
        let pattern = "a".repeat(64);

        assert_eq!(scan(&pattern, &pattern), Some(64));
        assert_eq!(scan(&pattern, &"a".repeat(63)), None);
    }

    #[test]
    fn an_alternation_matches_any_of_its_branches() {
        assert_eq!(scan("a|b|c", "a"), Some(1));
        assert_eq!(scan("a|b|c", "b"), Some(1));
        assert_eq!(scan("a|b|c", "c"), Some(1));
        assert_eq!(scan("a|b|c", "d"), None);
    }

    #[test]
    fn an_alternation_takes_the_longest_branch() {
        assert_eq!(scan("a|ab", "abc"), Some(2));
        assert_eq!(scan("ab|a", "abc"), Some(2));
        assert_eq!(scan("ab|a", "ac"), Some(1));
    }

    #[test]
    fn an_alternation_holds_branches_of_different_byte_lengths() {
        assert_eq!(scan("a|é", "a"), Some(1));
        assert_eq!(scan("a|é", "é"), Some(2));
        assert_eq!(scan("a|é", "b"), None);
    }

    #[test]
    fn an_alternation_of_one_branch_matches_that_branch() {
        let node = Node::Alternation(vec!["a".parse().unwrap()]);

        assert_eq!(matched(&node, b"a"), Some(1));
        assert_eq!(matched(&node, b"b"), None);
    }

    #[test]
    fn an_alternation_of_no_branches_matches_nothing() {
        let node = Node::Alternation(Vec::new());

        assert_eq!(matched(&node, b""), None);
        assert_eq!(matched(&node, b"a"), None);
    }

    #[test]
    fn an_empty_branch_matches_the_empty_string() {
        assert_eq!(scan("a|", "a"), Some(1));
        assert_eq!(scan("a|", "b"), Some(0));
    }

    #[test]
    fn an_alternation_gives_two_epsilon_transitions_for_each_branch() {
        let (nfa, _) = built(&"a|b".parse::<Node>().unwrap());

        assert_eq!(nfa.state_count(), 6);
        assert_eq!(epsilon_count(&nfa), 4);
    }

    #[test]
    fn an_alternation_inside_a_concatenation_matches_each_combination() {
        assert_eq!(scan("(a|b)c", "ac"), Some(2));
        assert_eq!(scan("(a|b)c", "bc"), Some(2));
        assert_eq!(scan("(a|b)c", "cc"), None);
        assert_eq!(scan("(a|b)c", "a"), None);
    }

    #[test]
    fn a_star_matches_zero_or_more_times() {
        assert_eq!(scan("a*", ""), Some(0));
        assert_eq!(scan("a*", "b"), Some(0));
        assert_eq!(scan("a*", "a"), Some(1));
        assert_eq!(scan("a*", "aaab"), Some(3));
    }

    #[test]
    fn a_plus_needs_one_time_at_least() {
        assert_eq!(scan("a+", ""), None);
        assert_eq!(scan("a+", "b"), None);
        assert_eq!(scan("a+", "a"), Some(1));
        assert_eq!(scan("a+", "aaab"), Some(3));
    }

    #[test]
    fn an_optional_matches_zero_times_or_one_time() {
        assert_eq!(scan("a?", ""), Some(0));
        assert_eq!(scan("a?", "b"), Some(0));
        assert_eq!(scan("a?", "aa"), Some(1));
    }

    #[test]
    fn a_repeated_group_repeats_each_of_its_parts() {
        assert_eq!(scan("(ab)*", "ababa"), Some(4));
        assert_eq!(scan("(ab)+", "ababa"), Some(4));
        assert_eq!(scan("(ab)+", "ba"), None);
        assert_eq!(scan("(ab)?", "abab"), Some(2));
    }

    #[test]
    fn a_repetition_of_a_character_of_more_than_one_byte_counts_bytes() {
        assert_eq!(scan("é*", "ééé"), Some(6));
        assert_eq!(scan("é+", "a"), None);
        assert_eq!(matched(&"é*".parse::<Node>().unwrap(), &[0xC3]), Some(0));
    }

    #[test]
    fn a_repetition_joins_the_parts_around_it() {
        assert_eq!(scan("a*b", "b"), Some(1));
        assert_eq!(scan("a*b", "aaab"), Some(4));
        assert_eq!(scan("a+b", "b"), None);
        assert_eq!(scan("a?b", "ab"), Some(2));
        assert_eq!(scan("(a|b)*", "abba"), Some(4));
    }

    #[test]
    fn a_star_of_a_fragment_that_matches_the_empty_string_stops() {
        assert_eq!(scan("(a?)*", "b"), Some(0));
        assert_eq!(scan("(a?)*", "aaa"), Some(3));
        assert_eq!(scan("(a*)*", "aaa"), Some(3));
        assert_eq!(scan("(a*)+", "b"), Some(0));
    }

    #[test]
    fn a_star_adds_two_states_and_four_epsilon_transitions() {
        let (nfa, _) = built(&"a*".parse::<Node>().unwrap());

        assert_eq!(nfa.state_count(), 4);
        assert_eq!(epsilon_count(&nfa), 4);
    }

    #[test]
    fn a_plus_and_an_optional_each_add_three_epsilon_transitions() {
        let (plus, _) = built(&"a+".parse::<Node>().unwrap());
        let (optional, _) = built(&"a?".parse::<Node>().unwrap());

        assert_eq!(plus.state_count(), 4);
        assert_eq!(epsilon_count(&plus), 3);
        assert_eq!(optional.state_count(), 4);
        assert_eq!(epsilon_count(&optional), 3);
    }

    #[test]
    fn a_repetition_of_an_exact_count_matches_that_count() {
        assert_eq!(scan("a{3}", "aaa"), Some(3));
        assert_eq!(scan("a{3}", "aaaa"), Some(3));
        assert_eq!(scan("a{3}", "aa"), None);
        assert_eq!(scan("a{1}", "a"), Some(1));
    }

    #[test]
    fn a_repetition_of_a_range_matches_between_its_bounds() {
        assert_eq!(scan("a{2,4}", "aa"), Some(2));
        assert_eq!(scan("a{2,4}", "aaa"), Some(3));
        assert_eq!(scan("a{2,4}", "aaaa"), Some(4));
        assert_eq!(scan("a{2,4}", "aaaaa"), Some(4));
        assert_eq!(scan("a{2,4}", "a"), None);
    }

    #[test]
    fn a_repetition_with_no_maximum_matches_each_count_above_its_minimum() {
        assert_eq!(scan("a{3,}", "aaa"), Some(3));
        assert_eq!(scan("a{3,}", "aaaaa"), Some(5));
        assert_eq!(scan("a{3,}", "aa"), None);
    }

    #[test]
    fn a_repetition_of_a_minimum_of_zero_matches_the_empty_string() {
        assert_eq!(scan("a{0}", "a"), Some(0));
        assert_eq!(scan("a{0,2}", "b"), Some(0));
        assert_eq!(scan("a{0,2}", "aaa"), Some(2));
        assert_eq!(scan("a{0,}", "b"), Some(0));
        assert_eq!(scan("a{0,}", "aaa"), Some(3));
    }

    #[test]
    fn a_repetition_of_a_group_repeats_each_of_its_parts() {
        assert_eq!(scan("(ab){2}", "ababab"), Some(4));
        assert_eq!(scan("(ab){2}", "aba"), None);
        assert_eq!(scan("(a|b){2}", "ba"), Some(2));
        assert_eq!(scan("é{2}", "éé"), Some(4));
    }

    #[test]
    fn a_counted_repetition_joins_the_parts_around_it() {
        assert_eq!(scan("a{2}b", "aab"), Some(3));
        assert_eq!(scan("a{2}b", "ab"), None);
        assert_eq!(scan("xa{0,2}y", "xy"), Some(2));
        assert_eq!(scan("xa{0,2}y", "xaay"), Some(4));
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "a repetition of 3 to 1 times")]
    fn a_maximum_below_the_minimum_panics() {
        let node: Node = "a"
            .parse::<Node>()
            .unwrap()
            .repeated(Repetitions::Range(3, 1));

        matched(&node, b"a");
    }

    #[test]
    fn the_exit_of_a_fragment_stays_clear() {
        for node in [Node::Epsilon, "a".parse().unwrap(), ".".parse().unwrap()] {
            let (nfa, part) = built(&node);

            assert!(nfa.transitions(part.exit()).is_empty());
            assert!(nfa.epsilons(part.exit()).is_empty());
            assert!(!nfa.accepts(part.entry()));
        }
    }
}
