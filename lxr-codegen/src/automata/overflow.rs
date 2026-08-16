use std::fmt::{Display, Formatter, Result};

/// A build that asks for more than an automaton holds.
///
/// A ceiling is a limit of the automaton, and not a defect in the caller. Thus a build reports the
/// overflow, and it does not panic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Overflow {
    /// The part of the automaton that is full.
    pub part: Part,
    /// The maximum number of the items in that part.
    pub maximum: usize,
}

impl Overflow {
    /// Creates an `Overflow` that reports `part` as full at `maximum` items.
    pub(crate) fn new(part: Part, maximum: usize) -> Self {
        Self { part, maximum }
    }
}

impl Display for Overflow {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        write!(
            formatter,
            "an automaton holds at most {} {}",
            self.maximum, self.part
        )
    }
}

impl std::error::Error for Overflow {}

/// The part that an [`Overflow`] reports.
///
/// A part belongs to an automaton, or to the table that an emitted lexer reads. The two hold the
/// same states, and the table numbers them in a narrower type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Part {
    /// The states of the automaton.
    States,
    /// The items of one [`Arena`](super::Arena), for example the transitions.
    Items,
    /// The rules that the table of a lexer numbers.
    Rules,
}

impl Display for Part {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        match self {
            Self::States => write!(formatter, "states"),
            Self::Items => write!(formatter, "items"),
            Self::Rules => write!(formatter, "rules"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_overflow_names_the_part_that_is_full_and_its_maximum() {
        let overflow = Overflow::new(Part::States, 16);

        assert_eq!(overflow.to_string(), "an automaton holds at most 16 states");
    }

    #[test]
    fn an_overflow_of_the_items_of_an_arena_names_the_items() {
        let overflow = Overflow::new(Part::Items, 8);

        assert_eq!(overflow.to_string(), "an automaton holds at most 8 items");
    }
}
