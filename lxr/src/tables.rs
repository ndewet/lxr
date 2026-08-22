use crate::action::Action;
use crate::matched::Matched;

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
/// [`repeats`](Self::repeats) and [`leaves`](Self::leaves) cut that work. A scan reads a run of the
/// bytes that one state reads into itself without a step, and it stops at a state that reads no
/// byte. Each one is a property of the transitions, thus a table without them gives the same
/// tokens.
///
/// # Panics
///
/// The fields are public, because the emitted source builds a `Tables` in a `static`. lxr builds
/// each table that it emits, and it agrees with each of these conditions. A `Tables` that lxr did
/// not build can break them. A scan of it then panics, and a `width` of 0 gives the wrong token
/// with no panic:
///
/// - `width` is not 0.
/// - `next` holds `width` values for each state, thus its length is a multiple of `width`.
/// - `repeats` holds four words for each state, or it is empty.
/// - `leaves` holds one bit for each state, or it is empty.
/// - `accept` holds one value for each state.
/// - Each value of `classes` is below `width`, and each value of `next` is below the state count.
/// - Each value of `accept` is at most the length of `actions`.
/// - `start` is not empty, and each of its values is below the state count.
/// - The `go` of each action is below the length of `start`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tables<'a> {
    /// The class of each byte. Class 0 means that no rule reads the byte at any state.
    pub classes: &'a [u16; 256],
    /// The state that each state and each class goes to, one row of `width` values for each state.
    pub next: &'a [u16],
    /// The bytes that each state reads into itself, as four words for each state, or empty.
    ///
    /// The mask of the state `s` starts at `s * 4`. The bit `b % 64` of the word `b / 64` is 1 if
    /// the state reads the byte `b` into itself. A scan reads a run of such bytes in one loop, and
    /// it reads [`classes`](Self::classes) and [`next`](Self::next) for none of them.
    ///
    /// An empty list holds no bit for any state. The scan then reads one byte at a time, and it
    /// gives the same tokens.
    pub repeats: &'a [u64],
    /// The states that read no byte, as one bit for each state, or empty.
    ///
    /// The bit `s % 64` of the word `s / 64` is 1 if the state `s` reads each byte into the dead
    /// state. The scan stops at such a state, thus it reads no byte to learn that the match ends.
    ///
    /// An empty list holds no bit for any state. The scan then reads one byte more at the end of a
    /// match, and it gives the same tokens.
    pub leaves: &'a [u64],
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
    /// Returns the longest match of `input` at `at`, under the start condition at `condition`.
    ///
    /// This is the scan that reads the tables. [`Lexer::find`](crate::Lexer::find) gives it to a
    /// lexer that emits no scan of its own.
    ///
    /// A step that gives the state that it started from is the start of a run. A rule that reads a
    /// run, for example a name or the text of a string, spends most of its bytes in one such
    /// state. The function then reads the rest of the run in one loop with
    /// [`repeats`](Self::repeats), and it reads no table for those bytes.
    ///
    /// The state of a run gives one accept for the whole run, because a longer run gives a longer
    /// match of the same rule. Thus the loop writes the accept one time, and not one time for each
    /// byte.
    ///
    /// # Panics
    ///
    /// This function panics if `condition` is not a start condition of the tables, or if the
    /// tables disagree with the conditions of [`Tables`].
    // The tables are a constant of each lexer. A call would hide that constant from the caller,
    // and each step would then read the width and each pointer from memory.
    #[inline(always)]
    pub fn find(&self, input: &[u8], at: usize, condition: u16) -> Matched {
        let mut state = self.start[usize::from(condition)];
        let mut read = at;
        let mut accept = 0;
        let mut length = 0;

        while let Some(&byte) = input.get(read) {
            let next = self.step(state, byte);
            if next == 0 {
                break;
            }
            read += 1;

            if next == state {
                let repeats = self.repeats(state);
                while let Some(&byte) = input.get(read) {
                    if repeats[usize::from(byte >> 6)] >> (byte & 63) & 1 == 0 {
                        break;
                    }
                    read += 1;
                }
            } else {
                state = next;
            }

            if let Some(rule) = self.accepts(state) {
                accept = rule + 1;
                length = read - at;
                if self.leaf(state) {
                    break;
                }
            }
        }

        Matched {
            accept,
            length,
            read: read - at,
        }
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

    /// Returns the bytes that `state` reads into itself, as a mask of 256 bits.
    ///
    /// The scan reads a run of those bytes in one loop. Thus it holds the mask in registers, and
    /// it reads no table until the run ends.
    ///
    /// The result holds no bit if [`repeats`](Self::repeats) is empty. The scan then reads one
    /// byte at a time.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not a state of the tables.
    pub fn repeats(&self, state: u16) -> [u64; 4] {
        if self.repeats.is_empty() {
            return [0; 4];
        }

        let start = usize::from(state) * 4;
        [
            self.repeats[start],
            self.repeats[start + 1],
            self.repeats[start + 2],
            self.repeats[start + 3],
        ]
    }

    /// Returns `true` if `state` reads each byte into the dead state.
    ///
    /// A scan that reaches such a state has the whole match. Thus it stops, and it reads no byte
    /// to learn that the match ends.
    ///
    /// The result is `false` for each state if [`leaves`](Self::leaves) is empty.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not a state of the tables.
    pub fn leaf(&self, state: u16) -> bool {
        if self.leaves.is_empty() {
            return false;
        }

        let state = usize::from(state);
        self.leaves[state / 64] >> (state % 64) & 1 != 0
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

#[cfg(test)]
mod tests {
    use super::*;

    static CLASSES: [u16; 256] = [0; 256];
    static NEXT: [u16; 4] = [0, 0, 0, 0];
    static ACCEPT: [u16; 2] = [0, 1];
    static START: [u16; 1] = [1];
    static ACTIONS: [Action; 1] = [Action::token()];

    /// State 1 reads the bytes 64 and 66 into itself, and it reads no other byte.
    static REPEATS: [u64; 8] = [0, 0, 0, 0, 0, 0b101, 0, 0];

    /// State 1 reads no byte.
    static LEAVES: [u64; 1] = [0b10];

    /// Returns tables that hold the masks of the runs and the leaves.
    fn tables() -> Tables<'static> {
        Tables {
            classes: &CLASSES,
            next: &NEXT,
            repeats: &REPEATS,
            leaves: &LEAVES,
            width: 2,
            accept: &ACCEPT,
            start: &START,
            actions: &ACTIONS,
        }
    }

    /// Returns tables that hold neither mask.
    fn bare() -> Tables<'static> {
        Tables {
            repeats: &[],
            leaves: &[],
            ..tables()
        }
    }

    #[test]
    fn a_state_gives_the_bytes_that_it_reads_into_itself() {
        assert_eq!(tables().repeats(1), [0, 0b101, 0, 0]);
        assert_eq!(tables().repeats(0), [0; 4]);
    }

    #[test]
    fn a_table_of_no_run_gives_no_byte() {
        assert_eq!(bare().repeats(1), [0; 4]);
    }

    #[test]
    fn a_state_that_reads_no_byte_is_a_leaf() {
        assert!(tables().leaf(1));
        assert!(!tables().leaf(0));
    }

    #[test]
    fn a_table_of_no_leaf_gives_no_leaf() {
        assert!(!bare().leaf(1));
    }
}
