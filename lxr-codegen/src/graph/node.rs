use super::id::NodeId;
use crate::compiler::ByteRange;

/// One node of a rule graph.
///
/// Each node becomes one function of the emitted source. The scan enters a node at an offset of
/// the input, and the node reads the bytes at that offset.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Node {
    /// Reads one byte, then goes to the node of the arm that holds that byte.
    Fork(Fork),
    /// Reads a sequence of bytes at one time.
    Rope(Rope),
    /// Gives the token of one rule, and ends the step.
    Leaf(Leaf),
    /// Reports that no rule matched, and ends the step.
    Fault,
}

/// A node that reads one byte.
///
/// The arms hold no byte in common. A byte that no arm holds gives the miss, and the scan then
/// reads that byte again at the node of the miss.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fork {
    /// One arm for each node that a byte of this fork goes to.
    pub arms: Vec<Arm>,
    /// The node that a byte of no arm goes to, at the offset of this fork.
    pub miss: Edge,
}

/// One arm of a [`Fork`].
///
/// The arm holds each byte range that goes to [`edge`](Self::edge). Two ranges of one node share
/// one arm, thus the emitted source holds one call for each node and not one for each range.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Arm {
    /// The byte ranges that take this arm, in ascending sequence.
    pub ranges: Vec<ByteRange>,
    /// The node that the arm goes to, at the offset after the byte.
    pub edge: Edge,
}

/// A node that reads a sequence of bytes at one time.
///
/// The bytes are forced: one rule of the lexer reads them and no other byte. A comparison of the
/// whole sequence thus replaces one node for each byte.
///
/// A sequence that does not match gives the miss, at the offset of the first byte of the rope. The
/// scan then reads those bytes a second time under the rules that remain.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Rope {
    /// The bytes that the rope reads.
    pub bytes: Vec<u8>,
    /// The node that a match goes to, at the offset after the bytes.
    pub then: Edge,
    /// The node that a mismatch goes to, at the offset of the first byte.
    pub miss: Edge,
}

/// A node that ends the step with the token of one rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Leaf {
    /// The rule that won the match.
    pub rule: u16,
    /// Whether the match ends at the offset of this leaf.
    ///
    /// A leaf that the scan reaches at the end of its match reads the offset alone. A leaf that
    /// the scan reaches after it read past the match takes the offset of that match as the
    /// marker.
    pub here: bool,
}

/// One edge of the graph, and the marker that it carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Edge {
    /// The node that the edge goes to.
    pub target: NodeId,
    /// The offset of the last accept that the edge carries to that node.
    pub carry: Carry,
    /// Whether the edge closes a cycle of the graph.
    ///
    /// A call along such an edge holds one stack frame for each byte of the input in a build that
    /// makes no optimization. The emitted source thus writes the number of the node and returns to
    /// the driver of the step.
    pub back: bool,
}

/// The offset of the last accept that one edge carries.
///
/// A node that holds no accept behind it takes no marker, and each edge into it carries
/// [`Nothing`](Self::Nothing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Carry {
    /// The node takes no marker.
    Nothing,
    /// The accept is at the offset of this node plus this number of bytes.
    Ahead(usize),
    /// The accept is at the offset that this node took as its marker.
    Marker,
}

impl Node {
    /// Returns each edge that leaves this node.
    pub fn edges(&self) -> Vec<Edge> {
        match self {
            Self::Fork(fork) => fork
                .arms
                .iter()
                .map(|arm| arm.edge)
                .chain([fork.miss])
                .collect(),
            Self::Rope(rope) => vec![rope.then, rope.miss],
            Self::Leaf(_) | Self::Fault => Vec::new(),
        }
    }

    /// Returns each edge that leaves this node, so that a caller can mark the edges of a cycle.
    pub fn edges_mut(&mut self) -> Vec<&mut Edge> {
        match self {
            Self::Fork(fork) => fork
                .arms
                .iter_mut()
                .map(|arm| &mut arm.edge)
                .chain([&mut fork.miss])
                .collect(),
            Self::Rope(rope) => vec![&mut rope.then, &mut rope.miss],
            Self::Leaf(_) | Self::Fault => Vec::new(),
        }
    }
}
