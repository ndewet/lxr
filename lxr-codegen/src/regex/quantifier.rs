/// The permitted repetition counts for an expression.
///
/// A quantifier always has a valid range. Use [`range`](Self::range) to make a
/// bounded quantifier from variable counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Quantifier {
    minimum: usize,
    maximum: Option<usize>,
}

impl Quantifier {
    /// The `*` quantifier, which permits zero or more repetitions.
    pub const ZERO_OR_MORE: Self = Self::at_least(0);

    /// The `+` quantifier, which permits one or more repetitions.
    pub const ONE_OR_MORE: Self = Self::at_least(1);

    /// The `?` quantifier, which permits zero repetitions or one repetition.
    pub const ZERO_OR_ONE: Self = Self {
        minimum: 0,
        maximum: Some(1),
    };

    /// Creates a quantifier that permits exactly `count` repetitions.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr_codegen::regex::Quantifier;
    ///
    /// assert_eq!(Quantifier::exactly(3).minimum(), 3);
    /// assert_eq!(Quantifier::exactly(3).maximum(), Some(3));
    /// ```
    pub const fn exactly(count: usize) -> Self {
        Self {
            minimum: count,
            maximum: Some(count),
        }
    }

    /// Creates a quantifier that permits `minimum` or more repetitions.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr_codegen::regex::Quantifier;
    ///
    /// assert_eq!(Quantifier::at_least(2).minimum(), 2);
    /// assert_eq!(Quantifier::at_least(2).maximum(), None);
    /// ```
    pub const fn at_least(minimum: usize) -> Self {
        Self {
            minimum,
            maximum: None,
        }
    }

    /// Creates a quantifier for the inclusive range from `minimum` to `maximum`.
    ///
    /// # Errors
    ///
    /// This function returns a [`QuantifierRangeError`] if `maximum` is below
    /// `minimum`.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr_codegen::regex::Quantifier;
    ///
    /// assert!(Quantifier::range(2, 5).is_ok());
    /// assert!(Quantifier::range(5, 2).is_err());
    /// ```
    pub fn range(minimum: usize, maximum: usize) -> Result<Self, QuantifierRangeError> {
        if maximum < minimum {
            return Err(QuantifierRangeError { minimum, maximum });
        }
        Ok(Self {
            minimum,
            maximum: Some(maximum),
        })
    }

    /// Returns the smallest permitted repetition count.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr_codegen::regex::Quantifier;
    ///
    /// assert_eq!(Quantifier::ONE_OR_MORE.minimum(), 1);
    /// ```
    pub const fn minimum(self) -> usize {
        self.minimum
    }

    /// Returns the largest permitted repetition count, if it has one.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr_codegen::regex::Quantifier;
    ///
    /// assert_eq!(Quantifier::ZERO_OR_ONE.maximum(), Some(1));
    /// assert_eq!(Quantifier::ZERO_OR_MORE.maximum(), None);
    /// ```
    pub const fn maximum(self) -> Option<usize> {
        self.maximum
    }

    /// Returns `true` if Thompson construction needs one operand fragment.
    pub(crate) const fn has_direct_construction(self) -> bool {
        matches!(
            (self.minimum, self.maximum),
            (0, None) | (1, None) | (0, Some(1))
        )
    }
}

/// An invalid inclusive range for a [`Quantifier`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuantifierRangeError {
    /// The requested minimum repetition count.
    pub minimum: usize,
    /// The requested maximum repetition count.
    pub maximum: usize,
}

impl std::fmt::Display for QuantifierRangeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "the maximum repetition count {} is below the minimum {}",
            self.maximum, self.minimum
        )
    }
}

impl std::error::Error for QuantifierRangeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quantifier_range_cannot_have_an_inverted_bound() {
        assert_eq!(
            Quantifier::range(3, 1),
            Err(QuantifierRangeError {
                minimum: 3,
                maximum: 1,
            })
        );
    }

    #[test]
    fn equivalent_quantifiers_have_one_representation() {
        assert_eq!(Quantifier::at_least(0), Quantifier::ZERO_OR_MORE);
        assert_eq!(Quantifier::at_least(1), Quantifier::ONE_OR_MORE);
        assert_eq!(Quantifier::range(0, 1), Ok(Quantifier::ZERO_OR_ONE));
        assert_eq!(Quantifier::range(3, 3), Ok(Quantifier::exactly(3)));
    }
}
