/// A flat store of items, in one group for each index.
///
/// The arena keeps each item of each group in one vector, and the offset of each group in another
/// vector. Thus a group is a slice, and a read of a group makes no allocation.
///
/// To make an `Arena`, use an [`ArenaBuilder`](super::ArenaBuilder).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arena<T> {
    offsets: Vec<u32>,
    items: Vec<T>,
}

impl<T> Arena<T> {
    /// Creates an `Arena` from the offset of each group and the items.
    ///
    /// The offsets hold one more value than the number of the groups. The last value is the
    /// number of the items. The values ascend. [`ArenaBuilder`](super::ArenaBuilder) makes them.
    pub(super) fn new(offsets: Vec<u32>, items: Vec<T>) -> Self {
        debug_assert!(
            offsets.last() == Some(&(items.len() as u32)),
            "the last offset is the number of the items"
        );
        Self { offsets, items }
    }

    /// Returns the items of the group at `index`, or `None` if the arena has no such group.
    pub fn get(&self, index: usize) -> Option<&[T]> {
        let start = *self.offsets.get(index)? as usize;
        let end = *self.offsets.get(index + 1)? as usize;
        Some(&self.items[start..end])
    }

    /// Returns the number of the groups in the arena.
    pub fn group_count(&self) -> usize {
        self.offsets.len() - 1
    }

    /// Returns each item of the arena, the items of the first group first.
    pub fn items(&self) -> &[T] {
        &self.items
    }
}

impl<T> Default for Arena<T> {
    /// Creates an `Arena` that holds no group and no item.
    fn default() -> Self {
        Self {
            offsets: vec![0],
            items: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arena() -> Arena<u32> {
        Arena::new(vec![0, 1, 1, 4], vec![10, 20, 30, 40])
    }

    #[test]
    fn a_group_is_the_slice_from_its_offset_to_the_next_offset() {
        assert_eq!(arena().get(0), Some(&[10][..]));
        assert_eq!(arena().get(2), Some(&[20, 30, 40][..]));
    }

    #[test]
    fn a_group_without_an_item_is_an_empty_slice() {
        assert_eq!(arena().get(1), Some(&[][..]));
    }

    #[test]
    fn the_arena_has_one_group_less_than_it_has_offsets() {
        assert_eq!(arena().group_count(), 3);
    }

    #[test]
    fn a_group_outside_the_arena_gives_nothing() {
        assert_eq!(arena().get(3), None);
        assert_eq!(arena().get(9), None);
    }

    #[test]
    fn the_arena_gives_each_item_in_one_slice() {
        assert_eq!(arena().items(), &[10, 20, 30, 40]);
    }

    #[test]
    fn the_default_arena_holds_no_group() {
        let arena = Arena::<u32>::default();

        assert_eq!(arena.group_count(), 0);
        assert_eq!(arena.get(0), None);
        assert_eq!(arena.items(), &[]);
    }
}
