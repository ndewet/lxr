/// Stores the permitted repetition counts for an expression.
///
/// The optional maximum is never less than the minimum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Quantifier {
    minimum: usize,
    maximum: Option<usize>,
}

impl Quantifier {
    /// The `*` quantifier, which permits zero or more repetitions.
    pub(crate) const ZERO_OR_MORE: Self = Self::at_least(0);

    /// The `+` quantifier, which permits one or more repetitions.
    pub(crate) const ONE_OR_MORE: Self = Self::at_least(1);

    /// The `?` quantifier, which permits zero or one repetition.
    pub(crate) const ZERO_OR_ONE: Self = Self {
        minimum: 0,
        maximum: Some(1),
    };

    /// Creates a quantifier for exactly `count` repetitions.
    pub(crate) const fn exactly(count: usize) -> Self {
        Self {
            minimum: count,
            maximum: Some(count),
        }
    }

    /// Creates a quantifier for `minimum` or more repetitions.
    pub(crate) const fn at_least(minimum: usize) -> Self {
        Self {
            minimum,
            maximum: None,
        }
    }

    /// Creates a quantifier for `minimum..=maximum` repetitions.
    ///
    /// # Errors
    ///
    /// This function returns a [`QuantifierRangeError`] if `maximum` is below
    /// `minimum`.
    pub(crate) fn range(minimum: usize, maximum: usize) -> Result<Self, QuantifierRangeError> {
        if maximum < minimum {
            return Err(QuantifierRangeError { minimum, maximum });
        }
        Ok(Self {
            minimum,
            maximum: Some(maximum),
        })
    }

    /// Returns the minimum repetition count.
    pub(crate) const fn minimum(self) -> usize {
        self.minimum
    }

    /// Returns the maximum repetition count, if one exists.
    pub(crate) const fn maximum(self) -> Option<usize> {
        self.maximum
    }

    /// Returns whether Thompson construction can use one operand fragment.
    pub(crate) const fn has_direct_construction(self) -> bool {
        matches!(
            (self.minimum, self.maximum),
            (0, None) | (1, None) | (0, Some(1))
        )
    }
}

/// Reports an inverted [`Quantifier`] range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct QuantifierRangeError {
    /// The requested minimum.
    pub(crate) minimum: usize,
    /// The requested maximum.
    pub(crate) maximum: usize,
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
