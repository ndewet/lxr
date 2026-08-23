/// The result of matching at one input offset.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Match<T> {
    /// A rule produced a token of this byte length.
    Token(T, usize),
    /// A generated token together with its absolute start and byte length.
    TokenAt(T, usize, usize),
    /// A rule skipped a match of this byte length.
    Skip(usize),
    /// A matched value could not be constructed.
    Value(usize),
    /// A generated value failure together with its absolute start and byte length.
    ValueAt(usize, usize),
    /// No rule matched.
    None,
    /// No rule matched at this absolute offset.
    NoneAt(usize),
    /// Internal skips consumed the remainder of the input.
    End,
}

impl<T> Match<T> {
    /// Returns the matched byte length, or zero when no rule matched.
    pub fn length(&self) -> usize {
        match self {
            Self::Token(_, length) | Self::Skip(length) | Self::Value(length) => *length,
            Self::TokenAt(_, _, length) | Self::ValueAt(_, length) => *length,
            Self::None | Self::NoneAt(_) | Self::End => 0,
        }
    }
}
