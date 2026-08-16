use super::error::{BuildError, BuildErrorKind};
use super::rule::Rule;
use crate::automata::StartId;
use crate::regex::{Node, Repetitions};

/// The maximum size of the pattern of a rule.
///
/// The construction has no counter. Thus it makes one copy of the expression
/// for each permitted repetition, and a nested repetition multiplies the count
/// of the copies. A pattern of 17 characters can ask for more states than the
/// memory holds.
///
/// [`Lexicon::rule`] rejects a pattern of an
/// [`expanded_size`](Node::expanded_size) above this maximum. One node costs
/// at most 20 states, thus the pattern of a rule costs at most about two
/// million states. A pattern that a person writes stays far below the maximum.
pub const MAX_PATTERN_SIZE: usize = 100_000;

/// The rules of a lexer, and the start conditions that they belong to.
///
/// A start condition is a start state of the automaton. The lexer scans each
/// token under one condition, thus only the rules of that condition can match.
/// A string and a comment each need their own condition.
///
/// A lexicon hands out each [`StartId`]. [`rule`](Self::rule) rejects an
/// identifier that this lexicon did not declare, thus
/// [`compile`](super::compile) needs no check.
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
    /// A rule that fails a check changes nothing.
    ///
    /// # Errors
    ///
    /// This function returns a [`BuildError`] of one of these kinds:
    ///
    /// - [`NoCondition`](BuildErrorKind::NoCondition), if `conditions` is
    ///   empty.
    /// - [`UnknownCondition`](BuildErrorKind::UnknownCondition), if
    ///   `conditions` holds an identifier that this lexicon did not declare.
    /// - [`MatchesEmpty`](BuildErrorKind::MatchesEmpty), if `pattern` matches
    ///   the empty string.
    /// - [`InvertedRepetition`](BuildErrorKind::InvertedRepetition), if a
    ///   repetition of `pattern` has a maximum below its minimum.
    /// - [`PatternTooLarge`](BuildErrorKind::PatternTooLarge), if the
    ///   [`expanded_size`](Node::expanded_size) of `pattern` is above
    ///   [`MAX_PATTERN_SIZE`].
    pub fn rule(
        &mut self,
        pattern: Node,
        accept: R,
        conditions: &[StartId],
    ) -> Result<(), BuildError> {
        let index = self.rules.len();
        self.check(&pattern, conditions)
            .map_err(|kind| kind.in_rule(index))?;
        self.rules
            .push(Rule::new(pattern, accept, conditions.to_vec()));
        Ok(())
    }

    /// Returns the first kind of failure that `pattern` or `conditions` gives.
    fn check(&self, pattern: &Node, conditions: &[StartId]) -> Result<(), BuildErrorKind> {
        if conditions.is_empty() {
            return Err(BuildErrorKind::NoCondition);
        }
        for condition in conditions {
            if condition.index() >= self.conditions {
                return Err(BuildErrorKind::UnknownCondition {
                    condition: condition.index(),
                    declared: self.conditions,
                });
            }
        }
        if pattern.matches_empty() {
            return Err(BuildErrorKind::MatchesEmpty);
        }
        if let Some(Repetitions::Range(minimum, maximum)) = pattern.inverted_repetition() {
            return Err(BuildErrorKind::InvertedRepetition { minimum, maximum });
        }
        let size = pattern.expanded_size();
        if size > MAX_PATTERN_SIZE {
            return Err(BuildErrorKind::PatternTooLarge {
                size,
                maximum: MAX_PATTERN_SIZE,
            });
        }
        Ok(())
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
        assert_eq!(lexicon.rule(class('a'), 0, &[code]), Ok(()));
        assert_eq!(lexicon.rule(class('b'), 1, &[code, string]), Ok(()));

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
    fn a_rule_with_no_start_condition_is_rejected() {
        let error = lexicon().rule(class('a'), 0, &[]).unwrap_err();

        assert_eq!(error, BuildErrorKind::NoCondition.in_rule(0));
    }

    #[test]
    fn a_rule_with_a_start_condition_of_another_lexicon_is_rejected() {
        let mut other = lexicon();
        let string = other.condition();

        let mut lexicon = lexicon();
        let code = lexicon.initial();
        let error = lexicon.rule(class('a'), 0, &[code, string]).unwrap_err();

        assert_eq!(
            error,
            BuildErrorKind::UnknownCondition {
                condition: 1,
                declared: 1,
            }
            .in_rule(0)
        );
    }

    #[test]
    fn a_rule_with_a_pattern_that_matches_the_empty_string_is_rejected() {
        let mut lexicon = lexicon();
        let code = lexicon.initial();
        let pattern: Node = "a*".parse().unwrap();
        let error = lexicon.rule(pattern, 0, &[code]).unwrap_err();

        assert_eq!(error, BuildErrorKind::MatchesEmpty.in_rule(0));
    }

    #[test]
    fn a_rule_with_an_inverted_repetition_is_rejected() {
        let mut lexicon = lexicon();
        let code = lexicon.initial();
        let pattern = class('a').repeated(Repetitions::Range(5, 2));
        let error = lexicon.rule(pattern, 0, &[code]).unwrap_err();

        assert_eq!(
            error,
            BuildErrorKind::InvertedRepetition {
                minimum: 5,
                maximum: 2,
            }
            .in_rule(0)
        );
    }

    #[test]
    fn a_rule_with_a_pattern_of_nested_repetitions_is_rejected() {
        let mut lexicon = lexicon();
        let code = lexicon.initial();
        let pattern: Node = "(a{65535}){65535}".parse().unwrap();
        let error = lexicon.rule(pattern, 0, &[code]).unwrap_err();

        assert!(matches!(
            error.kind,
            BuildErrorKind::PatternTooLarge {
                maximum: MAX_PATTERN_SIZE,
                ..
            }
        ));
    }

    #[test]
    fn a_rule_with_a_pattern_below_the_maximum_size_is_accepted() {
        let mut lexicon = lexicon();
        let code = lexicon.initial();
        let pattern: Node = "(a{1000}){99}".parse().unwrap();

        assert_eq!(lexicon.rule(pattern, 0, &[code]), Ok(()));
        assert_eq!(lexicon.into_parts().0.len(), 1);
    }

    #[test]
    fn a_rule_that_fails_a_check_changes_nothing() {
        let mut lexicon = lexicon();
        let code = lexicon.initial();
        let pattern: Node = "a*".parse().unwrap();

        assert!(lexicon.rule(pattern, 0, &[code]).is_err());
        assert!(lexicon.into_parts().0.is_empty());
    }

    #[test]
    fn an_error_names_the_index_of_the_rule_at_fault() {
        let mut lexicon = lexicon();
        let code = lexicon.initial();
        let pattern: Node = "a*".parse().unwrap();

        assert_eq!(lexicon.rule(class('a'), 0, &[code]), Ok(()));
        let error = lexicon.rule(pattern, 1, &[code]).unwrap_err();

        assert_eq!(error.rule, Some(1));
    }
}
