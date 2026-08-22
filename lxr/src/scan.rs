use std::fmt::{Debug, Formatter, Result as FormatResult};
use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::ops::Range;

use crate::error::ScanError;
use crate::lexer::Lexer;
use crate::located::Locations;

/// One scan of an input, in progress.
///
/// The scan gives one token at a time, thus it implements [`Iterator`]. It also holds where the
/// last token is. Read [`span`](Self::span), [`slice`](Self::slice), [`line`](Self::line), and
/// [`column`](Self::column) after each token.
///
/// A rule that skips gives no token, thus it moves none of the four. They hold the last token or
/// the last fault, and a space after that token does not move them.
///
/// A `for` loop takes the scan, thus the body of the loop cannot read the place of the token. Use
/// [`located`](Self::located) for that loop, and it gives the place with the token.
///
/// An offset counts bytes from the start of the input, and a token holds no borrow of the input.
///
/// A scan of an input that the rules match makes no allocation, and it reads each byte one time. A
/// region that no rule ends is different. The scan reads such a region again at each start
/// position, thus it records the states that gave no accept, and it stops at a state that it
/// recorded. The record needs memory, thus a scan of such a region makes one allocation of one
/// megabyte at most.
///
/// To make a `Scan`, use [`Lexer::scan`].
pub struct Scan<'a, T> {
    input: &'a str,
    offset: usize,
    condition: u16,
    /// The line at which the last token starts.
    line: u32,
    /// The column at which the last token starts.
    column: u32,
    span: Range<usize>,
    /// The line at which the next token starts.
    next_line: u32,
    /// The column at which the next token starts.
    next_column: u32,
    /// One bit for each state at each position, or empty until a region needs the record.
    memo: Vec<u64>,
    /// The offset that the first position of [`memo`](Self::memo) holds.
    base: usize,
    /// The number of the words of one position of [`memo`](Self::memo).
    words: usize,
    /// The furthest offset that a match of this scan reached.
    furthest: usize,
    token: PhantomData<fn() -> T>,
}

/// The place at which the input after a match starts.
///
/// The scan counts the line and the column while it steps the automaton, thus it carries the place
/// of a match with that match. A [`Scan`] holds no other line and no other column of the input.
#[derive(Debug, Clone, Copy)]
struct Place {
    /// The line, counted from 1.
    line: u32,
    /// The column in characters, counted from 1.
    column: u32,
}

impl Place {
    /// Counts `byte` into the place.
    ///
    /// A byte of the form `10xxxxxx` continues a character, thus the column does not count it.
    #[inline]
    fn read(&mut self, byte: u8) {
        if byte == b'\n' {
            self.line += 1;
            self.column = 1;
        } else if byte & 0xC0 != 0x80 {
            self.column += 1;
        }
    }
}

/// The rule that won the longest match, the bytes of that match, and the place after it.
#[derive(Debug, Clone, Copy)]
struct Matched {
    /// The index of the rule that matched.
    rule: u16,
    /// The number of the bytes of the match.
    length: usize,
    /// The place at which the input after the match starts.
    place: Place,
}

/// The number of the bytes that a scan reads again before it makes the record of its states.
///
/// A scan of an input that each rule matches stays below this limit, thus it makes no allocation.
#[cfg(not(test))]
const MEMO_THRESHOLD: usize = 4096;

/// The record starts after the first match of a test, thus each test reads both paths.
#[cfg(test)]
const MEMO_THRESHOLD: usize = 1;

/// The maximum number of the bytes of the record of the states.
///
/// A region above this size gives the record a new base, and the record then holds the region that
/// follows that base.
const MEMO_LIMIT: usize = 1 << 20;

impl<'a, T: Lexer> Scan<'a, T> {
    /// Creates a scan of `input` under the first start condition.
    pub(crate) fn new(input: &'a str) -> Self {
        Self {
            input,
            offset: 0,
            condition: 0,
            line: 1,
            column: 1,
            span: 0..0,
            next_line: 1,
            next_column: 1,
            memo: Vec::new(),
            base: 0,
            words: 0,
            furthest: 0,
            token: PhantomData,
        }
    }

    /// Returns the bytes of the last token, counted from the start of the input.
    ///
    /// The result is `0..0` before the first token.
    pub fn span(&self) -> Range<usize> {
        self.span.clone()
    }

    /// Returns the text of the last token.
    ///
    /// The result is empty before the first token. After a [`ScanError`], it is the text at fault.
    pub fn slice(&self) -> &'a str {
        &self.input[self.span.clone()]
    }

    /// Gives the place of each token with the token, in place of the token alone.
    ///
    /// A `for` loop takes the scan, thus the body of the loop cannot read [`span`](Self::span) or
    /// [`line`](Self::line). Each [`Located`](crate::Located) of this iterator carries them.
    ///
    /// # Examples
    ///
    #[cfg_attr(feature = "derive", doc = "```")]
    #[cfg_attr(not(feature = "derive"), doc = "```ignore")]
    /// use lxr::Lexer;
    ///
    /// #[derive(Debug, PartialEq, Lexer)]
    /// #[lxr(skip = " +")]
    /// enum Token {
    ///     #[lxr(regex = "[a-z]+")]
    ///     Word,
    /// }
    ///
    /// let places: Vec<_> = Token::scan("one two")
    ///     .located()
    ///     .map(|found| found.expect("each character belongs to a token"))
    ///     .map(|found| (found.span, found.line, found.column))
    ///     .collect();
    ///
    /// assert_eq!(places, vec![(0..3, 1, 1), (4..7, 1, 5)]);
    /// ```
    pub fn located(self) -> Locations<'a, T> {
        Locations::new(self)
    }

    /// Returns the line at which the last token starts, counted from 1.
    pub fn line(&self) -> u32 {
        self.line
    }

    /// Returns the column at which the last token starts, counted from 1.
    ///
    /// The column counts characters, and not bytes. Thus a character above ASCII counts as one.
    pub fn column(&self) -> u32 {
        self.column
    }

    /// Returns the start condition under which the scan reads the next token.
    ///
    /// A rule changes the condition after it matches. Thus this is the condition of the next token,
    /// and not the condition of the last one.
    ///
    /// # Panics
    ///
    /// This function panics if the tables name a condition that the lexer does not hold.
    pub fn condition(&self) -> T::Condition {
        T::condition(self.condition)
    }

    /// Returns the offset at which the next token starts.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Returns the input that the scan has not read.
    pub fn remainder(&self) -> &'a str {
        &self.input[self.offset..]
    }

    /// Moves the scan forward by `length` bytes, and puts the next token at `place`.
    ///
    /// [`longest_match`](Self::longest_match) counts the place while it steps the automaton, thus
    /// this function reads no byte.
    fn bump(&mut self, length: usize, place: Place) {
        self.offset += length;
        self.next_line = place.line;
        self.next_column = place.column;
    }

    /// Returns the number of the bytes of the character at the offset of the scan, and the place
    /// of the input after that character.
    ///
    /// The scan calls this function for a character that no rule matches. That character is one
    /// column, or it is a newline.
    ///
    /// # Panics
    ///
    /// This function panics if the offset of the scan is at the end of the input.
    fn faulted(&self) -> (usize, Place) {
        let bytes = self.input.as_bytes();
        let mut length = 1;
        while self.offset + length < bytes.len() && bytes[self.offset + length] & 0xC0 == 0x80 {
            length += 1;
        }

        let place = if bytes[self.offset] == b'\n' {
            Place {
                line: self.next_line + 1,
                column: 1,
            }
        } else {
            Place {
                line: self.next_line,
                column: self.next_column + 1,
            }
        };
        (length, place)
    }

    /// Records the last token as the `length` bytes at the offset of the scan, then moves forward.
    ///
    /// A rule that skips its match gives no token. Thus it calls [`bump`](Self::bump) alone, and
    /// the place of the last token stays where it is.
    fn take(&mut self, length: usize, place: Place) {
        self.line = self.next_line;
        self.column = self.next_column;
        self.span = self.offset..self.offset + length;
        self.bump(length, place);
    }

    /// Returns the longest match at the offset of the scan.
    ///
    /// The scan keeps the last accept that it reached, thus a rule that matches a longer input
    /// wins, and the earliest rule wins a tie. Each rule of a lexer matches at least one byte,
    /// thus a match of no length gives `None`.
    ///
    /// The function counts the line and the column of each byte that it steps, and it keeps the
    /// place of the last accept with that accept. Thus the scan reads each byte of a match one
    /// time, and [`bump`](Self::bump) reads none of them again.
    ///
    /// The scan stops at the dead state, at the end of the input, or at a state that a match of an
    /// earlier offset recorded. A record states that the same state at the same position gave no
    /// accept after it, thus the rest of that scan gives no accept here.
    ///
    /// # Panics
    ///
    /// This function panics if the tables disagree with the conditions of [`Tables`](crate::Tables).
    fn longest_match(&mut self) -> Option<Matched> {
        let tables = T::TABLES;
        let start = tables.start[usize::from(self.condition)];

        self.prepare();
        let (best, read) = if self.memo.is_empty() {
            self.run_match(start)
        } else {
            self.recorded_match(start)
        };

        self.furthest = self.furthest.max(self.offset + read);
        self.record(start, best.map_or(0, |matched| matched.length), read);
        best
    }

    /// Returns the longest match from `start`, and the number of the bytes that it read.
    ///
    /// A step that gives the state that it started from is the start of a run. A rule that reads a
    /// run, for example a name or the text of a string, spends most of its bytes in one such
    /// state. The function then reads the rest of the run in one loop with
    /// [`Tables::repeats`](crate::Tables::repeats), and it reads no table for those bytes.
    ///
    /// The state of a run gives one accept for the whole run, because a longer run gives a longer
    /// match of the same rule. Thus the loop writes the accept one time, and not one time for each
    /// byte.
    ///
    /// The record of the states is empty here. [`recorded_match`](Self::recorded_match) reads a
    /// region that needs that record.
    ///
    /// # Panics
    ///
    /// This function panics if the tables disagree with the conditions of [`Tables`](crate::Tables).
    fn run_match(&self, start: u16) -> (Option<Matched>, usize) {
        let tables = T::TABLES;
        let bytes = self.input.as_bytes();
        let mut state = start;
        let mut read = self.offset;
        let mut best = None;
        let mut place = self.place();

        while let Some(&byte) = bytes.get(read) {
            let next = tables.step(state, byte);
            if next == 0 {
                break;
            }
            place.read(byte);
            read += 1;

            if next == state {
                let repeats = tables.repeats(state);
                while let Some(&byte) = bytes.get(read) {
                    if repeats[usize::from(byte >> 6)] >> (byte & 63) & 1 == 0 {
                        break;
                    }
                    place.read(byte);
                    read += 1;
                }
            } else {
                state = next;
            }

            if let Some(rule) = tables.accepts(state) {
                best = Some(Matched {
                    rule,
                    length: read - self.offset,
                    place,
                });
                if tables.leaf(state) {
                    break;
                }
            }
        }

        (best, read - self.offset)
    }

    /// Returns the longest match from `start`, and the number of the bytes that it read.
    ///
    /// The function reads one byte at a time, and it stops at a state that a match of an earlier
    /// offset recorded. A record states that the same state at the same position gave no accept
    /// after it, thus the rest of that scan gives no accept here.
    ///
    /// # Panics
    ///
    /// This function panics if the tables disagree with the conditions of [`Tables`](crate::Tables).
    fn recorded_match(&self, start: u16) -> (Option<Matched>, usize) {
        let tables = T::TABLES;
        let mut state = start;
        let mut best = None;
        let mut read = 0;
        let mut place = self.place();

        for (index, &byte) in self.input.as_bytes()[self.offset..].iter().enumerate() {
            state = tables.step(state, byte);
            if state == 0 {
                break;
            }
            place.read(byte);
            read = index + 1;
            if let Some(rule) = tables.accepts(state) {
                best = Some(Matched {
                    rule,
                    length: read,
                    place,
                });
            }
            if self.recorded(state, self.offset + read) {
                break;
            }
        }

        (best, read)
    }

    /// Returns the place at which the next token starts.
    fn place(&self) -> Place {
        Place {
            line: self.next_line,
            column: self.next_column,
        }
    }

    /// Makes the record of the states, or moves it, if the scan reads a region a second time.
    ///
    /// The record holds [`MEMO_LIMIT`] bytes at most, and the input needs no more than one
    /// position for each byte. An offset outside the record moves the base to that offset, and the
    /// record then holds the region that follows it.
    fn prepare(&mut self) {
        if self.furthest.saturating_sub(self.offset) < MEMO_THRESHOLD {
            return;
        }

        if self.memo.is_empty() {
            self.words = T::TABLES.accept.len().div_ceil(64).max(1);
            let limit = MEMO_LIMIT / (self.words * 8);
            let positions = limit.min(self.input.len() - self.offset + 1).max(1);
            self.memo = vec![0; positions * self.words];
            self.base = self.offset;
        } else if !self.holds(self.offset) {
            self.memo.fill(0);
            self.base = self.offset;
        }
    }

    /// Returns whether the record holds `position`.
    fn holds(&self, position: usize) -> bool {
        self.words != 0
            && position >= self.base
            && (position - self.base) * self.words < self.memo.len()
    }

    /// Returns the word of `state` at `position`, and the bit of that state in the word.
    fn bit(&self, state: u16, position: usize) -> Option<(usize, u64)> {
        if !self.holds(position) {
            return None;
        }

        let state = usize::from(state);
        let word = (position - self.base) * self.words + state / 64;
        Some((word, 1 << (state % 64)))
    }

    /// Returns whether a match of an earlier offset reached `state` at `position` and gave no
    /// accept after it.
    fn recorded(&self, state: u16, position: usize) -> bool {
        self.bit(state, position)
            .is_some_and(|(word, mask)| self.memo[word] & mask != 0)
    }

    /// Records each state that the match of `read` bytes from `start` reached after `keep` bytes.
    ///
    /// The match gave its last accept at `keep` bytes, thus each state after that one gave no
    /// accept. The match reads the same bytes a second time, because it holds the states of the
    /// first time nowhere.
    ///
    /// # Panics
    ///
    /// This function panics if the tables disagree with the conditions of [`Tables`](crate::Tables).
    fn record(&mut self, start: u16, keep: usize, read: usize) {
        if self.memo.is_empty() || read <= keep {
            return;
        }

        let tables = T::TABLES;
        let mut state = start;
        for index in 0..read {
            state = tables.step(state, self.input.as_bytes()[self.offset + index]);
            if index < keep {
                continue;
            }
            if let Some((word, mask)) = self.bit(state, self.offset + index + 1) {
                self.memo[word] |= mask;
            }
        }
    }
}

impl<T: Lexer> Iterator for Scan<'_, T> {
    type Item = std::result::Result<T, ScanError>;

    fn next(&mut self) -> Option<Self::Item> {
        let tables = T::TABLES;

        loop {
            if self.offset >= self.input.len() {
                return None;
            }

            let Some(matched) = self.longest_match() else {
                let (length, place) = self.faulted();
                self.take(length, place);
                return Some(Err(ScanError::no_rule(
                    self.span.clone(),
                    self.line,
                    self.column,
                )));
            };

            let action = tables.actions[usize::from(matched.rule)];
            if let Some(condition) = action.go {
                self.condition = condition;
            }
            if action.skip {
                self.bump(matched.length, matched.place);
                continue;
            }

            self.take(matched.length, matched.place);
            let text = if T::READS_TEXT { self.slice() } else { "" };
            let value = T::token(matched.rule, text);
            return Some(
                value.ok_or_else(|| ScanError::value(self.span.clone(), self.line, self.column)),
            );
        }
    }
}

impl<T: Lexer> FusedIterator for Scan<'_, T> {}

impl<T> Debug for Scan<'_, T> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
        formatter
            .debug_struct("Scan")
            .field("offset", &self.offset)
            .field("condition", &self.condition)
            .field("line", &self.line)
            .field("column", &self.column)
            .field("span", &self.span)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Action;
    use crate::tables::Tables;

    /// The token of the lexer of the tests.
    #[derive(Debug, PartialEq, Eq)]
    enum Token {
        /// One `a`.
        One,
        /// A run of `a` that a `b` ends.
        Many,
    }

    /// The class of each byte: 1 for `a`, 2 for `b`, and 0 for each other byte.
    static CLASSES: [u16; 256] = {
        let mut classes = [0; 256];
        classes[b'a' as usize] = 1;
        classes[b'b' as usize] = 2;
        classes
    };

    /// The states of the rules `a` and `a+b`.
    ///
    /// State 1 is the start. State 2 reads the first `a`, and it accepts the rule `a`. State 4
    /// reads each `a` after that one, and it accepts nothing. State 3 reads the `b` that ends the
    /// run, and it accepts the rule `a+b`.
    static NEXT: [u16; 15] = [
        0, 0, 0, //
        0, 2, 0, //
        0, 4, 3, //
        0, 0, 0, //
        0, 4, 3,
    ];

    /// State 4 reads the letter `a` into itself, thus a run of `a` after the first one is one run.
    static REPEATS: [u64; 20] = {
        let mut repeats = [0; 20];
        repeats[4 * 4 + 1] = 1 << (b'a' & 63);
        repeats
    };

    /// State 3 reads no byte, thus a scan that reaches it has the whole run.
    static LEAVES: [u64; 1] = [1 << 3];

    static ACCEPT: [u16; 5] = [0, 0, 1, 2, 0];
    static START: [u16; 1] = [1];
    static ACTIONS: [Action; 2] = [Action::token(), Action::token()];

    impl Lexer for Token {
        type Condition = ();

        const TABLES: Tables<'static> = Tables {
            classes: &CLASSES,
            next: &NEXT,
            repeats: &REPEATS,
            leaves: &LEAVES,
            width: 3,
            accept: &ACCEPT,
            start: &START,
            actions: &ACTIONS,
        };

        fn token(rule: u16, _text: &str) -> Option<Self> {
            match rule {
                0 => Some(Self::One),
                1 => Some(Self::Many),
                other => panic!("the lexer holds no rule {other}"),
            }
        }

        fn condition(_index: u16) {}
    }

    /// Returns each token of a scan of `input`, and `None` for each fault.
    fn steps(input: &str) -> Vec<Option<Token>> {
        Token::scan(input).map(Result::ok).collect()
    }

    #[test]
    fn a_run_that_no_b_ends_gives_one_token_for_each_a() {
        let input = "a".repeat(2000);

        let found = steps(&input);

        assert_eq!(found.len(), 2000);
        assert!(
            found
                .iter()
                .all(|token| token.as_ref() == Some(&Token::One))
        );
    }

    #[test]
    fn a_run_that_a_b_ends_gives_one_token_of_the_whole_run() {
        let mut input = "a".repeat(2000);
        input.push('b');

        assert_eq!(steps(&input), vec![Some(Token::Many)]);
    }

    #[test]
    fn a_run_of_a_reads_the_b_of_a_later_run() {
        let input = format!("{}b{}b", "a".repeat(500), "a".repeat(500));

        assert_eq!(steps(&input), vec![Some(Token::Many), Some(Token::Many)]);
    }

    #[test]
    fn a_byte_that_no_rule_matches_gives_a_fault_between_two_runs() {
        let input = format!("{}c{}b", "a".repeat(300), "a".repeat(300));

        let found = steps(&input);

        assert_eq!(found.len(), 302);
        assert_eq!(found[300], None);
        assert_eq!(found[301], Some(Token::Many));
    }

    #[test]
    fn a_newline_that_no_rule_matches_moves_the_line() {
        let mut scan = Token::scan("a\na");

        assert_eq!(scan.next(), Some(Ok(Token::One)));
        assert_eq!((scan.line(), scan.column()), (1, 1));

        let error = scan
            .next()
            .expect("the scan gives one result for the newline")
            .expect_err("no rule matches a newline");
        assert_eq!((error.line, error.column), (1, 2));

        assert_eq!(scan.next(), Some(Ok(Token::One)));
        assert_eq!((scan.line(), scan.column()), (2, 1));
    }

    #[test]
    fn the_record_holds_the_states_of_a_region_that_the_scan_reads_again() {
        let input = "a".repeat(3000);
        let mut scan = Token::scan(&input);

        assert!(scan.next().is_some());
        assert!(scan.memo.is_empty());

        assert!(scan.next().is_some());
        assert!(!scan.memo.is_empty());
    }
}
