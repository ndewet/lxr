use crate::regex::charset::CharSet;

/// A node of the syntax tree of a regular expression.
///
/// Each node matches a set of strings. A leaf is an [`Epsilon`](Node::Epsilon)
/// or a [`Class`](Node::Class). Each other variant holds the nodes that its
/// operator applies to.
///
/// To make a tree from a pattern, use [`FromStr`](std::str::FromStr). To make
/// a tree by hand, use the methods on this type.
///
/// # Examples
///
/// ```
/// use lxr_codegen::regex::{CharSet, Node};
///
/// let node: Node = "ab".parse().unwrap();
/// assert_eq!(
///     node,
///     Node::Concatenation(vec![
///         Node::Class(CharSet::single('a')),
///         Node::Class(CharSet::single('b')),
///     ]),
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    // A concatenation and an alternation hold their children in a list. They
    // do not nest in pairs. Thus the depth of a tree is the depth of the
    // groups in the pattern, and not the length of the pattern. The methods
    // flatten both sides. Thus a folded chain is equal to the same pattern
    // with any other grouping.
    /// Matches the empty string.
    Epsilon,
    /// Matches one character from the set.
    Class(CharSet),
    /// Matches each part in sequence.
    Concatenation(Vec<Node>),
    /// Matches one of the branches.
    Alternation(Vec<Node>),
    /// Matches the expression as many times as the repetition permits.
    Repetition(Box<Node>, Repetitions),
    /// Matches the expression zero or more times.
    Star(Box<Node>),
    /// Matches the expression one or more times.
    Plus(Box<Node>),
    /// Matches the expression zero times or one time.
    Optional(Box<Node>),
}

impl Node {
    /// Returns a node that matches `self` and then `other`.
    ///
    /// The method flattens a concatenation into one node. It also removes an
    /// [`Epsilon`](Node::Epsilon) operand.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr_codegen::regex::{CharSet, Node};
    ///
    /// let a = Node::Class(CharSet::single('a'));
    /// let b = Node::Class(CharSet::single('b'));
    ///
    /// assert_eq!(
    ///     a.clone().concat(b.clone()),
    ///     Node::Concatenation(vec![a.clone(), b]),
    /// );
    /// assert_eq!(a.clone().concat(Node::Epsilon), a);
    /// ```
    pub fn concat(self, other: Self) -> Self {
        match (self, other) {
            (Self::Epsilon, node) | (node, Self::Epsilon) => node,
            (left, right) => {
                let mut parts = left.into_concatenation_parts();
                parts.extend(right.into_concatenation_parts());
                Self::Concatenation(parts)
            }
        }
    }

    /// Returns a node that matches `self` or `other`.
    ///
    /// The method flattens an alternation into one node.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr_codegen::regex::{CharSet, Node};
    ///
    /// let a = Node::Class(CharSet::single('a'));
    /// let b = Node::Class(CharSet::single('b'));
    ///
    /// assert_eq!(
    ///     a.clone().alternate(b.clone()),
    ///     Node::Alternation(vec![a, b]),
    /// );
    /// ```
    pub fn alternate(self, other: Self) -> Self {
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

    /// Returns a node that matches `self` zero or more times.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr_codegen::regex::{CharSet, Node};
    ///
    /// let a = Node::Class(CharSet::single('a'));
    /// assert_eq!(a.clone().star(), Node::Star(Box::new(a)));
    /// ```
    pub fn star(self) -> Self {
        Self::Star(Box::new(self))
    }

    /// Returns a node that matches `self` one or more times.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr_codegen::regex::{CharSet, Node};
    ///
    /// let a = Node::Class(CharSet::single('a'));
    /// assert_eq!(a.clone().plus(), Node::Plus(Box::new(a)));
    /// ```
    pub fn plus(self) -> Self {
        Self::Plus(Box::new(self))
    }

    /// Returns a node that matches `self` zero times or one time.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr_codegen::regex::{CharSet, Node};
    ///
    /// let a = Node::Class(CharSet::single('a'));
    /// assert_eq!(a.clone().optional(), Node::Optional(Box::new(a)));
    /// ```
    pub fn optional(self) -> Self {
        Self::Optional(Box::new(self))
    }

    /// Returns a node that matches `self` as many times as `repetitions`
    /// permits.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr_codegen::regex::{CharSet, Node, Repetitions};
    ///
    /// let a = Node::Class(CharSet::single('a'));
    /// let counts = Repetitions::Range(2, 4);
    ///
    /// assert_eq!(
    ///     a.clone().repeated(counts),
    ///     Node::Repetition(Box::new(a), counts),
    /// );
    /// ```
    pub fn repeated(self, repetitions: Repetitions) -> Self {
        Self::Repetition(Box::new(self), repetitions)
    }

    /// Returns `true` if the node matches the empty string.
    ///
    /// A rule of a lexer needs a pattern that reads at least one character. A
    /// pattern that matches the empty string gives a match of no length, thus
    /// the lexer makes no progress.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr_codegen::regex::Node;
    ///
    /// assert!("a*".parse::<Node>().unwrap().matches_empty());
    /// assert!(!"a+".parse::<Node>().unwrap().matches_empty());
    /// ```
    pub fn matches_empty(&self) -> bool {
        match self {
            Self::Epsilon | Self::Star(_) | Self::Optional(_) => true,
            Self::Class(_) => false,
            Self::Concatenation(parts) => parts.iter().all(Self::matches_empty),
            Self::Alternation(branches) => branches.iter().any(Self::matches_empty),
            Self::Plus(inner) => inner.matches_empty(),
            Self::Repetition(
                inner,
                Repetitions::Range(minimum, _) | Repetitions::AtLeast(minimum),
            ) => *minimum == 0 || inner.matches_empty(),
        }
    }

    /// Returns the first repetition of the tree whose maximum is below its
    /// minimum.
    ///
    /// Such a repetition matches nothing. The parser rejects it, but a tree
    /// that a caller builds by hand can hold one.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr_codegen::regex::{CharSet, Node, Repetitions};
    ///
    /// let a = Node::Class(CharSet::single('a'));
    /// let inverted = Repetitions::Range(5, 2);
    ///
    /// assert_eq!(
    ///     a.clone().repeated(inverted).inverted_repetition(),
    ///     Some(inverted),
    /// );
    /// assert_eq!(
    ///     a.repeated(Repetitions::Range(2, 5)).inverted_repetition(),
    ///     None,
    /// );
    /// ```
    pub fn inverted_repetition(&self) -> Option<Repetitions> {
        match self {
            Self::Epsilon | Self::Class(_) => None,
            Self::Concatenation(parts) | Self::Alternation(parts) => {
                parts.iter().find_map(Self::inverted_repetition)
            }
            Self::Star(inner) | Self::Plus(inner) | Self::Optional(inner) => {
                inner.inverted_repetition()
            }
            Self::Repetition(inner, repetitions) => match *repetitions {
                Repetitions::Range(minimum, maximum) if maximum < minimum => Some(*repetitions),
                _ => inner.inverted_repetition(),
            },
        }
    }

    /// Returns the number of the nodes of the tree, after each repetition
    /// expands into one copy for each permitted repetition.
    ///
    /// A construction that has no counter makes those copies. The count thus
    /// gives the size of the pattern that such a construction reads. A nested
    /// repetition multiplies the count. Therefore a short pattern can give a
    /// very large count.
    ///
    /// The count saturates at [`usize::MAX`].
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr_codegen::regex::Node;
    ///
    /// let one: Node = "a".parse().unwrap();
    /// let many: Node = "(a{100}){100}".parse().unwrap();
    ///
    /// assert_eq!(one.expanded_size(), 1);
    /// assert!(many.expanded_size() > 10_000);
    /// ```
    pub fn expanded_size(&self) -> usize {
        match self {
            Self::Epsilon | Self::Class(_) => 1,
            Self::Concatenation(parts) | Self::Alternation(parts) => parts
                .iter()
                .fold(1, |total, part| total.saturating_add(part.expanded_size())),
            Self::Star(inner) | Self::Plus(inner) | Self::Optional(inner) => {
                inner.expanded_size().saturating_add(1)
            }
            Self::Repetition(inner, repetitions) => {
                let (minimum, maximum) = match *repetitions {
                    Repetitions::Range(minimum, maximum) => (minimum, maximum),
                    Repetitions::AtLeast(minimum) => (minimum, minimum.saturating_add(1)),
                };
                inner
                    .expanded_size()
                    .saturating_mul(maximum)
                    .saturating_add(maximum.saturating_sub(minimum))
                    .saturating_add(1)
            }
        }
    }
}

/// The number of times that a [`Repetition`](Node::Repetition) matches its
/// expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Repetitions {
    /// Matches a minimum of the first count of times, and a maximum of the
    /// second count of times.
    Range(usize, usize),
    /// Matches a minimum of this count of times. There is no maximum.
    AtLeast(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class(character: char) -> Node {
        Node::Class(CharSet::single(character))
    }

    fn folded(atoms: usize) -> Node {
        (0..atoms).fold(Node::Epsilon, |node, _| node.concat(class('a')))
    }

    #[test]
    fn concatenation_holds_every_part_in_one_node() {
        let node = class('a').concat(class('b')).concat(class('c'));
        assert_eq!(
            node,
            Node::Concatenation(vec![class('a'), class('b'), class('c')])
        );
    }

    #[test]
    fn alternation_holds_every_branch_in_one_node() {
        let node = class('a').alternate(class('b')).alternate(class('c'));
        assert_eq!(
            node,
            Node::Alternation(vec![class('a'), class('b'), class('c')])
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
            Node::Star(Box::new(Node::Concatenation(vec![class('a'), class('b')])))
        );
    }

    #[test]
    fn epsilon_is_absorbed_by_concatenation() {
        assert_eq!(Node::Epsilon.concat(class('a')), class('a'));
        assert_eq!(class('a').concat(Node::Epsilon), class('a'));
        assert_eq!(Node::Epsilon.concat(Node::Epsilon), Node::Epsilon);
    }

    #[test]
    fn a_long_concatenation_is_flat_rather_than_deep() {
        match folded(100_000) {
            Node::Concatenation(parts) => assert_eq!(parts.len(), 100_000),
            other => panic!("expected one concatenation node, got {other:?}"),
        }
    }

    #[test]
    fn a_long_concatenation_drops_without_overflowing_the_stack() {
        drop(folded(100_000));
    }

    #[test]
    fn a_node_that_reads_at_least_one_character_does_not_match_the_empty_string() {
        for pattern in ["a", "ab", "a|b", "a+", "a{1,3}", "(a|b)+c", "(a*b)+"] {
            let node: Node = pattern.parse().unwrap();
            assert!(!node.matches_empty(), "{pattern} matches the empty string");
        }
    }

    #[test]
    fn a_node_that_reads_no_character_matches_the_empty_string() {
        for pattern in ["a*", "a?", "a{0,3}", "(a|)", "a*b*", "(a*)+", "(a?){2}"] {
            let node: Node = pattern.parse().unwrap();
            assert!(node.matches_empty(), "{pattern} reads a character");
        }
        assert!(Node::Epsilon.matches_empty());
    }

    #[test]
    fn the_expanded_size_counts_one_copy_for_each_permitted_repetition() {
        let single: Node = "a".parse().unwrap();
        let three: Node = "a{3}".parse().unwrap();
        let range: Node = "a{1,3}".parse().unwrap();
        let at_least: Node = "a{2,}".parse().unwrap();

        assert_eq!(single.expanded_size(), 1);
        assert_eq!(three.expanded_size(), 4);
        assert_eq!(range.expanded_size(), 6);
        assert_eq!(at_least.expanded_size(), 5);
    }

    #[test]
    fn a_nested_repetition_multiplies_the_expanded_size() {
        let node: Node = "(a{100}){100}".parse().unwrap();
        assert_eq!(node.expanded_size(), 10_101);
    }

    #[test]
    fn the_expanded_size_saturates_rather_than_overflows() {
        let node = class('a').repeated(Repetitions::Range(0, usize::MAX));
        assert_eq!(node.expanded_size(), usize::MAX);
    }

    #[test]
    fn a_long_alternation_drops_without_overflowing_the_stack() {
        let node = (0..100_000).fold(Node::Epsilon, |node, _| node.alternate(class('a')));
        drop(node);
    }
}
