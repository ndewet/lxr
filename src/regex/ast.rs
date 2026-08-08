use crate::regex::charset::CharSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Epsilon,
    Class(CharSet),
    Concatenation(Box<Node>, Box<Node>),
    Alternation(Box<Node>, Box<Node>),
    Repetition(Box<Node>, Repetitions),
    Star(Box<Node>),
    Plus(Box<Node>),
    Optional(Box<Node>),
}

impl Node {
    pub fn concat(self, other: Self) -> Self {
        match (self, other) {
            (Self::Epsilon, node) | (node, Self::Epsilon) => node,
            (left, right) => Self::Concatenation(Box::new(left), Box::new(right)),
        }
    }

    pub fn alternate(self, other: Self) -> Self {
        Self::Alternation(Box::new(self), Box::new(other))
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
