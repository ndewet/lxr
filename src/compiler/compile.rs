use super::alphabet::Alphabet;
use super::lexicon::Lexicon;
use super::thompson;
use crate::automata::{Nfa, NfaBuilder, StateId};

/// Compiles the rules of a lexer into one NFA.
///
/// The automaton has one start state for each start condition of `lexicon`.
/// Start `i` is the condition that the `i` call to
/// [`condition`](Lexicon::condition) gave. A scan under a start state reads
/// only the rules of that condition.
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
pub fn compile<A: Alphabet, R>(alphabet: A, lexicon: Lexicon<R>) -> Nfa<A::Label, R> {
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

    builder.build(&starts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::{Automaton, StartId, longest_match};
    use crate::compiler::{ByteRange, Bytes};

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
    fn scan(nfa: &Nfa<ByteRange, Token>, start: StartId, input: &str) -> Option<(Token, usize)> {
        let mut execution = nfa.execute(start);
        longest_match(&mut execution, start, input.as_bytes())
            .map(|found| (found.accept, found.length))
    }

    /// Builds a lexer that has a code condition and a string condition.
    fn lexer() -> (Nfa<ByteRange, Token>, StartId, StartId) {
        let mut lexicon = Lexicon::new();
        let code = lexicon.initial();
        let string = lexicon.condition();

        lexicon.rule("let|fn".parse().unwrap(), Token::Keyword, &[code]);
        lexicon.rule(
            "[a-z][a-z0-9]*".parse().unwrap(),
            Token::Identifier,
            &[code],
        );
        lexicon.rule("\"".parse().unwrap(), Token::Quote, &[code, string]);
        lexicon.rule("[^\"]+".parse().unwrap(), Token::Text, &[string]);

        (compile(Bytes, lexicon), code, string)
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
        one.rule("ab".parse().unwrap(), Token::Quote, &[code]);

        let mut two = Lexicon::new();
        let code = two.initial();
        let string = two.condition();
        two.rule("ab".parse().unwrap(), Token::Quote, &[code, string]);

        let one = compile(Bytes, one);
        let two = compile(Bytes, two);

        assert_eq!(two.state_count(), one.state_count() + 1);
        assert_eq!(two.epsilons(two.start_state(code)).len(), 1);
        assert_eq!(two.epsilons(two.start_state(string)).len(), 1);
    }

    #[test]
    fn a_lexicon_of_no_rule_matches_nothing() {
        let lexicon: Lexicon<Token> = Lexicon::new();
        let start = lexicon.initial();
        let nfa = compile(Bytes, lexicon);

        assert_eq!(nfa.start_count(), 1);
        assert_eq!(scan(&nfa, start, "a"), None);
        assert_eq!(scan(&nfa, start, ""), None);
    }

    /// Reads each token of `input`, in the manner of a lexer driver.
    ///
    /// The scan starts under `code`. A [`Quote`](Token::Quote) changes the
    /// start condition, thus the same bytes give a different token inside a
    /// string.
    fn tokens(
        nfa: &Nfa<ByteRange, Token>,
        code: StartId,
        string: StartId,
        input: &str,
    ) -> Vec<Token> {
        let mut execution = nfa.execute(code);
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
}
