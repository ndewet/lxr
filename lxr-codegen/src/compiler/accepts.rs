use crate::automata::{Arena, StateId};

/// The accept of each state of an automaton.
///
/// An automaton knows which states accept. It does not know what an accept
/// means. This table holds the meaning: the accept of the rule that the lexer
/// matched at that state.
///
/// A lexicon gives its rules in the sequence of precedence. Thus
/// [`lowest`](Self::lowest) selects the rule of the highest precedence of the
/// rules that match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accepts<A> {
    accepts: Vec<Option<A>>,
}

impl<A> Accepts<A> {
    /// Creates a table of `count` states, in which each state of `marks`
    /// accepts.
    ///
    /// # Panics
    ///
    /// This function panics if a state of `marks` is not below `count`, or if
    /// two marks name the same state.
    pub(super) fn new(count: usize, marks: Vec<(StateId, A)>) -> Self {
        let mut accepts = Vec::with_capacity(count);
        accepts.resize_with(count, || None);
        for (state, accept) in marks {
            let slot = accepts
                .get_mut(state.index())
                .unwrap_or_else(|| state.outside(count));
            assert!(
                slot.is_none(),
                "state {} already has an accept",
                state.index()
            );
            *slot = Some(accept);
        }
        Self { accepts }
    }

    /// Returns the accept of the state that `id` refers to, or `None` if the
    /// state does not accept.
    ///
    /// # Panics
    ///
    /// This function panics if `id` is not in the table.
    pub fn get(&self, id: StateId) -> Option<&A> {
        self.accepts
            .get(id.index())
            .unwrap_or_else(|| id.outside(self.accepts.len()))
            .as_ref()
    }

    /// Returns the number of the states in the table.
    pub fn state_count(&self) -> usize {
        self.accepts.len()
    }
}

impl<A: Clone + PartialEq> Accepts<A> {
    /// Returns the accepts of the automaton that minimization made, in a table
    /// of `count` states.
    ///
    /// `states` holds the state of that automaton that each state of this
    /// table belongs to.
    /// [`Minimization`](crate::automata::Minimization) gives it. Two states
    /// join only if they accept the same, thus each state of the result takes
    /// the accept of the states that it stands for.
    ///
    /// # Panics
    ///
    /// This function panics if `states` does not hold one state for each state
    /// of the table, if a state of `states` is at or above `count`, if a state
    /// of the result stands for no state, or if two states that joined hold a
    /// different accept.
    pub fn minimized(&self, states: &[StateId], count: usize) -> Self {
        assert_eq!(
            states.len(),
            self.accepts.len(),
            "a table of {} states needs one state for each of them, and not {}",
            self.accepts.len(),
            states.len()
        );

        let mut joined: Vec<Option<Option<A>>> = Vec::with_capacity(count);
        joined.resize_with(count, || None);
        for (accept, state) in self.accepts.iter().zip(states) {
            let slot = joined
                .get_mut(state.index())
                .unwrap_or_else(|| state.outside(count));
            match slot {
                Some(held) => assert!(held == accept, "state {} holds two accepts", state.index()),
                None => *slot = Some(accept.clone()),
            }
        }

        let accepts = joined
            .into_iter()
            .map(|slot| slot.expect("a state of the result stands for at least one state"))
            .collect();
        Self { accepts }
    }
}

impl<A: Ord + Clone> Accepts<A> {
    /// Returns the lowest accept of the states in `states`, or `None` if no
    /// state of `states` accepts.
    ///
    /// The rules of a lexicon are in the sequence of precedence, thus the
    /// lowest accept wins a tie.
    ///
    /// # Panics
    ///
    /// This function panics if a state in `states` is not in the table.
    pub fn lowest(&self, states: &[StateId]) -> Option<A> {
        states.iter().filter_map(|&id| self.get(id)).min().cloned()
    }

    /// Returns the accepts of the automaton that determinization made.
    ///
    /// `subsets` holds the states behind each state of that automaton.
    /// [`Determinization`](crate::automata::Determinization) gives it. The
    /// accept of one state is the lowest accept of its set.
    ///
    /// # Panics
    ///
    /// This function panics if a state in `subsets` is not in the table.
    pub fn determinized(&self, subsets: &Arena<StateId>) -> Self {
        let accepts = (0..subsets.group_count())
            .map(|index| {
                let subset = subsets
                    .get(index)
                    .expect("the index is below the number of the groups");
                self.lowest(subset)
            })
            .collect();
        Self { accepts }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::ArenaBuilder;

    fn state(index: usize) -> StateId {
        StateId::new(index)
    }

    fn accepts() -> Accepts<u32> {
        Accepts::new(4, vec![(state(1), 7), (state(3), 3)])
    }

    /// Builds the subsets of three states from one group of states for each
    /// state.
    fn subsets(groups: &[&[usize]]) -> Arena<StateId> {
        let mut builder = ArenaBuilder::new();
        for (group, states) in groups.iter().enumerate() {
            for &index in *states {
                builder.push(group, state(index));
            }
        }
        builder
            .build(groups.len())
            .expect("a test stays below the capacity")
    }

    #[test]
    fn only_a_state_that_a_mark_names_accepts() {
        let accepts = accepts();

        assert_eq!(accepts.get(state(0)), None);
        assert_eq!(accepts.get(state(1)), Some(&7));
        assert_eq!(accepts.state_count(), 4);
    }

    #[test]
    #[should_panic(expected = "state 9 is outside an arena of 4 states")]
    fn a_read_of_a_state_outside_the_table_panics() {
        accepts().get(state(9));
    }

    #[test]
    #[should_panic(expected = "state 9 is outside an arena of 2 states")]
    fn a_mark_of_a_state_outside_the_table_panics() {
        Accepts::new(2, vec![(state(9), 0)]);
    }

    #[test]
    #[should_panic(expected = "state 1 already has an accept")]
    fn two_marks_of_the_same_state_panic() {
        Accepts::new(2, vec![(state(1), 0), (state(1), 1)]);
    }

    #[test]
    fn the_lowest_accept_of_a_set_is_the_lowest_accept_of_its_states() {
        let accepts = accepts();

        assert_eq!(accepts.lowest(&[state(1), state(3)]), Some(3));
        assert_eq!(accepts.lowest(&[state(1)]), Some(7));
        assert_eq!(accepts.lowest(&[state(0), state(2)]), None);
        assert_eq!(accepts.lowest(&[]), None);
    }

    #[test]
    fn minimization_gives_the_accept_of_the_states_that_joined() {
        let accepts = accepts().minimized(&[state(0), state(1), state(0), state(2)], 3);

        assert_eq!(accepts.state_count(), 3);
        assert_eq!(accepts.get(state(0)), None);
        assert_eq!(accepts.get(state(1)), Some(&7));
        assert_eq!(accepts.get(state(2)), Some(&3));
    }

    #[test]
    #[should_panic(expected = "state 0 holds two accepts")]
    fn two_states_of_a_different_accept_that_joined_panic() {
        accepts().minimized(&[state(0), state(0), state(0), state(0)], 1);
    }

    #[test]
    #[should_panic(expected = "a table of 4 states needs one state for each of them, and not 2")]
    fn a_state_for_each_state_of_the_table_is_needed() {
        accepts().minimized(&[state(0), state(1)], 2);
    }

    #[test]
    #[should_panic(expected = "state 5 is outside an arena of 2 states")]
    fn a_state_outside_the_result_panics() {
        accepts().minimized(&[state(0), state(5), state(0), state(1)], 2);
    }

    #[test]
    fn determinization_gives_the_lowest_accept_of_each_set() {
        let accepts = accepts().determinized(&subsets(&[&[0], &[1, 3], &[3]]));

        assert_eq!(accepts.state_count(), 3);
        assert_eq!(accepts.get(state(0)), None);
        assert_eq!(accepts.get(state(1)), Some(&3));
        assert_eq!(accepts.get(state(2)), Some(&3));
    }
}
