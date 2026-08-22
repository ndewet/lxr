use super::overflow::{Overflow, Part};

/// A flat store of items, in one group for each index.
///
/// The arena keeps each item of each group in one vector, and the offset of each group in another
/// vector. Thus a group is a slice, and a read of a group makes no allocation.
///
/// To make an `Arena`, use an [`ArenaBuilder`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arena<T> {
    offsets: Vec<u32>,
    items: Vec<T>,
}

impl<T> Arena<T> {
    /// Creates an `Arena` from the offset of each group and the items.
    ///
    /// The offsets hold one more value than the number of the groups. The last value is the
    /// number of the items. The values ascend. [`ArenaBuilder`] makes them.
    fn new(offsets: Vec<u32>, items: Vec<T>) -> Self {
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
    #[cfg(test)]
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

/// An [`Arena`] that is not complete.
///
/// Add the items in any sequence, then build the arena with [`build`](Self::build).
#[derive(Debug)]
pub struct ArenaBuilder<T> {
    entries: Vec<(u32, T)>,
    capacity: usize,
}

impl<T> ArenaBuilder<T> {
    /// The number of the items that an [`Arena`] holds.
    ///
    /// The last offset of an arena is the number of the items, and an offset is a `u32`.
    pub const CAPACITY: usize = u32::MAX as usize;

    /// Creates an `ArenaBuilder` that holds no item.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an `ArenaBuilder` that holds at most `capacity` items.
    ///
    /// The tests need a capacity below [`CAPACITY`](Self::CAPACITY).
    fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::new(),
            capacity,
        }
    }

    /// Adds `item` to the group at `group`.
    ///
    /// # Panics
    ///
    /// This function panics if `group` is above `u32::MAX`.
    pub fn push(&mut self, group: usize, item: T) {
        let group = u32::try_from(group).expect("an arena holds at most u32::MAX + 1 groups");
        self.entries.push((group, item));
    }

    /// Builds an [`Arena`] of `group_count` groups.
    ///
    /// The items of one group stay in the sequence in which you added them.
    ///
    /// # Errors
    ///
    /// This function returns an [`Overflow`] if the builder holds more than
    /// [`CAPACITY`](Self::CAPACITY) items.
    ///
    /// # Panics
    ///
    /// This function panics if the group of an item is not below `group_count`.
    pub fn build(self, group_count: usize) -> Result<Arena<T>, Overflow> {
        if self.entries.len() > self.capacity {
            return Err(Overflow::new(Part::Items, self.capacity));
        }

        let mut offsets = vec![0u32; group_count + 1];
        for &(group, _) in &self.entries {
            let index = group as usize;
            assert!(
                index < group_count,
                "group {index} is outside an arena of {group_count} groups"
            );
            offsets[index + 1] += 1;
        }
        for index in 1..offsets.len() {
            offsets[index] += offsets[index - 1];
        }

        let mut cursors = offsets.clone();
        let mut slots: Vec<Option<T>> = (0..self.entries.len()).map(|_| None).collect();
        for (group, item) in self.entries {
            let cursor = &mut cursors[group as usize];
            slots[*cursor as usize] = Some(item);
            *cursor += 1;
        }

        let items = slots
            .into_iter()
            .map(|slot| slot.expect("the offsets cover each slot one time"))
            .collect();
        Ok(Arena::new(offsets, items))
    }
}

impl<T> Default for ArenaBuilder<T> {
    /// Creates an `ArenaBuilder` that holds no item.
    fn default() -> Self {
        Self::with_capacity(Self::CAPACITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An arena of three groups, from the offsets and the items directly.
    fn arena() -> Arena<u32> {
        Arena::new(vec![0, 1, 1, 4], vec![10, 20, 30, 40])
    }

    /// An arena of `groups` groups, from the group and the item of each entry.
    fn built(groups: usize, entries: &[(usize, u32)]) -> Arena<u32> {
        let mut builder = ArenaBuilder::new();
        for &(group, item) in entries {
            builder.push(group, item);
        }
        builder
            .build(groups)
            .expect("the arena is below its capacity")
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

    #[test]
    fn a_group_keeps_the_sequence_in_which_the_items_arrived() {
        let arena = built(2, &[(1, 30), (0, 10), (1, 20), (1, 10)]);

        assert_eq!(arena.get(0), Some(&[10][..]));
        assert_eq!(arena.get(1), Some(&[30, 20, 10][..]));
    }

    #[test]
    fn a_group_that_gets_no_item_is_empty() {
        let arena = built(3, &[(2, 10)]);

        assert_eq!(arena.get(0), Some(&[][..]));
        assert_eq!(arena.get(1), Some(&[][..]));
        assert_eq!(arena.get(2), Some(&[10][..]));
    }

    #[test]
    fn the_arena_holds_the_items_of_the_first_group_first() {
        let arena = built(2, &[(1, 30), (0, 10), (1, 20)]);

        assert_eq!(arena.items(), &[10, 30, 20]);
    }

    #[test]
    fn an_arena_without_an_item_keeps_its_groups() {
        let arena = built(3, &[]);

        assert_eq!(arena.group_count(), 3);
        assert_eq!(arena.get(2), Some(&[][..]));
        assert_eq!(arena.items(), &[]);
    }

    #[test]
    fn an_arena_without_a_group_holds_nothing() {
        let arena = built(0, &[]);

        assert_eq!(arena.group_count(), 0);
        assert_eq!(arena.get(0), None);
    }

    #[test]
    #[should_panic(expected = "group 2 is outside an arena of 2 groups")]
    fn an_item_outside_the_arena_panics() {
        built(2, &[(0, 10), (2, 20)]);
    }

    #[test]
    fn an_arena_at_its_capacity_builds() {
        let mut builder = ArenaBuilder::with_capacity(2);
        builder.push(0, 10);
        builder.push(0, 20);

        assert_eq!(
            builder.build(1).map(|arena| arena.items().to_vec()),
            Ok(vec![10, 20])
        );
    }

    #[test]
    fn an_arena_past_its_capacity_reports_an_overflow() {
        let mut builder = ArenaBuilder::with_capacity(2);
        builder.push(0, 10);
        builder.push(0, 20);
        builder.push(0, 30);

        assert_eq!(builder.build(1), Err(Overflow::new(Part::Items, 2)));
    }
}
