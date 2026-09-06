use super::{BuildError, StateId};

/// Stores one group of edges for each state.
///
/// Each group is a slice in one shared edge allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AdjacencyList<T> {
    offsets: Vec<u32>,
    edges: Vec<T>,
}

impl<T> AdjacencyList<T> {
    fn new(offsets: Vec<u32>, edges: Vec<T>) -> Self {
        debug_assert!(
            offsets.last() == Some(&(edges.len() as u32)),
            "the last offset is the number of edges"
        );
        Self { offsets, edges }
    }

    /// Returns the edges from `state`, or `None` if the list has no such state.
    pub(crate) fn get(&self, state: StateId) -> Option<&[T]> {
        let start = *self.offsets.get(state.index())? as usize;
        let end = *self.offsets.get(state.index() + 1)? as usize;
        Some(&self.edges[start..end])
    }

    /// Returns the number of states in the list.
    pub(crate) fn state_count(&self) -> usize {
        self.offsets.len() - 1
    }

    /// Returns all edges in state sequence.
    #[cfg(test)]
    pub(crate) fn edges(&self) -> &[T] {
        &self.edges
    }
}

impl<T> Default for AdjacencyList<T> {
    fn default() -> Self {
        Self {
            offsets: vec![0],
            edges: Vec::new(),
        }
    }
}

/// Collects edges before it builds an [`AdjacencyList`].
#[derive(Debug)]
pub(crate) struct AdjacencyListBuilder<T> {
    edges: Vec<(StateId, T)>,
    capacity: usize,
}

impl<T> AdjacencyListBuilder<T> {
    /// The maximum number of stored edges.
    pub(crate) const CAPACITY: usize = u32::MAX as usize;

    /// Creates an empty builder.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn with_capacity(capacity: usize) -> Self {
        Self {
            edges: Vec::new(),
            capacity,
        }
    }

    /// Adds an outgoing edge to `state`.
    pub(crate) fn add(&mut self, state: StateId, edge: T) {
        self.edges.push((state, edge));
    }

    /// Builds a list with `state_count` groups.
    ///
    /// # Errors
    ///
    /// This function returns a [`BuildError`] if the edge count exceeds the capacity.
    ///
    /// # Panics
    ///
    /// This function panics if an edge source is outside the state range.
    pub(crate) fn build(self, state_count: usize) -> Result<AdjacencyList<T>, BuildError> {
        if self.edges.len() > self.capacity {
            return Err(BuildError::TooManyTransitions {
                capacity: self.capacity,
            });
        }

        let mut offsets = vec![0u32; state_count + 1];
        for &(state, _) in &self.edges {
            let index = state.index();
            assert!(
                index < state_count,
                "state {index} is outside an adjacency list of {state_count} states"
            );
            offsets[index + 1] += 1;
        }
        for index in 1..offsets.len() {
            offsets[index] += offsets[index - 1];
        }

        let mut cursors = offsets.clone();
        let mut slots: Vec<Option<T>> = (0..self.edges.len()).map(|_| None).collect();
        for (state, edge) in self.edges {
            let cursor = &mut cursors[state.index()];
            slots[*cursor as usize] = Some(edge);
            *cursor += 1;
        }

        let edges = slots
            .into_iter()
            .map(|slot| slot.expect("the offsets cover each edge slot one time"))
            .collect();
        Ok(AdjacencyList::new(offsets, edges))
    }
}

impl<T> Default for AdjacencyListBuilder<T> {
    fn default() -> Self {
        Self::with_capacity(Self::CAPACITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> AdjacencyList<u32> {
        AdjacencyList::new(vec![0, 1, 1, 4], vec![10, 20, 30, 40])
    }

    fn built(groups: usize, entries: &[(usize, u32)]) -> AdjacencyList<u32> {
        let mut builder = AdjacencyListBuilder::new();
        for &(group, item) in entries {
            builder.add(StateId::new(group), item);
        }
        builder
            .build(groups)
            .expect("the adjacency list is below its capacity")
    }

    #[test]
    fn a_group_is_the_slice_from_its_offset_to_the_next_offset() {
        assert_eq!(list().get(StateId::new(0)), Some(&[10][..]));
        assert_eq!(list().get(StateId::new(2)), Some(&[20, 30, 40][..]));
    }

    #[test]
    fn a_group_without_an_item_is_an_empty_slice() {
        assert_eq!(list().get(StateId::new(1)), Some(&[][..]));
    }

    #[test]
    fn the_adjacency_list_has_one_group_less_than_it_has_offsets() {
        assert_eq!(list().state_count(), 3);
    }

    #[test]
    fn a_group_outside_the_adjacency_list_gives_nothing() {
        assert_eq!(list().get(StateId::new(3)), None);
        assert_eq!(list().get(StateId::new(9)), None);
    }

    #[test]
    fn the_adjacency_list_gives_each_item_in_one_slice() {
        assert_eq!(list().edges(), &[10, 20, 30, 40]);
    }

    #[test]
    fn the_default_adjacency_list_holds_no_group() {
        let list = AdjacencyList::<u32>::default();

        assert_eq!(list.state_count(), 0);
        assert_eq!(list.get(StateId::new(0)), None);
        assert_eq!(list.edges(), &[]);
    }

    #[test]
    fn a_group_keeps_the_sequence_in_which_the_items_arrived() {
        let list = built(2, &[(1, 30), (0, 10), (1, 20), (1, 10)]);

        assert_eq!(list.get(StateId::new(0)), Some(&[10][..]));
        assert_eq!(list.get(StateId::new(1)), Some(&[30, 20, 10][..]));
    }

    #[test]
    fn a_group_that_gets_no_item_is_empty() {
        let list = built(3, &[(2, 10)]);

        assert_eq!(list.get(StateId::new(0)), Some(&[][..]));
        assert_eq!(list.get(StateId::new(1)), Some(&[][..]));
        assert_eq!(list.get(StateId::new(2)), Some(&[10][..]));
    }

    #[test]
    fn the_adjacency_list_holds_the_items_of_the_first_group_first() {
        let list = built(2, &[(1, 30), (0, 10), (1, 20)]);

        assert_eq!(list.edges(), &[10, 30, 20]);
    }

    #[test]
    fn an_adjacency_list_without_an_item_keeps_its_groups() {
        let list = built(3, &[]);

        assert_eq!(list.state_count(), 3);
        assert_eq!(list.get(StateId::new(2)), Some(&[][..]));
        assert_eq!(list.edges(), &[]);
    }

    #[test]
    fn an_adjacency_list_without_a_group_holds_nothing() {
        let list = built(0, &[]);

        assert_eq!(list.state_count(), 0);
        assert_eq!(list.get(StateId::new(0)), None);
    }

    #[test]
    #[should_panic(expected = "state 2 is outside an adjacency list of 2 states")]
    fn an_edge_outside_the_adjacency_list_panics() {
        built(2, &[(0, 10), (2, 20)]);
    }

    #[test]
    fn an_adjacency_list_at_its_capacity_builds() {
        let mut builder = AdjacencyListBuilder::with_capacity(2);
        builder.add(StateId::new(0), 10);
        builder.add(StateId::new(0), 20);

        assert_eq!(
            builder.build(1).map(|list| list.edges().to_vec()),
            Ok(vec![10, 20])
        );
    }

    #[test]
    fn an_adjacency_list_past_its_capacity_reports_an_error() {
        let mut builder = AdjacencyListBuilder::with_capacity(2);
        builder.add(StateId::new(0), 10);
        builder.add(StateId::new(0), 20);
        builder.add(StateId::new(0), 30);

        assert_eq!(
            builder.build(1),
            Err(BuildError::TooManyTransitions { capacity: 2 })
        );
    }
}
