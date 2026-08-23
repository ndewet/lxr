use super::id::NodeId;
use super::node::Node;

/// The nodes of a rule graph, and the node at which each start condition begins.
///
/// To build an arena, use [`build`](super::build()).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arena {
    nodes: Vec<Node>,
    starts: Vec<NodeId>,
    markers: Vec<bool>,
}

impl Arena {
    /// Creates an arena from its nodes, the node of each start condition, and the nodes that take
    /// a marker.
    ///
    /// # Panics
    ///
    /// This function panics if `starts` is empty, if a start is not a node of `nodes`, or if
    /// `markers` holds no answer for each node.
    pub(super) fn new(nodes: Vec<Node>, starts: Vec<NodeId>, markers: Vec<bool>) -> Self {
        assert!(!starts.is_empty(), "a graph needs at least one start node");
        assert_eq!(
            markers.len(),
            nodes.len(),
            "a graph needs one answer for each of its {} nodes",
            nodes.len()
        );
        for &start in &starts {
            assert!(
                start.index() < nodes.len(),
                "start node {} is outside a graph of {} nodes",
                start.index(),
                nodes.len()
            );
        }

        Self {
            nodes,
            starts,
            markers,
        }
    }

    /// Returns the number of the nodes of the graph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the node that `id` refers to.
    ///
    /// # Panics
    ///
    /// This function panics if `id` is not a node of the graph.
    pub fn node(&self, id: NodeId) -> &Node {
        self.nodes
            .get(id.index())
            .unwrap_or_else(|| self.outside(id))
    }

    /// Returns each node of the graph, in the sequence of the identifiers.
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// Returns the node at which the start condition at `condition` begins.
    ///
    /// # Panics
    ///
    /// This function panics if `condition` is not a start condition of the graph.
    pub fn start(&self, condition: usize) -> NodeId {
        *self.starts.get(condition).unwrap_or_else(|| {
            panic!(
                "condition {condition} is outside a graph of {} start conditions",
                self.starts.len()
            )
        })
    }

    /// Returns the number of the start conditions of the graph.
    pub fn start_count(&self) -> usize {
        self.starts.len()
    }

    /// Returns whether the node that `id` refers to takes the offset of the last accept.
    ///
    /// A node that holds an accept behind it takes that offset as a parameter. Each other node
    /// reads the offset that it is at, thus it needs no parameter.
    ///
    /// # Panics
    ///
    /// This function panics if `id` is not a node of the graph.
    pub fn takes_marker(&self, id: NodeId) -> bool {
        *self
            .markers
            .get(id.index())
            .unwrap_or_else(|| self.outside(id))
    }

    /// Returns each rule that a leaf of the graph gives.
    ///
    /// A rule that no leaf gives can never win a match.
    pub fn winners(&self) -> Vec<u16> {
        let mut rules: Vec<u16> = self
            .nodes
            .iter()
            .filter_map(|node| match node {
                Node::Leaf(leaf) => Some(leaf.rule),
                _ => None,
            })
            .collect();
        rules.sort_unstable();
        rules.dedup();
        rules
    }

    /// Reports `id` as outside the graph.
    ///
    /// # Panics
    ///
    /// This function panics each time.
    fn outside(&self, id: NodeId) -> ! {
        panic!(
            "node {} is outside a graph of {} nodes",
            id.index(),
            self.nodes.len()
        )
    }
}
