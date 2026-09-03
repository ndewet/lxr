use std::fmt::{Display, Formatter};

/// An error from an automaton builder.
///
/// Each variant identifies the storage that reached its capacity. The
/// capacity is a limit of the input, and not a defect in the builder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BuildError {
    /// The build needs more states than the state storage can hold.
    TooManyStates {
        /// The number of states that the storage can hold.
        capacity: usize,
    },
    /// The build needs more transitions than the transition storage can hold.
    TooManyTransitions {
        /// The number of transitions that the storage can hold.
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
