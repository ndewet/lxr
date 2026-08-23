/// An index into the node arena of a rule graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(u32);

impl NodeId {
    /// Creates a `NodeId` from an index into the node arena.
    ///
    /// # Panics
    ///
    /// This function panics if `index` is above [`u32::MAX`].
    pub fn new(index: usize) -> Self {
        Self(u32::try_from(index).expect("a graph holds at most u32::MAX + 1 nodes"))
    }

    /// Returns the index into the node arena that this identifier refers to.
    pub fn index(self) -> usize {
        self.0 as usize
    }

    /// Returns this identifier as the number that the emitted source writes.
    pub fn number(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_node_id_round_trips_through_its_index() {
        assert_eq!(NodeId::new(0).index(), 0);
        assert_eq!(NodeId::new(7).number(), 7);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    #[should_panic(expected = "a graph holds at most u32::MAX + 1 nodes")]
    fn a_node_id_past_the_last_index_panics() {
        NodeId::new(u32::MAX as usize + 1);
    }
}
