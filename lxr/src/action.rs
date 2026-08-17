/// What a lexer does when one rule matches.
///
/// [`Tables::accept`](crate::Tables::accept) gives the rule that a state accepts, and
/// [`Tables::actions`](crate::Tables::actions) holds one `Action` for each rule.
///
/// The fields are public, because the emitted source builds each `Action` in a `static`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Action {
    /// `true` if the lexer reads the match and gives no token.
    ///
    /// A rule that reads a space or a comment carries this. The scan moves forward by the length of
    /// the match, then it reads the next token.
    pub skip: bool,
    /// The start condition that the scan changes to, or `None` if it keeps the condition.
    ///
    /// The condition changes after the match. Thus a rule that opens a string reads the quote under
    /// the condition of the code, and the scan reads the next token under the condition of the
    /// string.
    pub go: Option<u16>,
}

impl Action {
    /// Creates an action that gives a token and keeps the start condition.
    pub const fn token() -> Self {
        Self {
            skip: false,
            go: None,
        }
    }

    /// Creates an action that gives no token and keeps the start condition.
    pub const fn skip() -> Self {
        Self {
            skip: true,
            go: None,
        }
    }

    /// Returns this action with the start condition that it changes to.
    pub const fn going(self, condition: u16) -> Self {
        Self {
            skip: self.skip,
            go: Some(condition),
        }
    }
}
