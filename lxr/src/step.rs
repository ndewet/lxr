use crate::run::Run;

/// What one step of a lexer found at one offset of the input.
///
/// The emitted source of a lexer writes it, and [`Scan`](crate::Scan) reads it. Thus a step builds
/// no value and it reads no table.
///
/// The fields are public, because the emitted source writes each one.
///
/// The step also carries [`run`](Self::run), which one scan keeps across its steps. Thus a `Step`
/// belongs to one scan of one input, and two scans need two of them.
#[derive(Debug, Clone, Copy)]
pub struct Step<T> {
    /// What the rule that won gives.
    pub outcome: Outcome<T>,
    /// The number of the bytes of the match, or 0 if no rule matched.
    pub length: usize,
    /// The number of the bytes that the step read, the bytes after the match included.
    ///
    /// The value is at or below the true one. A step that reads a sequence of bytes a second time
    /// counts the second read alone, thus the record of [`Scan`](crate::Scan) starts later and it
    /// stays correct.
    pub read: usize,
    /// The start condition under which the scan reads the next token.
    ///
    /// The scan writes the condition of this step, and the step keeps it unless the rule that won
    /// changes it.
    pub condition: u16,
    /// The run of bytes that one node read last.
    ///
    /// A node that reads a run and that ends no match writes this record. A later step that
    /// reaches that node inside the run reads the record, and it stops where the run stopped.
    pub run: Run,
}

/// What the rule that won one step gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome<T> {
    /// The rule gives this token.
    Token(T),
    /// The rule reads its match and gives no token, for example a rule that reads a space.
    Skip,
    /// The rule gives a token that holds a field, and the text of the match does not fit that
    /// field.
    Value,
    /// No rule matched.
    None,
}

impl<T> Step<T> {
    /// Creates the step of a scan that starts under the start condition at `condition`.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::{Outcome, Step};
    ///
    /// let step = Step::<()>::new(0);
    ///
    /// assert_eq!(step.outcome, Outcome::None);
    /// assert_eq!(step.length, 0);
    /// ```
    pub const fn new(condition: u16) -> Self {
        Self {
            outcome: Outcome::None,
            length: 0,
            read: 0,
            condition,
            run: Run::new(),
        }
    }

    /// Takes the outcome of the step, and leaves [`Outcome::None`] in its place.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::{Outcome, Step};
    ///
    /// let mut step = Step::new(0);
    /// step.outcome = Outcome::Token(7);
    ///
    /// assert_eq!(step.take(), Outcome::Token(7));
    /// assert_eq!(step.outcome, Outcome::None);
    /// ```
    pub fn take(&mut self) -> Outcome<T> {
        std::mem::replace(&mut self.outcome, Outcome::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_step_holds_the_start_condition_and_no_outcome() {
        let step = Step::<u8>::new(3);

        assert_eq!(step.condition, 3);
        assert_eq!(step.outcome, Outcome::None);
        assert_eq!((step.length, step.read), (0, 0));
    }

    #[test]
    fn taking_the_outcome_leaves_none_in_its_place() {
        let mut step = Step::new(0);
        step.outcome = Outcome::Token("word");

        assert_eq!(step.take(), Outcome::Token("word"));
        assert_eq!(step.take(), Outcome::None);
    }
}
