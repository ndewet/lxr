use super::rule::Rule;
use crate::automata::StartId;
use crate::regex::Node;

/// The rules of a lexer, and the start conditions that they belong to.
///
/// A start condition is a start state of the automaton. The lexer scans each
/// token under one condition, thus only the rules of that condition can match.
/// A string and a comment each need their own condition.
///
/// A lexicon hands out each [`StartId`]. It is the only source of one, thus a
/// rule cannot name a condition that does not exist. [`compile`](super::compile)
/// needs no count, and it needs no check.
///
/// Declare the conditions with [`condition`](Self::condition), add the rules
/// with [`rule`](Self::rule), then give the lexicon to
/// [`compile`](super::compile).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lexicon<R> {
    rules: Vec<Rule<R>>,
    conditions: usize,
}

impl<R> Lexicon<R> {
    /// Creates a lexicon that has one start condition and no rule.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the start condition that each lexicon has.
    ///
    /// A lexer that needs one condition needs only this one.
    pub fn initial(&self) -> StartId {
        StartId::new(0)
    }

    /// Declares a start condition, then returns its identifier.
    pub fn condition(&mut self) -> StartId {
        let id = StartId::new(self.conditions);
        self.conditions += 1;
        id
    }

    /// Adds a rule that matches `pattern` and gives `accept`.
    ///
    /// The rule is applicable under each start condition in `conditions`. The
    /// states of the pattern are built one time, whatever the number of the
    /// conditions.
    ///
    /// [`longest_match`](crate::automata::longest_match) selects the lowest
    /// accept of the accepts that it reaches at the longest length. Thus give
    /// the accepts in the sequence of precedence.
    ///
    /// # Panics
    ///
    /// This function panics if `conditions` is empty, because no scan can
    /// reach such a rule.
    pub fn rule(&mut self, pattern: Node, accept: R, conditions: &[StartId]) {
        assert!(
            !conditions.is_empty(),
            "a rule needs at least one start condition"
        );
        self.rules
            .push(Rule::new(pattern, accept, conditions.to_vec()));
    }

    /// Returns the rules, and the number of the start conditions.
    pub(super) fn into_parts(self) -> (Vec<Rule<R>>, usize) {
        (self.rules, self.conditions)
    }
}

impl<R> Default for Lexicon<R> {
    /// Creates a lexicon that has one start condition and no rule.
    fn default() -> Self {
        Self {
            rules: Vec::new(),
            conditions: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regex::CharSet;

    fn lexicon() -> Lexicon<u32> {
        Lexicon::new()
    }

    fn class(character: char) -> Node {
        Node::Class(CharSet::single(character))
    }

    #[test]
    fn a_new_lexicon_has_one_start_condition_and_no_rule() {
        let lexicon = lexicon();

        assert_eq!(lexicon.initial(), StartId::new(0));

        let (rules, conditions) = lexicon.into_parts();
        assert!(rules.is_empty());
        assert_eq!(conditions, 1);
    }

    #[test]
    fn each_condition_gets_the_next_identifier() {
        let mut lexicon = lexicon();

        assert_eq!(lexicon.condition(), StartId::new(1));
        assert_eq!(lexicon.condition(), StartId::new(2));
        assert_eq!(lexicon.initial(), StartId::new(0));
        assert_eq!(lexicon.into_parts().1, 3);
    }

    #[test]
    fn a_lexicon_keeps_its_rules_in_the_sequence_in_which_they_arrived() {
        let mut lexicon = lexicon();
        let code = lexicon.initial();
        let string = lexicon.condition();
        lexicon.rule(class('a'), 0, &[code]);
        lexicon.rule(class('b'), 1, &[code, string]);

        let (rules, conditions) = lexicon.into_parts();
        assert_eq!(conditions, 2);
        assert_eq!(
            rules,
            vec![
                Rule::new(class('a'), 0, vec![code]),
                Rule::new(class('b'), 1, vec![code, string]),
            ]
        );
    }

    #[test]
    fn a_default_lexicon_is_a_new_lexicon() {
        assert_eq!(Lexicon::<u32>::default(), lexicon());
    }

    #[test]
    #[should_panic(expected = "a rule needs at least one start condition")]
    fn a_rule_with_no_start_condition_panics() {
        lexicon().rule(class('a'), 0, &[]);
    }
}
