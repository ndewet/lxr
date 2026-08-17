use crate::action::Action;

/// The automaton of a lexer, as the tables that a scan reads.
///
/// The derive macro emits one static for each field, then it emits a `Tables` that refers to them.
/// Thus the tables live in the read only data of the program, and a scan makes no allocation.
///
/// State 0 is the dead state, and class 0 is the dead class. A scan stops when it reaches state 0.
///
/// A step reads [`classes`](Self::classes) with the byte, then it reads [`next`](Self::next) at the
/// state and that class:
///
/// ```text
/// state = next[state * width + classes[byte]]
/// ```
///
/// # Panics
///
/// The fields are public, because the emitted source builds a `Tables` in a `static`. lxr builds
/// each table that it emits, and it agrees with each of these conditions. A `Tables` that lxr did
/// not build can break them, and a scan of it panics:
///
/// - `next` holds `width` values for each state, thus its length is a multiple of `width`.
/// - `accept` holds one value for each state.
/// - Each value of `classes` is below `width`, and each value of `next` is below the state count.
/// - Each value of `accept` is at most the length of `actions`.
/// - `start` is not empty, and each of its values is below the state count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tables<'a> {
    /// The class of each byte. Class 0 means that no rule reads the byte at any state.
    pub classes: &'a [u16; 256],
    /// The state that each state and each class goes to, one row of `width` values for each state.
    pub next: &'a [u16],
    /// The number of the columns of one row of [`next`](Self::next).
    pub width: usize,
    /// The rule that each state accepts, plus one, or 0 if the state does not accept.
    pub accept: &'a [u16],
    /// The state at which each start condition begins a scan.
    pub start: &'a [u16],
    /// What the lexer does for each rule that matches.
    pub actions: &'a [Action],
}

impl Tables<'_> {
    /// Returns the number of the states, the dead state included.
    pub fn state_count(&self) -> usize {
        self.accept.len()
    }

    /// Returns the number of the start conditions.
    pub fn condition_count(&self) -> usize {
        self.start.len()
    }

    /// Returns the state at which `state` reads `byte`, or 0 if the scan stops there.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not a state of the tables, or if the tables disagree with
    /// the conditions of [`Tables`].
    pub fn step(&self, state: u16, byte: u8) -> u16 {
        let class = usize::from(self.classes[usize::from(byte)]);
        self.next[usize::from(state) * self.width + class]
    }

    /// Returns the rule that `state` accepts, or `None` if the state does not accept.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not a state of the tables.
    pub fn accepts(&self, state: u16) -> Option<u16> {
        match self.accept[usize::from(state)] {
            0 => None,
            rule => Some(rule - 1),
        }
    }
}
