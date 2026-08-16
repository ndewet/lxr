use crate::regex::Node;

/// One rule of a lexer.
///
/// A rule joins a pattern, the accept that a match gives, and the start
/// conditions in which the rule is applicable.
///
/// To add a rule, use [`Lexicon::rule`](super::Lexicon::rule). It is the only
/// way to make one, thus each start condition of a rule is a condition that
/// the lexicon declared.
///
/// [`compile`](super::compile) builds the states of the pattern one time. A
/// rule that is applicable in more than one start condition thus costs no more
/// states than a rule that is applicable in one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule<A> {
    /// The pattern that the rule matches.
    pub(super) pattern: Node,
    /// The accept that a match of the pattern gives.
    pub(super) accept: A,
    /// The start conditions in which the rule is applicable.
    pub(super) conditions: Vec<usize>,
}

impl<A> Rule<A> {
    /// Creates a rule from its pattern, its accept, and its start conditions.
    pub(super) fn new(pattern: Node, accept: A, conditions: Vec<usize>) -> Self {
        Self {
            pattern,
            accept,
            conditions,
        }
    }
}
