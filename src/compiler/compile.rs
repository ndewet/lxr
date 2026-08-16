use super::alphabet::Alphabet;
use super::error::{BuildError, BuildErrorKind};
use super::lexicon::Lexicon;
use super::thompson;
use crate::automata::{Nfa, NfaBuilder, StateId};

/// Compiles the rules of a lexer into one NFA.
///
/// The automaton has one start state for each start condition of `lexicon`.
/// The start state of a condition is at the index of its
/// [`StartId`](crate::automata::StartId). Give that identifier to
/// [`start_state`](Nfa::start_state). Start 0 is
/// [`initial`](Lexicon::initial), because a lexicon declares that condition
/// before each other condition. A scan under a start state reads only the
/// rules of that condition.
///
/// The function owns the [`NfaBuilder`](crate::automata::NfaBuilder). It
/// pushes one start state for each start condition. Then, for each rule, it
/// makes the states of the pattern one time with
/// [`thompson::fragment`](super::thompson::fragment). It makes the exit
/// accept, and it adds an epsilon transition from each start condition of the
/// rule to the entry.
///
/// [`longest_match`](crate::automata::longest_match) selects the lowest accept
/// of the accepts that it reaches at the longest length. Thus give the accepts
/// in the sequence of precedence.
///
/// # Errors
///
/// This function returns a [`BuildError`] of the kind
/// [`TooLarge`](BuildErrorKind::TooLarge) if the rules together need a larger
/// automaton than one automaton holds. [`Lexicon::rule`] finds each other
/// fault.
pub fn compile<A: Alphabet, R>(
    alphabet: A,
    lexicon: Lexicon<R>,
) -> Result<Nfa<A::Label, R>, BuildError> {
    let (rules, conditions) = lexicon.into_parts();

    let mut builder = NfaBuilder::new();
    let starts: Vec<StateId> = (0..conditions).map(|_| builder.push()).collect();

    for rule in rules {
        let part = thompson::fragment(&rule.pattern, &alphabet, &mut builder);
        builder.accept(part.exit(), rule.accept);
        for condition in rule.conditions {
            builder.epsilon(starts[condition.index()], part.entry());
        }
    }

    builder
        .build(&starts)
        .map_err(|overflow| BuildErrorKind::from(overflow).in_lexicon())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::{Automaton, Dfa, Label, StartId, StateId, longest_match};
    use crate::compiler::{ByteRange, Bytes};
    use crate::regex::Node;

    /// The accepts of the test lexer, in the sequence of precedence.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum Token {
        Keyword,
        Identifier,
        Quote,
        Text,
    }

    /// Returns the accept and the length of the longest match at the start of
    /// `input`, under `start`.
    fn scan<T>(automaton: &T, start: StartId, input: &str) -> Option<(Token, usize)>
    where
        T: Automaton<Symbol = u8, Accept = Token>,
    {
        let mut execution = automaton.execute(start);
        longest_match(&mut execution, start, input.as_bytes())
            .map(|found| (found.accept, found.length))
    }

    /// Adds a rule to `lexicon`. Each rule of a test is valid.
    fn rule(lexicon: &mut Lexicon<Token>, pattern: &str, accept: Token, conditions: &[StartId]) {
        let pattern: Node = pattern.parse().expect("the pattern is valid");
        lexicon
            .rule(pattern, accept, conditions)
            .expect("the rule passes each check");
    }

    /// Compiles `lexicon`. A test stays below the capacity of an automaton.
    fn compiled(lexicon: Lexicon<Token>) -> Nfa<ByteRange, Token> {
        compile(Bytes, lexicon).expect("a test stays below the capacity")
    }

    /// Builds a lexer that has a code condition and a string condition.
    fn lexer() -> (Nfa<ByteRange, Token>, StartId, StartId) {
        let mut lexicon = Lexicon::new();
        let code = lexicon.initial();
        let string = lexicon.condition();

        rule(&mut lexicon, "let|fn", Token::Keyword, &[code]);
        rule(&mut lexicon, "[a-z][a-z0-9]*", Token::Identifier, &[code]);
        rule(&mut lexicon, "\"", Token::Quote, &[code, string]);
        rule(&mut lexicon, "[^\"]+", Token::Text, &[string]);

        (compiled(lexicon), code, string)
    }

    #[test]
    fn each_start_condition_reads_only_its_own_rules() {
        let (nfa, code, string) = lexer();

        assert_eq!(scan(&nfa, code, "let"), Some((Token::Keyword, 3)));
        assert_eq!(scan(&nfa, string, "let"), Some((Token::Text, 3)));
        assert_eq!(scan(&nfa, code, "\"a"), Some((Token::Quote, 1)));
        assert_eq!(scan(&nfa, string, "\"a"), Some((Token::Quote, 1)));
    }

    #[test]
    fn the_lowest_accept_wins_at_the_same_length() {
        let (nfa, code, _) = lexer();

        assert_eq!(scan(&nfa, code, "let"), Some((Token::Keyword, 3)));
        assert_eq!(scan(&nfa, code, "letter"), Some((Token::Identifier, 6)));
    }

    #[test]
    fn the_longest_match_wins_across_the_rules_of_one_condition() {
        let (nfa, _, string) = lexer();

        assert_eq!(scan(&nfa, string, "abc\""), Some((Token::Text, 3)));
    }

    #[test]
    fn a_rule_of_no_condition_of_the_scan_does_not_match() {
        let (nfa, code, string) = lexer();

        assert_eq!(scan(&nfa, code, "!!"), None);
        assert_eq!(scan(&nfa, string, "!!"), Some((Token::Text, 2)));
    }

    #[test]
    fn a_rule_of_two_conditions_costs_one_set_of_states() {
        let mut one = Lexicon::new();
        let code = one.initial();
        rule(&mut one, "ab", Token::Quote, &[code]);

        let mut two = Lexicon::new();
        let code = two.initial();
        let string = two.condition();
        rule(&mut two, "ab", Token::Quote, &[code, string]);

        let one = compiled(one);
        let two = compiled(two);

        assert_eq!(two.state_count(), one.state_count() + 1);
        assert_eq!(two.epsilons(two.start_state(code)).len(), 1);
        assert_eq!(two.epsilons(two.start_state(string)).len(), 1);
    }

    #[test]
    fn a_lexicon_of_no_rule_matches_nothing() {
        let lexicon: Lexicon<Token> = Lexicon::new();
        let start = lexicon.initial();
        let nfa = compiled(lexicon);

        assert_eq!(nfa.start_count(), 1);
        assert_eq!(scan(&nfa, start, "a"), None);
        assert_eq!(scan(&nfa, start, ""), None);
    }

    /// Reads each token of `input`, in the manner of a lexer driver.
    ///
    /// The scan starts under `code`. A [`Quote`](Token::Quote) changes the
    /// start condition, thus the same bytes give a different token inside a
    /// string.
    fn tokens<T>(automaton: &T, code: StartId, string: StartId, input: &str) -> Vec<Token>
    where
        T: Automaton<Symbol = u8, Accept = Token>,
    {
        let mut execution = automaton.execute(code);
        let mut condition = code;
        let mut offset = 0;
        let mut found = Vec::new();

        while offset < input.len() {
            let scanned = longest_match(&mut execution, condition, &input.as_bytes()[offset..])
                .expect("each byte of the input belongs to a token");
            assert!(
                scanned.length > 0,
                "a rule that reads no byte stops the scan"
            );
            offset += scanned.length;
            if scanned.accept == Token::Quote {
                condition = if condition == code { string } else { code };
            }
            found.push(scanned.accept);
        }

        found
    }

    #[test]
    fn a_lexer_reads_each_token_of_its_input_in_sequence() {
        let (nfa, code, string) = lexer();

        assert_eq!(
            tokens(&nfa, code, string, "let\"let\"fn"),
            vec![
                Token::Keyword,
                Token::Quote,
                Token::Text,
                Token::Quote,
                Token::Keyword,
            ],
        );
        assert_eq!(
            tokens(&nfa, code, string, "letter\"a b\""),
            vec![Token::Identifier, Token::Quote, Token::Text, Token::Quote,],
        );
    }

    #[test]
    fn each_condition_gets_its_own_start_state() {
        let (nfa, code, string) = lexer();

        assert_eq!(nfa.start_count(), 2);
        assert_ne!(nfa.start_state(code), nfa.start_state(string));
    }

    /// Determinizes `nfa`. A test stays below the capacity of an automaton.
    fn determinized(nfa: &Nfa<ByteRange, Token>) -> Dfa<ByteRange, Token> {
        nfa.determinize().expect("a test stays below the capacity")
    }

    /// The inputs of the differential test. Each one is a token, a part of a
    /// token, or an input that the lexer rejects.
    const INPUTS: [&str; 18] = [
        "", "l", "let", "letter", "let9", "f", "fn", "fun", "a", "z9", "9", "!", " ", "\"",
        "\"let\"", "a b", "é", "\u{80}",
    ];

    #[test]
    fn the_dfa_of_a_lexer_gives_the_same_match_as_its_nfa() {
        let (nfa, code, string) = lexer();
        let dfa = determinized(&nfa);

        for start in [code, string] {
            for input in INPUTS {
                assert_eq!(
                    scan(&dfa, start, input),
                    scan(&nfa, start, input),
                    "input {input:?} under start {}",
                    start.index()
                );
            }
        }
    }

    #[test]
    fn a_lexer_reads_each_token_of_its_input_with_a_dfa() {
        let (nfa, code, string) = lexer();
        let dfa = determinized(&nfa);

        assert_eq!(
            tokens(&dfa, code, string, "let\"let\"fn"),
            vec![
                Token::Keyword,
                Token::Quote,
                Token::Text,
                Token::Quote,
                Token::Keyword,
            ],
        );
        assert_eq!(
            tokens(&dfa, code, string, "letter\"a b\""),
            vec![Token::Identifier, Token::Quote, Token::Text, Token::Quote,],
        );
    }

    #[test]
    fn each_state_of_the_dfa_reads_one_byte_into_a_maximum_of_one_state() {
        let (nfa, _, _) = lexer();
        let dfa = determinized(&nfa);

        for index in 0..dfa.state_count() {
            let transitions = dfa.transitions(StateId::new(index));
            for byte in 0..=u8::MAX {
                let count = transitions
                    .iter()
                    .filter(|transition| transition.label.matches(byte))
                    .count();
                assert!(
                    count <= 1,
                    "state {index} reads the byte {byte:#04X} into {count} states"
                );
            }
        }
    }

    #[test]
    fn the_dfa_keeps_one_start_state_for_each_start_condition() {
        let (nfa, code, string) = lexer();
        let dfa = determinized(&nfa);

        assert_eq!(dfa.start_count(), nfa.start_count());
        assert_ne!(dfa.start_state(code), dfa.start_state(string));
    }
}
