/// The result of one scan of the input at one offset.
///
/// [`Lexer::find`](crate::Lexer::find) gives it. The emitted source of a lexer builds it, thus the
/// fields are public.
///
/// The longest match wins, and the earliest rule wins a tie.
/// [`length`](Self::length) counts the bytes of that match, and [`read`](Self::read) counts the
/// bytes that the scan read. The two differ when the scan reads past the last accept, and the scan
/// stops a region that no rule ends with that difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Matched {
    /// The rule that won the match, plus one, or 0 if no rule matched.
    ///
    /// [`Tables::accept`](crate::Tables::accept) numbers a rule in the same manner.
    pub accept: u16,
    /// The number of the bytes of the match, or 0 if no rule matched.
    pub length: usize,
    /// The number of the bytes that the scan read, the bytes after the match included.
    pub read: usize,
}

impl Matched {
    /// Creates the result of a scan that no rule matched, and that read `read` bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::Matched;
    ///
    /// assert_eq!(Matched::none(3).rule(), None);
    /// ```
    pub const fn none(read: usize) -> Self {
        Self {
            accept: 0,
            length: 0,
            read,
        }
    }

    /// Creates the result of a scan that the rule at `rule` won with `length` bytes, after it read
    /// `read` bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::Matched;
    ///
    /// let matched = Matched::new(2, 4, 6);
    ///
    /// assert_eq!(matched.rule(), Some(2));
    /// assert_eq!(matched.length, 4);
    /// ```
    pub const fn new(rule: u16, length: usize, read: usize) -> Self {
        Self {
            accept: rule + 1,
            length,
            read,
        }
    }

    /// Returns the rule that won the match, or `None` if no rule matched.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::Matched;
    ///
    /// assert_eq!(Matched::new(0, 1, 1).rule(), Some(0));
    /// assert_eq!(Matched::none(0).rule(), None);
    /// ```
    pub const fn rule(&self) -> Option<u16> {
        self.accept.checked_sub(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_match_holds_the_rule_that_won_it() {
        let matched = Matched::new(7, 3, 5);

        assert_eq!(matched.accept, 8);
        assert_eq!(matched.rule(), Some(7));
        assert_eq!((matched.length, matched.read), (3, 5));
    }

    #[test]
    fn a_scan_that_no_rule_matched_holds_the_bytes_that_it_read() {
        let matched = Matched::none(9);

        assert_eq!(matched.rule(), None);
        assert_eq!((matched.length, matched.read), (0, 9));
    }
}
