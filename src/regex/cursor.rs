use std::str::Chars;

#[derive(Clone)]
pub(crate) struct Cursor<'a> {
    characters: Chars<'a>,
    current: Option<char>,
    next: Option<char>,
    position: usize,
}

impl<'a> Cursor<'a> {
    pub(crate) fn new(text: &'a str) -> Self {
        let mut characters = text.chars();
        let current = characters.next();
        let next = characters.next();
        Self {
            characters,
            current,
            next,
            position: 0,
        }
    }

    pub(crate) fn position(&self) -> usize {
        self.position
    }

    pub(crate) fn peek(&self) -> Option<char> {
        self.current
    }

    pub(crate) fn peek_ahead(&self) -> Option<char> {
        self.next
    }

    pub(crate) fn pop(&mut self) -> Option<char> {
        let character = self.current?;
        self.current = self.next;
        self.next = self.characters.next();
        self.position += 1;
        Some(character)
    }

    pub(crate) fn accept(&mut self, wanted: char) -> bool {
        let matched = self.current == Some(wanted);
        if matched {
            self.pop();
        }
        matched
    }

    pub(crate) fn pop_digit(&mut self, radix: u32) -> Option<u32> {
        let digit = self.current?.to_digit(radix)?;
        self.pop();
        Some(digit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_cursor_starts_at_position_zero() {
        let cursor = Cursor::new("ab+c");
        assert_eq!(cursor.position(), 0);
    }

    #[test]
    fn test_pop_increments_position() {
        let mut cursor = Cursor::new("ab+c");
        for expected in 1..=4 {
            cursor.pop();
            assert_eq!(cursor.position(), expected);
        }
    }

    #[test]
    fn test_pop_returns_characters_in_order() {
        let mut cursor = Cursor::new("ab+c");
        for expected in ['a', 'b', '+', 'c'] {
            assert_eq!(cursor.pop(), Some(expected));
        }
    }

    #[test]
    fn test_pop_at_end_returns_none_and_does_not_advance() {
        let mut cursor = Cursor::new("a");
        assert_eq!(cursor.pop(), Some('a'));
        assert_eq!(cursor.pop(), None);
        assert_eq!(cursor.pop(), None);
        assert_eq!(cursor.position(), 1);
    }

    #[test]
    fn test_peek_returns_the_character_that_pop_will_return() {
        let mut cursor = Cursor::new("qwerty9");
        for _ in 0..7 {
            let peeked = cursor.peek();
            assert_eq!(peeked, cursor.pop());
        }
    }

    #[test]
    fn test_peek_does_not_advance_position() {
        let cursor = Cursor::new("ab");
        cursor.peek();
        cursor.peek();
        assert_eq!(cursor.position(), 0);
    }

    #[test]
    fn test_peek_at_end_returns_none() {
        let cursor = Cursor::new("");
        assert_eq!(cursor.peek(), None);
    }

    #[test]
    fn test_peek_ahead_returns_the_character_after_the_next_one() {
        let mut cursor = Cursor::new("ab");
        assert_eq!(cursor.peek_ahead(), Some('b'));
        cursor.pop();
        assert_eq!(cursor.peek_ahead(), None);
    }

    #[test]
    fn test_accept_consumes_only_a_matching_character() {
        let mut cursor = Cursor::new("ab");
        assert!(!cursor.accept('b'));
        assert_eq!(cursor.position(), 0);
        assert!(cursor.accept('a'));
        assert_eq!(cursor.position(), 1);
    }

    #[test]
    fn test_pop_digit_consumes_only_digits_of_the_given_radix() {
        let mut cursor = Cursor::new("7a");
        assert_eq!(cursor.pop_digit(10), Some(7));
        assert_eq!(cursor.pop_digit(10), None);
        assert_eq!(cursor.position(), 1);
        assert_eq!(cursor.pop_digit(16), Some(10));
        assert_eq!(cursor.position(), 2);
    }
}
