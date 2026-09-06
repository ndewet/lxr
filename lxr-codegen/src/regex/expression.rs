use crate::regex::charset::CharSet;
use crate::regex::quantifier::Quantifier;

/// Stores one node of a parsed regex syntax tree.
///
/// Each node denotes a regular language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Expression {
    // Concatenation and alternation use flat lists. Tree depth follows group
    // nesting instead of pattern length.
    /// Matches the empty string.
    Epsilon,
    /// Matches one character from the set.
    Class(CharSet),
    /// Matches each part in sequence.
    Concatenation(Vec<Expression>),
    /// Matches one of the branches.
    Alternation(Vec<Expression>),
    /// Matches the expression as many times as the quantifier permits.
    Repetition(Box<Expression>, Quantifier),
}

impl Expression {
    /// Concatenates `self` with `other`.
    ///
    /// The result flattens nested concatenations and removes epsilon operands.
    pub(crate) fn concat(self, other: Self) -> Self {
        match (self, other) {
            (Self::Epsilon, node) | (node, Self::Epsilon) => node,
            (left, right) => {
                let mut parts = left.into_concatenation_parts();
                parts.extend(right.into_concatenation_parts());
                Self::Concatenation(parts)
            }
        }
    }

    /// Alternates between `self` and `other` in one flat node.
    pub(crate) fn alternate(self, other: Self) -> Self {
        let mut branches = self.into_alternation_branches();
        branches.extend(other.into_alternation_branches());
        Self::Alternation(branches)
    }

    fn into_concatenation_parts(self) -> Vec<Self> {
        match self {
            Self::Concatenation(parts) => parts,
            node => vec![node],
        }
    }

    fn into_alternation_branches(self) -> Vec<Self> {
        match self {
            Self::Alternation(branches) => branches,
            node => vec![node],
        }
    }

    /// Applies the `*` quantifier to `self`.
    pub(crate) fn star(self) -> Self {
        self.repeated(Quantifier::ZERO_OR_MORE)
    }

    /// Applies the `+` quantifier to `self`.
    pub(crate) fn plus(self) -> Self {
        self.repeated(Quantifier::ONE_OR_MORE)
    }

    /// Applies the `?` quantifier to `self`.
    pub(crate) fn optional(self) -> Self {
        self.repeated(Quantifier::ZERO_OR_ONE)
    }

    /// Applies `quantifier` to `self`.
    pub(crate) fn repeated(self, quantifier: Quantifier) -> Self {
        Self::Repetition(Box::new(self), quantifier)
    }

    /// Returns whether this expression matches the empty string.
    ///
    /// Lexer rules must not be nullable because each match must consume input.
    pub(crate) fn is_nullable(&self) -> bool {
        match self {
            Self::Epsilon => true,
            Self::Class(_) => false,
            Self::Concatenation(parts) => parts.iter().all(Self::is_nullable),
            Self::Alternation(branches) => branches.iter().any(Self::is_nullable),
            Self::Repetition(inner, quantifier) => quantifier.minimum() == 0 || inner.is_nullable(),
        }
    }

    /// Returns the node count after bounded repetitions expand.
    ///
    /// This estimate predicts Thompson construction size. It saturates at
    /// [`usize::MAX`].
    pub(crate) fn expanded_size(&self) -> usize {
        match self {
            Self::Epsilon | Self::Class(_) => 1,
            Self::Concatenation(parts) | Self::Alternation(parts) => parts
                .iter()
                .fold(1, |total, part| total.saturating_add(part.expanded_size())),
            Self::Repetition(inner, quantifier) if quantifier.has_direct_construction() => {
                inner.expanded_size().saturating_add(1)
            }
            Self::Repetition(inner, quantifier) => {
                let minimum = quantifier.minimum();
                let maximum = quantifier
                    .maximum()
                    .unwrap_or_else(|| minimum.saturating_add(1));
                expanded_repetition_size(inner, minimum, maximum)
            }
        }
    }
}

fn expanded_repetition_size(expression: &Expression, minimum: usize, maximum: usize) -> usize {
    expression
        .expanded_size()
        .saturating_mul(maximum)
        .saturating_add(maximum.saturating_sub(minimum))
        .saturating_add(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class(character: char) -> Expression {
        Expression::Class(CharSet::single(character))
    }

    fn folded(atoms: usize) -> Expression {
        (0..atoms).fold(Expression::Epsilon, |node, _| node.concat(class('a')))
    }

    #[test]
    fn concatenation_holds_every_part_in_one_expression() {
        let node = class('a').concat(class('b')).concat(class('c'));
        assert_eq!(
            node,
            Expression::Concatenation(vec![class('a'), class('b'), class('c')])
        );
    }

    #[test]
    fn alternation_holds_every_branch_in_one_expression() {
        let node = class('a').alternate(class('b')).alternate(class('c'));
        assert_eq!(
            node,
            Expression::Alternation(vec![class('a'), class('b'), class('c')])
        );
    }

    #[test]
    fn grouping_does_not_change_a_concatenation() {
        let left_folded = class('a').concat(class('b')).concat(class('c'));
        let right_folded = class('a').concat(class('b').concat(class('c')));
        assert_eq!(left_folded, right_folded);
    }

    #[test]
    fn grouping_does_not_change_an_alternation() {
        let left_folded = class('a').alternate(class('b')).alternate(class('c'));
        let right_folded = class('a').alternate(class('b').alternate(class('c')));
        assert_eq!(left_folded, right_folded);
    }

    #[test]
    fn quantifiers_still_nest_their_operand() {
        let node = class('a').concat(class('b')).star();
        assert_eq!(
            node,
            Expression::Repetition(
                Box::new(Expression::Concatenation(vec![class('a'), class('b')])),
                Quantifier::ZERO_OR_MORE,
            )
        );
    }

    #[test]
    fn epsilon_is_absorbed_by_concatenation() {
        assert_eq!(Expression::Epsilon.concat(class('a')), class('a'));
        assert_eq!(class('a').concat(Expression::Epsilon), class('a'));
        assert_eq!(
            Expression::Epsilon.concat(Expression::Epsilon),
            Expression::Epsilon
        );
    }

    #[test]
    fn a_long_concatenation_is_flat_rather_than_deep() {
        match folded(100_000) {
            Expression::Concatenation(parts) => assert_eq!(parts.len(), 100_000),
            other => panic!("expected one concatenation node, got {other:?}"),
        }
    }

    #[test]
    fn a_long_concatenation_drops_without_overflowing_the_stack() {
        drop(folded(100_000));
    }

    #[test]
    fn an_expression_that_reads_at_least_one_character_does_not_match_the_empty_string() {
        for pattern in ["a", "ab", "a|b", "a+", "a{1,3}", "(a|b)+c", "(a*b)+"] {
            let node: Expression = pattern.parse().unwrap();
            assert!(!node.is_nullable(), "{pattern} matches the empty string");
        }
    }

    #[test]
    fn an_expression_that_reads_no_character_matches_the_empty_string() {
        for pattern in ["a*", "a?", "a{0,3}", "(a|)", "a*b*", "(a*)+", "(a?){2}"] {
            let node: Expression = pattern.parse().unwrap();
            assert!(node.is_nullable(), "{pattern} reads a character");
        }
        assert!(Expression::Epsilon.is_nullable());
    }

    #[test]
    fn the_expanded_size_counts_one_copy_for_each_permitted_repetition() {
        let single: Expression = "a".parse().unwrap();
        let three: Expression = "a{3}".parse().unwrap();
        let range: Expression = "a{1,3}".parse().unwrap();
        let at_least: Expression = "a{2,}".parse().unwrap();

        assert_eq!(single.expanded_size(), 1);
        assert_eq!(three.expanded_size(), 4);
        assert_eq!(range.expanded_size(), 6);
        assert_eq!(at_least.expanded_size(), 5);
    }

    #[test]
    fn a_nested_repetition_multiplies_the_expanded_size() {
        let node: Expression = "(a{100}){100}".parse().unwrap();
        assert_eq!(node.expanded_size(), 10_101);
    }

    #[test]
    fn the_expanded_size_saturates_rather_than_overflows() {
        let node = class('a').repeated(Quantifier::range(0, usize::MAX).unwrap());
        assert_eq!(node.expanded_size(), usize::MAX);
    }

    #[test]
    fn a_long_alternation_drops_without_overflowing_the_stack() {
        let node = (0..100_000).fold(Expression::Epsilon, |node, _| node.alternate(class('a')));
        drop(node);
    }
}
