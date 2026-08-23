use super::accepts::Accepts;
use super::alphabet::Alphabet;
use super::error::{BuildError, BuildErrorKind};
use super::lexicon::Lexicon;
use super::thompson;
use crate::automata::{Automaton, NfaBuilder, NondeterministicFiniteAutomaton, StateId};

/// The automaton of a lexer, and what each of its states means.
///
/// [`compile`](compile()) gives it.
#[derive(Debug, Clone)]
pub struct Compilation<L, R> {
    /// The automaton of each rule together.
    pub nfa: NondeterministicFiniteAutomaton<L>,
    /// The accept of each state that accepts.
    pub accepts: Accepts<R>,
    /// The index of the rule that owns each state, in the sequence of the
    /// states.
    ///
    /// A start state belongs to no rule, thus it holds `None`. Each other
    /// state comes from the pattern of one rule, because the construction
    /// makes the states of one rule at a time and joins no two rules.
    ///
    /// A pass that reads one rule of a state apart from the rest reads this.
    pub owners: Vec<Option<usize>>,
}

/// Compiles the rules of a lexer into one NFA, and into the accept of each
/// state that accepts.
///
/// The automaton has one start state for each start condition of `lexicon`.
/// The start state of a condition is at the index of its
/// start condition. Give that index to
/// [`start_state`](crate::automata::Automaton::start_state). Start 0 is
/// [`initial`](Lexicon::initial), because a lexicon declares that condition
/// before each other condition. A scan under a start state reads only the
/// rules of that condition.
///
/// The function owns the [`NfaBuilder`]. It
/// pushes one start state for each start condition. Then, for each rule, it
/// makes the states of the pattern one time with
/// [`thompson::fragment`]. It makes the exit
/// accept, and it adds an epsilon transition from each start condition of the
/// rule to the entry.
///
/// The automaton says which states accept. [`Accepts`] says what each accept
/// means. [`lowest`](Accepts::lowest) selects the lowest accept of the accepts
/// at one length. Thus give the accepts in the sequence of precedence.
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
) -> Result<Compilation<A::Label, R>, BuildError> {
    let (rules, conditions) = lexicon.into_parts();

    let mut builder = NfaBuilder::new();
    let starts: Vec<StateId> = (0..conditions).map(|_| builder.push()).collect();
    let mut marks = Vec::new();
    let mut owners = vec![None; builder.state_count()];

    for (index, rule) in rules.into_iter().enumerate() {
        let part = thompson::fragment(&rule.pattern, &alphabet, &mut builder);
        builder.accept(part.exit());
        marks.push((part.exit(), rule.accept));
        owners.resize(builder.state_count(), Some(index));
        for condition in rule.conditions {
            let start = *starts
                .get(condition)
                .expect("a lexicon declares each start condition of each of its rules");
            builder.epsilon(start, part.entry());
        }
    }

    let nfa = builder
        .build(&starts)
        .map_err(|overflow| BuildErrorKind::from(overflow).in_lexicon())?;
    let accepts = Accepts::new(nfa.state_count(), marks);
    owners.resize(nfa.state_count(), None);
    Ok(Compilation {
        nfa,
        accepts,
        owners,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::{Execution, Scanner};
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

    /// The automaton of a lexer, and the accept of each state that accepts.
    struct Lexer<T> {
        automaton: T,
        accepts: Accepts<Token>,
    }

    impl<T: Scanner<Symbol = u8>> Lexer<T> {
        /// Returns the accept and the length of the longest match at the start
        /// of `input`, under `start`.
        fn scan(&self, start: usize, input: &str) -> Option<(Token, usize)> {
            self.matched(&mut self.automaton.execute(), start, input)
        }

        /// Returns the accept and the length of the longest match that
        /// `execution` finds at the start of `input`, under `start`.
        ///
        /// A driver makes one execution, then it reads each token with that
        /// execution. Thus the scan makes the buffers one time.
        fn matched(
            &self,
            execution: &mut T::Execution<'_>,
            start: usize,
            input: &str,
        ) -> Option<(Token, usize)> {
            execution
                .longest_match(start, input.as_bytes(), |states| {
                    self.accepts
                        .lowest(states)
                        .expect("a state that accepts has an accept")
                })
                .map(|found| (found.accept, found.length))
        }

        /// Reads each token of `input`, in the manner of a lexer driver.
        ///
        /// The scan starts under `code`. A [`Quote`](Token::Quote) changes the
        /// start condition, thus the same bytes give a different token inside
        /// a string.
        fn tokens(&self, code: usize, string: usize, input: &str) -> Vec<Token> {
            let mut execution = self.automaton.execute();
            let mut condition = code;
            let mut offset = 0;
            let mut found = Vec::new();

            while offset < input.len() {
                let (accept, length) = self
                    .matched(&mut execution, condition, &input[offset..])
                    .expect("each byte of the input belongs to a token");
                assert!(length > 0, "a rule that reads no byte stops the scan");
                offset += length;
                if accept == Token::Quote {
                    condition = if condition == code { string } else { code };
                }
                found.push(accept);
            }

            found
        }
    }

    /// Adds a rule to `lexicon`. Each rule of a test is valid.
    fn rule(lexicon: &mut Lexicon<Token>, pattern: &str, accept: Token, conditions: &[usize]) {
        let pattern: Node = pattern.parse().expect("the pattern is valid");
        lexicon
            .rule(pattern, accept, conditions)
            .expect("the rule passes each check");
    }

    /// Compiles `lexicon`. A test stays below the capacity of an automaton.
    fn compiled(lexicon: Lexicon<Token>) -> Lexer<NondeterministicFiniteAutomaton<ByteRange>> {
        let compilation = compile(Bytes, lexicon).expect("a test stays below the capacity");
        Lexer {
            automaton: compilation.nfa,
            accepts: compilation.accepts,
        }
    }

    /// Builds a lexer that has a code condition and a string condition.
    fn lexer() -> (
        Lexer<NondeterministicFiniteAutomaton<ByteRange>>,
        usize,
        usize,
    ) {
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
        let (lexer, code, string) = lexer();

        assert_eq!(lexer.scan(code, "let"), Some((Token::Keyword, 3)));
        assert_eq!(lexer.scan(string, "let"), Some((Token::Text, 3)));
        assert_eq!(lexer.scan(code, "\"a"), Some((Token::Quote, 1)));
        assert_eq!(lexer.scan(string, "\"a"), Some((Token::Quote, 1)));
    }

    #[test]
    fn the_lowest_accept_wins_at_the_same_length() {
        let (lexer, code, _) = lexer();

        assert_eq!(lexer.scan(code, "let"), Some((Token::Keyword, 3)));
        assert_eq!(lexer.scan(code, "letter"), Some((Token::Identifier, 6)));
    }

    #[test]
    fn the_longest_match_wins_across_the_rules_of_one_condition() {
        let (lexer, _, string) = lexer();

        assert_eq!(lexer.scan(string, "abc\""), Some((Token::Text, 3)));
    }

    #[test]
    fn a_rule_of_no_condition_of_the_scan_does_not_match() {
        let (lexer, code, string) = lexer();

        assert_eq!(lexer.scan(code, "!!"), None);
        assert_eq!(lexer.scan(string, "!!"), Some((Token::Text, 2)));
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

        let one = compiled(one).automaton;
        let two = compiled(two).automaton;

        assert_eq!(two.state_count(), one.state_count() + 1);
        assert_eq!(two.epsilons(two.start_state(code)).len(), 1);
        assert_eq!(two.epsilons(two.start_state(string)).len(), 1);
    }

    #[test]
    fn a_lexicon_of_no_rule_matches_nothing() {
        let lexicon: Lexicon<Token> = Lexicon::new();
        let start = lexicon.initial();
        let lexer = compiled(lexicon);

        assert_eq!(lexer.automaton.start_count(), 1);
        assert_eq!(lexer.scan(start, "a"), None);
        assert_eq!(lexer.scan(start, ""), None);
    }

    #[test]
    fn a_lexer_reads_each_token_of_its_input_in_sequence() {
        let (lexer, code, string) = lexer();

        assert_eq!(
            lexer.tokens(code, string, "let\"let\"fn"),
            vec![
                Token::Keyword,
                Token::Quote,
                Token::Text,
                Token::Quote,
                Token::Keyword,
            ],
        );
        assert_eq!(
            lexer.tokens(code, string, "letter\"a b\""),
            vec![Token::Identifier, Token::Quote, Token::Text, Token::Quote,],
        );
    }

    #[test]
    fn each_condition_gets_its_own_start_state() {
        let (lexer, code, string) = lexer();

        assert_eq!(lexer.automaton.start_count(), 2);
        assert_ne!(
            lexer.automaton.start_state(code),
            lexer.automaton.start_state(string)
        );
    }
}
