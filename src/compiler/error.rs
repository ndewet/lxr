use crate::automata::{Overflow, Part};
use std::fmt::{Display, Formatter, Result};

/// A failure to build the automaton of a lexer.
///
/// The index of the rule gives the derive macro the span for its
/// `compile_error!`.
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
    /// match of no length, thus the lexer makes no progress.
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
    /// A ceiling of the automaton belongs to each rule together.
    pub(super) fn in_lexicon(self) -> BuildError {
        BuildError {
            rule: None,
            kind: self,
        }
    }

    /// Returns the correction for this kind of failure.
    ///
    /// A caller shows the text under the message, in the manner of a note from
    /// the compiler.
    pub fn help(&self) -> &'static str {
        match self {
            Self::NoCondition => "Give the rule at least one start condition.",
            Self::UnknownCondition { .. } => "Declare the start condition in this lexicon.",
            Self::MatchesEmpty => {
                "Each part of the pattern can match nothing. Write `+` in \
                 place of `*`, or remove a `?`."
            }
            Self::PatternTooLarge { .. } => {
                "Lower a repetition count. The construction makes one copy of \
                 the expression for each repetition."
            }
            Self::InvertedRepetition { .. } => "Write the minimum first, for example `{2,5}`.",
            Self::TooLarge { .. } => "Divide the lexer, or lower a repetition count.",
        }
    }
}

impl From<Overflow> for BuildErrorKind {
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
            Some(rule) => write!(formatter, "rule {rule}: {}", self.kind),
            None => write!(formatter, "{}", self.kind),
        }
    }
}

impl std::error::Error for BuildError {}

impl Display for BuildErrorKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        match self {
            Self::NoCondition => {
                write!(formatter, "the rule is applicable under no start condition")
            }
            Self::UnknownCondition { condition, .. } => {
                write!(formatter, "start condition {condition} is not declared")
            }
            Self::MatchesEmpty => {
                write!(formatter, "the pattern matches the empty string")
            }
            Self::PatternTooLarge { size, maximum } => write!(
                formatter,
                "the repetitions make {size} copies of the pattern, above the \
                 limit of {maximum}"
            ),
            Self::InvertedRepetition { minimum, maximum } => write!(
                formatter,
                "the repetition {{{minimum},{maximum}}} has a maximum below \
                 its minimum"
            ),
            Self::TooLarge { part, maximum } => write!(
                formatter,
                "the lexer needs more than the {maximum} {part} that one \
                 automaton holds"
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
            "rule 3: the rule is applicable under no start condition"
        );
        assert_eq!(
            error.kind.help(),
            "Give the rule at least one start condition."
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
            "the lexer needs more than the 8 states that one automaton holds"
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
            assert!(kind.help().ends_with('.'));
        }
    }

    #[test]
    fn a_message_names_the_construction_that_the_author_wrote() {
        let inverted = BuildErrorKind::InvertedRepetition {
            minimum: 5,
            maximum: 2,
        };
        let condition = BuildErrorKind::UnknownCondition {
            condition: 2,
            declared: 1,
        };

        assert_eq!(
            inverted.to_string(),
            "the repetition {5,2} has a maximum below its minimum"
        );
        assert_eq!(condition.to_string(), "start condition 2 is not declared");
    }
}
