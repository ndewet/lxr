use super::fragment::Fragment;
use crate::automata::{Label, NfaBuilder};
use crate::regex::CharSet;

/// The alphabet that the automaton reads.
///
/// Thompson construction knows the operators of a regular expression. It does
/// not know the alphabet. A [`Class`](crate::regex::Node::Class) leaf is the
/// only place at which the alphabet is applicable. This trait makes the states
/// of that leaf. Thus one construction serves a byte alphabet and a character
/// alphabet.
///
/// The trait varies the output of the leaf, and not its input. A regular
/// expression matches characters, thus a `Class` leaf always holds a
/// [`CharSet`]. An input of a different type, for example a stream of tokens,
/// needs a different syntax tree. It does not need a different alphabet.
pub trait Alphabet {
    /// The label of a transition in this alphabet.
    type Label: Label;

    /// Adds the states that match one character from `set` to `builder`, then
    /// returns them as a fragment.
    ///
    /// One character of `set` moves the automaton from the entry of the
    /// fragment to its exit. No other input does.
    ///
    /// A set that holds no characters gives a fragment that has no path from
    /// the entry to the exit.
    fn lower(&self, set: &CharSet, builder: &mut NfaBuilder<Self::Label>) -> Fragment;
}
