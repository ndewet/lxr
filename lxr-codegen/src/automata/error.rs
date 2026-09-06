use std::fmt::{Display, Formatter};

/// Reports an automaton capacity limit.
///
/// Capacity errors come from lexer input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) enum BuildError {
    /// The state count exceeds the capacity.
    TooManyStates {
        /// The maximum state count.
        capacity: usize,
    },
    /// The transition count exceeds the capacity.
    TooManyTransitions {
        /// The maximum transition count.
        capacity: usize,
    },
}

impl Display for BuildError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyStates { capacity } => {
                write!(formatter, "an automaton holds at most {capacity} states")
            }
            Self::TooManyTransitions { capacity } => {
                write!(
                    formatter,
                    "an automaton holds at most {capacity} transitions"
                )
            }
        }
    }
}

impl std::error::Error for BuildError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_state_capacity_error_names_its_capacity() {
        let error = BuildError::TooManyStates { capacity: 16 };

        assert_eq!(error.to_string(), "an automaton holds at most 16 states");
    }

    #[test]
    fn a_transition_capacity_error_names_its_capacity() {
        let error = BuildError::TooManyTransitions { capacity: 8 };

        assert_eq!(
            error.to_string(),
            "an automaton holds at most 8 transitions"
        );
    }
}
