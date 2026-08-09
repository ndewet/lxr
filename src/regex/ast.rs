use crate::regex::charset::CharSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Epsilon,
    Class(CharSet),
    Concatenation(Vec<Node>),
    Alternation(Vec<Node>),
    Repetition(Box<Node>, Repetitions),
    Star(Box<Node>),
    Plus(Box<Node>),
    Optional(Box<Node>),
}

impl Node {
    /// Sequences and alternations hold their children in a list rather than
    /// nesting pairwise, so a pattern's tree depth is its group nesting rather
    /// than its length. Both sides are flattened, which keeps a folded chain
    /// equal to the same pattern grouped any other way.
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

    pub fn star(self) -> Self {
        Self::Star(Box::new(self))
    }

    pub fn plus(self) -> Self {
        Self::Plus(Box::new(self))
    }

    pub fn optional(self) -> Self {
        Self::Optional(Box::new(self))
    }

    pub fn repeated(self, repetitions: Repetitions) -> Self {
        Self::Repetition(Box::new(self), repetitions)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Repetitions {
    Range(usize, usize),
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
    fn a_long_alternation_drops_without_overflowing_the_stack() {
        let node = (0..100_000).fold(Node::Epsilon, |node, _| node.alternate(class('a')));
        drop(node);
    }
}
