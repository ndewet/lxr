/// The result of matching at one input offset.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Match<T> {
    /// A rule produced a token of this byte length.
    Token(T, usize),
    /// A rule skipped a match of this byte length.
    Skip(usize),
    /// A matched value could not be constructed.
    Value(usize),
    /// No rule matched.
    None,
}

impl<T> Match<T> {
    /// Returns the matched byte length, or zero when no rule matched.
    pub fn length(&self) -> usize {
        match self {
            Self::Token(_, length) | Self::Skip(length) | Self::Value(length) => *length,
            Self::None => 0,
        }
    }
}
