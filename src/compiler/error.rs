use crate::automata::{Overflow, Part};
use std::fmt::{Display, Formatter, Result};

/// A failure to build the automaton of a lexer.
///
/// The error gives the index of the rule at fault, and the kind of the
/// failure. A lexicon keeps its rules in the sequence in which they arrived,
/// thus the index names the rule that the lexer author wrote. A fault that
/// belongs to the whole lexicon gives no index.
///
/// The derive macro turns this error into a `compile_error!` at the span of
/// that rule, or at the span of the lexer. Thus each check on a rule gives a
/// `BuildError`, and no check on a rule panics.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct BuildError {
    /// The index of the rule at fault, in the sequence in which the rules
    /// arrived. A fault of the whole lexicon gives `None`.
    pub rule: Option<usize>,
    /// The kind of the failure.
    pub kind: BuildErrorKind,
}

/// The kind of failure that a [`BuildError`] reports.
///
/// The enum gets a new variant for each new check. Thus the enum is not
/// exhaustive, and a match on it needs a `_` arm.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BuildErrorKind {
    /// A rule is applicable under no start condition, thus no scan reaches it.
    NoCondition,
    /// A rule names a start condition that the lexicon did not declare.
    UnknownCondition {
        /// The index of the start condition that the rule names.
        condition: usize,
        /// The number of the start conditions that the lexicon declared.
        declared: usize,
    },
    /// The pattern of a rule matches the empty string. Such a rule gives a
    /// match of no length, thus a driver that advances by the length of the
    /// match makes no progress.
    MatchesEmpty,
    /// The repetitions of a pattern expand it above the maximum size.
    PatternTooLarge {
        /// The number of the nodes after the expansion.
        size: usize,
        /// The maximum number of the nodes.
        maximum: usize,
    },
    /// A repetition of the pattern of a rule has a maximum below its minimum.
    /// Such a repetition matches nothing.
    InvertedRepetition {
        /// The minimum count of the repetition.
        minimum: usize,
        /// The maximum count of the repetition.
        maximum: usize,
    },
    /// The rules need a larger automaton than one automaton holds.
    TooLarge {
        /// The part of the automaton that is full.
        part: Part,
        /// The maximum number of the items in that part.
        maximum: usize,
    },
}

impl BuildErrorKind {
    /// Joins this kind to the rule at `rule`, then gives the error.
    pub(super) fn in_rule(self, rule: usize) -> BuildError {
        BuildError {
            rule: Some(rule),
            kind: self,
        }
    }

    /// Joins this kind to the whole lexicon, then gives the error.
    ///
    /// A ceiling of the automaton belongs to each rule together, thus the
    /// error names no rule.
    pub(super) fn in_lexicon(self) -> BuildError {
        BuildError {
            rule: None,
            kind: self,
        }
    }
}

impl From<Overflow> for BuildErrorKind {
    /// Takes the part and the maximum of `overflow`.
    ///
    /// The kind embeds both values. Thus a `BuildError` needs no other error,
    /// and it stays small and `Clone`.
    fn from(overflow: Overflow) -> Self {
        Self::TooLarge {
            part: overflow.part,
            maximum: overflow.maximum,
        }
    }
}

impl Display for BuildError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        match self.rule {
            Some(rule) => write!(formatter, "{} in rule {rule}", self.kind),
            None => write!(formatter, "{}", self.kind),
        }
    }
}

impl std::error::Error for BuildError {}

impl Display for BuildErrorKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        match self {
            Self::NoCondition => {
                write!(formatter, "a rule needs at least one start condition")
            }
            Self::UnknownCondition {
                condition,
                declared,
            } => write!(
                formatter,
                "start condition {condition} is outside a lexicon of \
                 {declared} start conditions"
            ),
            Self::MatchesEmpty => write!(
                formatter,
                "a rule needs a pattern that reads at least one character"
            ),
            Self::PatternTooLarge { size, maximum } => write!(
                formatter,
                "the repetitions expand a pattern to {size} nodes, above the \
                 maximum of {maximum} nodes"
            ),
            Self::InvertedRepetition { minimum, maximum } => write!(
                formatter,
                "a repetition of {minimum} to {maximum} times has no maximum \
                 at or above its minimum"
            ),
            Self::TooLarge { part, maximum } => write!(
                formatter,
                "the rules need more than the {maximum} {part} of one \
                 automaton"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_error_names_the_rule_at_fault() {
        let error = BuildErrorKind::NoCondition.in_rule(3);

        assert_eq!(error.rule, Some(3));
        assert_eq!(
            error.to_string(),
            "a rule needs at least one start condition in rule 3"
        );
    }

    #[test]
    fn an_error_of_the_whole_lexicon_names_no_rule() {
        let error = BuildErrorKind::TooLarge {
            part: Part::States,
            maximum: 8,
        }
        .in_lexicon();

        assert_eq!(error.rule, None);
        assert_eq!(
            error.to_string(),
            "the rules need more than the 8 states of one automaton"
        );
    }

    #[test]
    fn an_overflow_becomes_the_kind_that_holds_its_part_and_its_maximum() {
        let overflow = Overflow::new(Part::States, 8);

        assert_eq!(
            BuildErrorKind::from(overflow),
            BuildErrorKind::TooLarge {
                part: Part::States,
                maximum: 8,
            }
        );
    }

    #[test]
    fn each_kind_gives_a_message() {
        let kinds = [
            BuildErrorKind::UnknownCondition {
                condition: 2,
                declared: 1,
            },
            BuildErrorKind::MatchesEmpty,
            BuildErrorKind::PatternTooLarge {
                size: 200,
                maximum: 100,
            },
            BuildErrorKind::InvertedRepetition {
                minimum: 5,
                maximum: 2,
            },
            BuildErrorKind::TooLarge {
                part: Part::Items,
                maximum: 4,
            },
        ];

        for kind in kinds {
            assert!(!kind.to_string().is_empty());
        }
    }
}
