const MIN_CODEPOINT: u32 = char::MIN as u32;
const MAX_CODEPOINT: u32 = char::MAX as u32;
const SURROGATE_LOW: u32 = 0xD800;
const SURROGATE_HIGH: u32 = 0xDFFF;

/// A set of characters.
///
/// The set holds `char` values. Thus it holds no surrogate. The set keeps its
/// contents as disjoint ranges in ascending sequence. Each operation on the
/// set keeps that form.
///
/// # Examples
///
/// ```
/// use lxr::regex::CharSet;
///
/// let letters = CharSet::range('a', 'z');
/// let vowels = CharSet::single('a').union(&CharSet::single('e'));
/// let consonants = letters.subtract(&vowels);
///
/// assert_eq!(consonants.ranges().next(), Some(('b', 'd')));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharSet {
    ranges: Vec<CharRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CharRange {
    low: u32,
    high: u32,
}

impl CharSet {
    /// Creates a `CharSet` that holds no characters.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::regex::CharSet;
    ///
    /// assert!(CharSet::empty().is_empty());
    /// ```
    pub fn empty() -> Self {
        Self { ranges: Vec::new() }
    }

    /// Creates a `CharSet` that holds `c` and no other character.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::regex::CharSet;
    ///
    /// let set = CharSet::single('x');
    /// assert_eq!(set.ranges().collect::<Vec<_>>(), vec![('x', 'x')]);
    /// ```
    pub fn single(c: char) -> Self {
        let codepoint = c as u32;
        Self {
            ranges: vec![CharRange {
                low: codepoint,
                high: codepoint,
            }],
        }
    }

    /// Creates a `CharSet` that holds each character from `low` to `high`.
    ///
    /// Both ends are in the set.
    ///
    /// # Panics
    ///
    /// This function panics if `low` is above `high`.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::regex::CharSet;
    ///
    /// let digits = CharSet::range('0', '9');
    /// assert_eq!(digits.ranges().collect::<Vec<_>>(), vec![('0', '9')]);
    /// ```
    pub fn range(low: char, high: char) -> Self {
        assert!(low <= high);
        Self::from_ranges(vec![CharRange {
            low: low as u32,
            high: high as u32,
        }])
    }

    /// Returns the ranges that make the set, from the lowest to the highest.
    ///
    /// The ranges are disjoint and maximal. No two ranges overlap. Each range
    /// is as wide as the set permits.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::regex::CharSet;
    ///
    /// let set = CharSet::single('a').union(&CharSet::single('b'));
    /// assert_eq!(set.ranges().collect::<Vec<_>>(), vec![('a', 'b')]);
    /// ```
    pub fn ranges(&self) -> impl Iterator<Item = (char, char)> + '_ {
        self.ranges.iter().map(|range| {
            let low = char::from_u32(range.low).expect("a bound is never a surrogate");
            let high = char::from_u32(range.high).expect("a bound is never a surrogate");
            (low, high)
        })
    }

    /// Returns the set of the characters that are in `self` or in `other`.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::regex::CharSet;
    ///
    /// let set = CharSet::range('a', 'c').union(&CharSet::range('x', 'z'));
    /// assert_eq!(
    ///     set.ranges().collect::<Vec<_>>(),
    ///     vec![('a', 'c'), ('x', 'z')],
    /// );
    /// ```
    pub fn union(&self, other: &Self) -> Self {
        let mut all = self.ranges.clone();
        all.extend_from_slice(&other.ranges);
        Self::from_ranges(all)
    }

    /// Returns the set of the characters that are in `self` and not in
    /// `other`.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::regex::CharSet;
    ///
    /// let set = CharSet::range('a', 'e').subtract(&CharSet::single('c'));
    /// assert_eq!(
    ///     set.ranges().collect::<Vec<_>>(),
    ///     vec![('a', 'b'), ('d', 'e')],
    /// );
    /// ```
    pub fn subtract(&self, other: &Self) -> Self {
        let mut remaining = Vec::new();
        for range in &self.ranges {
            let mut low = range.low;
            for other_range in &other.ranges {
                if other_range.high < low || other_range.low > range.high {
                    continue;
                }
                if other_range.low > low {
                    remaining.push(CharRange {
                        low,
                        high: other_range.low - 1,
                    });
                }
                low = other_range.high.saturating_add(1);
                if low > range.high {
                    break;
                }
            }
            if low <= range.high {
                remaining.push(CharRange {
                    low,
                    high: range.high,
                });
            }
        }
        Self::from_ranges(remaining)
    }

    /// Returns the set of the characters that are not in `self`.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::regex::CharSet;
    ///
    /// let set = CharSet::any().negate();
    /// assert!(set.is_empty());
    /// ```
    pub fn negate(&self) -> Self {
        Self::any().subtract(self)
    }

    /// Creates a `CharSet` that holds each `char`.
    ///
    /// The set holds two ranges, because the surrogates are not characters.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::regex::CharSet;
    ///
    /// assert_eq!(CharSet::any().ranges().count(), 2);
    /// ```
    pub fn any() -> Self {
        Self {
            ranges: vec![
                CharRange {
                    low: MIN_CODEPOINT,
                    high: SURROGATE_LOW - 1,
                },
                CharRange {
                    low: SURROGATE_HIGH + 1,
                    high: MAX_CODEPOINT,
                },
            ],
        }
    }

    /// Creates a `CharSet` that holds the characters `0` to `9`.
    ///
    /// This is the set that the escape `\d` matches.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::regex::CharSet;
    ///
    /// assert_eq!(CharSet::digits().ranges().collect::<Vec<_>>(), vec![('0', '9')]);
    /// ```
    pub fn digits() -> Self {
        Self::range('0', '9')
    }

    /// Creates a `CharSet` that holds the ASCII letters, the characters `0` to
    /// `9`, and `_`.
    ///
    /// This is the set that the escape `\w` matches.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::regex::CharSet;
    ///
    /// assert_eq!(
    ///     CharSet::word().ranges().collect::<Vec<_>>(),
    ///     vec![('0', '9'), ('A', 'Z'), ('_', '_'), ('a', 'z')],
    /// );
    /// ```
    pub fn word() -> Self {
        Self::digits()
            .union(&Self::range('a', 'z'))
            .union(&Self::range('A', 'Z'))
            .union(&Self::single('_'))
    }

    /// Creates a `CharSet` that holds the space and the characters `\t` to
    /// `\r`.
    ///
    /// This is the set that the escape `\s` matches.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::regex::CharSet;
    ///
    /// assert_eq!(
    ///     CharSet::whitespace().ranges().collect::<Vec<_>>(),
    ///     vec![('\t', '\r'), (' ', ' ')],
    /// );
    /// ```
    pub fn whitespace() -> Self {
        Self::single(' ').union(&Self::range('\t', '\r'))
    }

    /// Returns `true` if the set holds no characters.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::regex::CharSet;
    ///
    /// assert!(CharSet::empty().is_empty());
    /// assert!(!CharSet::single('a').is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    fn from_ranges(mut ranges: Vec<CharRange>) -> Self {
        ranges.sort_by_key(|r| r.low);
        let mut result: Vec<CharRange> = Vec::with_capacity(ranges.len());
        for range in ranges {
            match result.last_mut() {
                Some(last) if range.low <= last.high.saturating_add(1) => {
                    last.high = last.high.max(range.high);
                }
                _ => result.push(range),
            }
        }
        let result = Self::remove_surrogates(result);
        Self { ranges: result }
    }

    fn remove_surrogates(ranges: Vec<CharRange>) -> Vec<CharRange> {
        let mut result = Vec::with_capacity(ranges.len() + 1);
        for range in ranges {
            if range.high < SURROGATE_LOW || range.low > SURROGATE_HIGH {
                result.push(range);
                continue;
            }
            if range.low < SURROGATE_LOW {
                result.push(CharRange {
                    low: range.low,
                    high: SURROGATE_LOW - 1,
                });
            }
            if range.high > SURROGATE_HIGH {
                result.push(CharRange {
                    low: SURROGATE_HIGH + 1,
                    high: range.high,
                });
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl CharSet {
        /// Returns `true` if the set holds `c`.
        pub fn contains(&self, c: char) -> bool {
            let cp = c as u32;
            self.ranges
                .binary_search_by(|range| {
                    if cp < range.low {
                        std::cmp::Ordering::Greater
                    } else if cp > range.high {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Equal
                    }
                })
                .is_ok()
        }
    }

    #[test]
    fn empty_charset_is_empty() {
        let charset = CharSet::empty();
        assert!(charset.is_empty());
    }

    #[test]
    fn empty_charset_does_not_contain_anything() {
        let charset = CharSet::empty();
        assert!(charset.is_empty());
        assert!(!charset.contains('a'));
        assert!(!charset.contains('\0'));
        assert!(!charset.contains(char::MAX));
    }

    #[test]
    fn single_charset_is_not_empty() {
        let charset = CharSet::single('a');
        assert!(!charset.is_empty());
    }

    #[test]
    fn single_charset_contains_expected_character() {
        let charset = CharSet::single('b');
        assert!(charset.contains('b'));
    }

    #[test]
    fn single_charset_does_not_contain_other_characters() {
        let charset = CharSet::single('c');
        assert!(!charset.contains('a'));
        assert!(!charset.contains('b'));
        assert!(!charset.contains('\0'));
        assert!(!charset.contains(char::MAX));
    }

    #[test]
    fn range_charset_is_not_empty() {
        let charset = CharSet::range('a', 'z');
        assert!(!charset.is_empty());
    }

    #[test]
    fn range_charset_contains_all_characters_in_range() {
        let charset = CharSet::range('b', 'e');
        assert!(charset.contains('b'));
        assert!(charset.contains('c'));
        assert!(charset.contains('d'));
        assert!(charset.contains('e'));
    }

    #[test]
    fn range_charset_does_not_contain_characters_outside_range() {
        let charset = CharSet::range('b', 'e');
        assert!(!charset.contains('a'));
        assert!(!charset.contains('f'));
        assert!(!charset.contains('\0'));
        assert!(!charset.contains(char::MAX));
    }

    #[test]
    fn any_charset_is_not_empty() {
        let charset = CharSet::any();
        assert!(!charset.is_empty());
    }

    #[test]
    fn any_charset_contains_any_character() {
        let charset = CharSet::any();
        assert!(charset.contains(char::MIN));
        assert!(charset.contains('a'));
        assert!(charset.contains('b'));
        assert!(charset.contains('c'));
        assert!(charset.contains('d'));
        assert!(charset.contains('e'));
        assert!(charset.contains('z'));
        assert!(charset.contains('!'));
        assert!(charset.contains('#'));
        assert!(charset.contains('%'));
        assert!(charset.contains('9'));
        assert!(charset.contains('\0'));
        assert!(charset.contains(char::MAX));
    }

    #[test]
    fn union_of_empty_charset_and_empty_charset_is_empty() {
        let empty1 = CharSet::empty();
        let empty2 = CharSet::empty();
        let union = empty1.union(&empty2);
        assert!(union.is_empty());
    }

    #[test]
    fn union_of_empty_charset_and_any_charset_is_any() {
        let empty = CharSet::empty();
        let any = CharSet::any();
        let union = empty.union(&any);
        assert!(!empty.eq(&any));
        assert!(union.eq(&any));
    }

    #[test]
    fn union_of_two_ranges_disjunct_ranges_contains_both_ranges() {
        let a_to_c = CharSet::range('a', 'c');
        let f_to_h = CharSet::range('f', 'h');
        let union = a_to_c.union(&f_to_h);

        assert!(!union.eq(&a_to_c));
        assert!(!union.eq(&f_to_h));

        assert!(union.contains('a'));
        assert!(union.contains('b'));
        assert!(union.contains('c'));

        assert!(!union.contains('d'));
        assert!(!union.contains('e'));

        assert!(union.contains('f'));
        assert!(union.contains('g'));
        assert!(union.contains('h'));

        assert!(!union.contains('i'));
    }

    #[test]
    fn union_of_two_overlapping_ranges_contains_full_range() {
        let b_to_d = CharSet::range('b', 'd');
        let c_to_h = CharSet::range('c', 'h');
        let union = b_to_d.union(&c_to_h);

        assert!(!union.eq(&b_to_d));
        assert!(!union.eq(&c_to_h));

        assert!(!union.contains('a'));
        assert!(union.contains('b'));
        assert!(union.contains('c'));
        assert!(union.contains('d'));
        assert!(union.contains('e'));
        assert!(union.contains('f'));
        assert!(union.contains('g'));
        assert!(union.contains('h'));

        assert!(!union.contains('i'));
    }

    #[test]
    fn negation_of_empty_is_any() {
        let empty = CharSet::empty();
        let any = CharSet::any();
        let negated = empty.negate();
        assert!(!negated.eq(&empty));
        assert!(negated.eq(&any));
    }

    #[test]
    fn negation_of_any_is_empty() {
        let empty = CharSet::empty();
        let any = CharSet::any();
        let negated = any.negate();
        assert!(!negated.eq(&any));
        assert!(negated.eq(&empty));
    }

    #[test]
    fn negation_of_range_is_two_ranges() {
        let range = CharSet::range('d', 'g');
        let negated = range.negate();

        assert!(negated.contains('a'));
        assert!(negated.contains('b'));
        assert!(negated.contains('c'));

        assert!(!negated.contains('d'));
        assert!(!negated.contains('e'));
        assert!(!negated.contains('f'));
        assert!(!negated.contains('g'));

        assert!(negated.contains('h'));
        assert!(negated.contains('i'));
        assert!(negated.contains('j'));
    }

    #[test]
    fn range_spanning_the_surrogate_gap_survives_double_negation() {
        let spanning = CharSet::range('\u{D7FF}', '\u{E000}');
        assert_eq!(spanning.negate().negate(), spanning);
    }

    #[test]
    fn sets_containing_the_same_characters_are_equal() {
        let as_union = CharSet::single('\u{D7FF}').union(&CharSet::single('\u{E000}'));
        let as_range = CharSet::range('\u{D7FF}', '\u{E000}');
        assert_eq!(as_union, as_range);
    }

    #[test]
    fn dot_equals_an_explicit_full_range() {
        assert_eq!(CharSet::range(char::MIN, char::MAX), CharSet::any());
    }
}
