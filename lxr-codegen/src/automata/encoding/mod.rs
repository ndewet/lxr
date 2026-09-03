//! Encodes character sets for the alphabet that an automaton reads.
//!
//! A regular expression matches Unicode scalar values. A generated automaton
//! reads bytes. An encoding connects these two domains. It maps one
//! [`Class`](crate::regex::Expression::Class) leaf to label sequences.
//!
//! [`Encoding`] defines that operation. [`Utf8`] implements it for UTF-8.
//! [`utf8::encode`] gives the byte sequences of one character set.
//!
//! [Thompson construction](super::nfa::thompson) reads this trait. Thus the
//! construction knows the operators of a regular expression. It does not know
//! how characters map to transition labels.

pub mod utf8;

pub use self::utf8::{ByteRange, ByteSequence, Utf8};

use super::label::Label;
use crate::regex::CharSet;

/// An encoding from character sets to NFA transition labels.
///
/// Thompson construction knows the operators of a regular expression. It does
/// not know the target encoding. A
/// [`Class`](crate::regex::Expression::Class) leaf is the only place at which
/// an encoding is applicable. This trait gives the sequences for that leaf.
/// Thompson construction makes their states and transitions.
///
/// The trait varies the output of the leaf, and not its input. A regular
/// expression matches characters, thus a `Class` leaf always holds a
/// [`CharSet`]. An input of a different type, for example a stream of tokens,
/// needs a different syntax tree. It does not need a different encoding.
pub trait Encoding {
    /// The label of a transition in the encoded automaton.
    type Label: Label;

    /// One non-empty sequence of transition labels.
    type Sequence: AsRef<[Self::Label]>;

    /// Encodes `set` as an alternation of label sequences.
    ///
    /// The sequences together match exactly the encodings of the characters
    /// in `set`. Each sequence must contain at least one label. An empty set
    /// gives no sequence.
    fn encode(&self, set: &CharSet) -> Vec<Self::Sequence>;
}
