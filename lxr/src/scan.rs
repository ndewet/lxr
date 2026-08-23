use std::cell::Cell;
use std::fmt::{Debug, Formatter, Result as FormatResult};
use std::iter::FusedIterator;
use std::ops::Range;

use crate::error::ScanError;
use crate::lexer::Lexer;
use crate::located::Locations;
use crate::step::{Outcome, Step};

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
/// A scan makes no allocation, and it reads each byte of a match one time.
/// [`line`](Self::line) and [`column`](Self::column) count the characters before the last token,
/// thus a caller that reads them reads those bytes a second time. The scan holds the place that it
/// counted, and it counts each byte of the input one time however many tokens the caller places.
/// That record is a [`Cell`], thus a `Scan` is not [`Sync`].
///
/// A region that no rule ends is different. A rule that needs a byte far ahead keeps the graph
/// alive to that byte, thus the scan reads such a region again at each start position. A node that
/// reads a run of bytes keeps the run that it read, and a later step that reaches that node inside
/// the run stops where the run stopped. Thus such a region costs its length. A region whose cycle
/// spans more than one node holds no such run, and it costs the square of its length.
///
/// To make a `Scan`, use [`Lexer::scan`].
pub struct Scan<'a, T> {
    input: &'a str,
    offset: usize,
    span: Range<usize>,
    /// The place of one offset of the input, which [`Scan::place`] moves forward.
    place: Cell<Cursor>,
    /// The result of the last step, and the start condition of the next one.
    step: Step<T>,
}

/// The line and the column of one offset of the input.
///
/// The scan counts no place while it matches. It holds the place of one offset instead, and it
/// counts forward from that offset when a caller asks for the place of a token. A token comes
/// after the token before it, thus the offset only moves forward and the scan counts each byte of
/// the input one time.
#[derive(Debug, Clone, Copy)]
struct Cursor {
    /// The offset that the line and the column belong to.
    offset: usize,
    /// The line, counted from 1.
    line: u32,
    /// The column in characters, counted from 1.
    column: u32,
}

impl<'a, T> Scan<'a, T> {
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

    /// Returns the line at which the last token starts, counted from 1.
    ///
    /// The scan counts the characters before the last token, thus it reads those bytes a second
    /// time. It counts each byte of the input one time however many tokens a caller places.
    pub fn line(&self) -> u32 {
        self.place().0
    }

    /// Returns the column at which the last token starts, counted from 1.
    ///
    /// The column counts characters, and not bytes. Thus a character above ASCII counts as one.
    pub fn column(&self) -> u32 {
        self.place().1
    }

    /// Returns the offset at which the next token starts.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Returns the input that the scan has not read.
    pub fn remainder(&self) -> &'a str {
        &self.input[self.offset..]
    }

    /// Returns the line and the column at which the last token starts.
    ///
    /// The scan counts forward from the place that it holds, then it keeps the new place. A token
    /// comes after the token before it, thus the count reads each byte of the input one time.
    fn place(&self) -> (u32, u32) {
        let mut cursor = self.place.get();
        if cursor.offset < self.span.start {
            for &byte in &self.input.as_bytes()[cursor.offset..self.span.start] {
                if byte == b'\n' {
                    cursor.line += 1;
                    cursor.column = 1;
                } else if byte & 0xC0 != 0x80 {
                    cursor.column += 1;
                }
            }
            cursor.offset = self.span.start;
            self.place.set(cursor);
        }

        (cursor.line, cursor.column)
    }

    /// Returns the number of the bytes of the character at the offset of the scan.
    ///
    /// The scan calls this function for a character that no rule matches.
    fn faulted(&self) -> usize {
        let bytes = self.input.as_bytes();
        let mut length = 1;
        while self.offset + length < bytes.len() && bytes[self.offset + length] & 0xC0 == 0x80 {
            length += 1;
        }
        length
    }

    /// Records the last token as the `length` bytes at `at`, then moves forward.
    ///
    /// A rule that skips its match gives no token. Thus it moves the offset alone, and the place
    /// of the last token stays where it is.
    fn take(&mut self, at: usize, length: usize) {
        self.span = at..at + length;
        self.offset = at + length;
    }
}

impl<'a, T: Lexer> Scan<'a, T> {
    /// Creates a scan of `input` under the first start condition.
    pub(crate) fn new(input: &'a str) -> Self {
        Self {
            input,
            offset: 0,
            span: 0..0,
            place: Cell::new(Cursor {
                offset: 0,
                line: 1,
                column: 1,
            }),
            step: Step::new(0),
        }
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

    /// Returns the start condition under which the scan reads the next token.
    ///
    /// A rule changes the condition after it matches. Thus this is the condition of the next token,
    /// and not the condition of the last one.
    ///
    /// # Panics
    ///
    /// This function panics if the lexer names a condition that it does not hold.
    pub fn condition(&self) -> T::Condition {
        T::condition(self.step.condition)
    }
}

impl<T: Lexer> Iterator for Scan<'_, T> {
    type Item = std::result::Result<T, ScanError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.offset >= self.input.len() {
                return None;
            }

            let at = self.offset;
            T::step(self.input, at, &mut self.step);
            let length = self.step.length;

            match self.step.take() {
                Outcome::Token(token) => {
                    self.take(at, length);
                    return Some(Ok(token));
                }
                Outcome::Skip => {
                    debug_assert!(length > 0, "a rule that reads no byte stops the scan");
                    self.offset = at + length;
                }
                Outcome::Value => {
                    self.take(at, length);
                    let (line, column) = self.place();
                    return Some(Err(ScanError::value(self.span.clone(), line, column)));
                }
                Outcome::None => {
                    self.take(at, self.faulted());
                    let (line, column) = self.place();
                    return Some(Err(ScanError::no_rule(self.span.clone(), line, column)));
                }
            }
        }
    }
}

impl<T: Lexer> FusedIterator for Scan<'_, T> {}

impl<T> Debug for Scan<'_, T> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
        let (line, column) = self.place();
        formatter
            .debug_struct("Scan")
            .field("offset", &self.offset)
            .field("condition", &self.step.condition)
            .field("line", &line)
            .field("column", &column)
            .field("span", &self.span)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The token of the lexer of the tests.
    #[derive(Debug, PartialEq, Eq)]
    enum Token {
        /// One `a`.
        One,
        /// A run of `a` that a `b` ends.
        Many,
    }

    /// A lexer of the rules `a` and `a+b`, written as the derive macro writes one.
    ///
    /// The graph holds three nodes. The root reads the first `a`. The second node accepts the rule
    /// `a` at its own offset. The third node reads the run, and it takes the offset of that accept
    /// as its marker.
    impl Lexer for Token {
        type Condition = ();

        fn step(input: &str, at: usize, step: &mut Step<Self>) {
            fn root(input: &str, at: usize, index: usize, step: &mut Step<Token>) {
                let bytes = input.as_bytes();
                let ::core::option::Option::Some(&byte) = bytes.get(index) else {
                    fault(at, index, step);
                    return;
                };
                match byte {
                    b'a' => first(input, at, index + 1, step),
                    _ => fault(at, index, step),
                }
            }

            fn first(input: &str, at: usize, index: usize, step: &mut Step<Token>) {
                let bytes = input.as_bytes();
                let ::core::option::Option::Some(&byte) = bytes.get(index) else {
                    one(at, index, step);
                    return;
                };
                match byte {
                    b'a' => run(input, at, index + 1, index, step),
                    b'b' => many(at, index + 1, step),
                    _ => one(at, index, step),
                }
            }

            fn run(input: &str, at: usize, index: usize, marker: usize, step: &mut Step<Token>) {
                let bytes = input.as_bytes();
                let mut index = index;
                while let ::core::option::Option::Some(&byte) = bytes.get(index) {
                    if byte == b'a' {
                        index += 1;
                    } else {
                        break;
                    }
                }
                let ::core::option::Option::Some(&byte) = bytes.get(index) else {
                    carried(at, index, marker, step);
                    return;
                };
                match byte {
                    b'b' => many(at, index + 1, step),
                    _ => carried(at, index, marker, step),
                }
            }

            fn one(at: usize, index: usize, step: &mut Step<Token>) {
                step.outcome = Outcome::Token(Token::One);
                step.length = index - at;
                step.read = index - at;
            }

            fn carried(at: usize, index: usize, marker: usize, step: &mut Step<Token>) {
                step.outcome = Outcome::Token(Token::One);
                step.length = marker - at;
                step.read = index - at;
            }

            fn many(at: usize, index: usize, step: &mut Step<Token>) {
                step.outcome = Outcome::Token(Token::Many);
                step.length = index - at;
                step.read = index - at;
            }

            fn fault(at: usize, index: usize, step: &mut Step<Token>) {
                step.outcome = Outcome::None;
                step.length = 0;
                step.read = index - at;
            }

            root(input, at, at, step);
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
        assert_eq!((error.line(), error.column()), (1, 2));

        assert_eq!(scan.next(), Some(Ok(Token::One)));
        assert_eq!((scan.line(), scan.column()), (2, 1));
    }

    #[test]
    fn a_step_reports_the_bytes_that_it_read_past_its_match() {
        let input = "a".repeat(2000);
        let mut step = Step::new(0);

        Token::step(&input, 0, &mut step);

        assert_eq!(step.length, 1);
        assert_eq!(step.read, 2000);
    }
}
