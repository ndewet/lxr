use super::execution::DfaExecution;
use crate::automata::arena::Arena;
use crate::automata::automaton::Automaton;
use crate::automata::id::{StartId, StateId};
use crate::automata::label::Label;
use crate::automata::transition::Transition;

/// A deterministic finite automaton.
///
/// The automaton holds states, transitions with a label of type `L`, an accept of type `A` for
/// each state that accepts, and one or more start states. It holds no epsilon transition, and the
/// labels of one state match no symbol in common. Thus a scan is in one state, or in no state.
///
/// The automaton holds no dead state. A state that reads a symbol that no label of that state
/// matches goes to no state, and the scan stops. Thus the automaton holds only the states that a
/// scan reaches.
///
/// To make a `Dfa`, use [`Nfa::determinize`](crate::automata::Nfa::determinize), or use a
/// [`DfaBuilder`](super::DfaBuilder).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dfa<L, A> {
    transitions: Arena<Transition<L>>,
    accepts: Vec<Option<A>>,
    starts: Vec<StateId>,
}

impl<L, A> Dfa<L, A> {
    /// Creates a `Dfa` from the transitions, the accepts, and the start states.
    #[expect(clippy::todo, unused_variables, reason = "step 3 of the plan")]
    pub(super) fn new(
        transitions: Arena<Transition<L>>,
        accepts: Vec<Option<A>>,
        starts: Vec<StateId>,
    ) -> Self {
        todo!()
    }

    /// Returns the transitions that leave the state that `id` refers to.
    ///
    /// The labels of the result match no symbol in common.
    ///
    /// # Panics
    ///
    /// This function panics if `id` is not in the state arena.
    #[expect(clippy::todo, unused_variables, reason = "step 3 of the plan")]
    pub fn transitions(&self, id: StateId) -> &[Transition<L>] {
        todo!()
    }

    /// Returns the accept of the state that `id` refers to, or `None` if the state does not
    /// accept.
    ///
    /// # Panics
    ///
    /// This function panics if `id` is not in the state arena.
    #[expect(clippy::todo, unused_variables, reason = "step 3 of the plan")]
    pub fn accept(&self, id: StateId) -> Option<&A> {
        todo!()
    }

    /// Returns the number of the states in the state arena.
    #[expect(clippy::todo, reason = "step 3 of the plan")]
    pub fn state_count(&self) -> usize {
        todo!()
    }

    /// Returns the number of the start states of the automaton.
    #[expect(clippy::todo, reason = "step 3 of the plan")]
    pub fn start_count(&self) -> usize {
        todo!()
    }

    /// Returns the state at which the automaton starts a scan under `start`.
    ///
    /// # Panics
    ///
    /// This function panics if `start` is not a start state of this automaton.
    #[expect(clippy::todo, unused_variables, reason = "step 3 of the plan")]
    pub fn start_state(&self, start: StartId) -> StateId {
        todo!()
    }

    /// Returns an iterator over the start identifiers and their states.
    #[expect(
        clippy::todo,
        unreachable_code,
        reason = "step 3 of the plan. An opaque return type needs a value, thus the empty iterator \
                  stays until the body lands."
    )]
    pub fn starts(&self) -> impl Iterator<Item = (StartId, StateId)> + '_ {
        todo!();
        std::iter::empty()
    }

    fn outside(&self, id: StateId) -> ! {
        panic!(
            "state {} is outside an arena of {} states",
            id.index(),
            self.state_count()
        )
    }
}

impl<L: Label, A> Dfa<L, A> {
    /// Reads `symbol` at the state that `from` refers to, then returns the state that the
    /// automaton goes to.
    ///
    /// The result is `None` if no label of that state matches `symbol`.
    ///
    /// # Panics
    ///
    /// This function panics if `from` is not in the state arena.
    #[expect(clippy::todo, unused_variables, reason = "step 3 of the plan")]
    pub fn step(&self, from: StateId, symbol: L::Symbol) -> Option<StateId> {
        todo!()
    }
}

impl<L: Label, A> Automaton for Dfa<L, A> {
    type Symbol = L::Symbol;
    type Accept = A;
    type Execution<'a>
        = DfaExecution<'a, L, A>
    where
        Self: 'a;

    #[expect(clippy::todo, unused_variables, reason = "step 5 of the plan")]
    fn execute(&self, start: StartId) -> Self::Execution<'_> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::arena_builder::ArenaBuilder;
    use crate::automata::testing::{Symbols, only, range};

    /// Builds a `Dfa` from one group of transitions for each state, the accepts, and the starts.
    fn dfa(
        transitions: &[&[(Symbols, usize)]],
        accepts: &[Option<u32>],
        starts: &[usize],
    ) -> Dfa<Symbols, u32> {
        let mut arena = ArenaBuilder::new();
        for (state, group) in transitions.iter().enumerate() {
            for &(label, target) in *group {
                arena.push(
                    state,
                    Transition {
                        label,
                        target: StateId::new(target),
                    },
                );
            }
        }
        let arena = arena
            .build(accepts.len())
            .expect("a test stays below the capacity");
        Dfa::new(
            arena,
            accepts.to_vec(),
            starts.iter().map(|&index| StateId::new(index)).collect(),
        )
    }

    /// Builds the automaton that matches `"a"` as accept 1, and `"ab"` as accept 0.
    fn chain() -> Dfa<Symbols, u32> {
        dfa(
            &[&[(only('a'), 1)], &[(only('b'), 2)], &[]],
            &[None, Some(1), Some(0)],
            &[0],
        )
    }

    #[test]
    fn the_automaton_gives_the_transitions_of_a_state() {
        let dfa = chain();

        assert_eq!(dfa.transitions(StateId::new(0)).len(), 1);
        assert_eq!(dfa.transitions(StateId::new(0))[0].target, StateId::new(1));
        assert_eq!(dfa.transitions(StateId::new(0))[0].label, only('a'));
    }

    #[test]
    fn a_state_without_a_transition_gives_an_empty_slice() {
        assert_eq!(chain().transitions(StateId::new(2)), &[]);
    }

    #[test]
    fn the_automaton_gives_the_accept_of_a_state() {
        let dfa = chain();

        assert_eq!(dfa.accept(StateId::new(1)), Some(&1));
        assert_eq!(dfa.accept(StateId::new(2)), Some(&0));
    }

    #[test]
    fn a_state_that_does_not_accept_gives_nothing() {
        assert_eq!(chain().accept(StateId::new(0)), None);
    }

    #[test]
    fn the_automaton_counts_its_states() {
        assert_eq!(chain().state_count(), 3);
    }

    #[test]
    #[should_panic(expected = "state 9 is outside an arena of 3 states")]
    fn a_read_of_the_transitions_of_a_state_outside_the_arena_panics() {
        chain().transitions(StateId::new(9));
    }

    #[test]
    #[should_panic(expected = "state 9 is outside an arena of 3 states")]
    fn a_read_of_the_accept_of_a_state_outside_the_arena_panics() {
        chain().accept(StateId::new(9));
    }

    #[test]
    fn a_step_gives_the_target_of_the_label_that_matches() {
        let dfa = chain();

        assert_eq!(dfa.step(StateId::new(0), 'a'), Some(StateId::new(1)));
        assert_eq!(dfa.step(StateId::new(1), 'b'), Some(StateId::new(2)));
    }

    #[test]
    fn a_step_on_a_symbol_that_no_label_matches_gives_nothing() {
        let dfa = chain();

        assert_eq!(dfa.step(StateId::new(0), 'b'), None);
        assert_eq!(dfa.step(StateId::new(2), 'a'), None);
    }

    #[test]
    fn a_step_selects_the_label_that_holds_the_symbol() {
        let dfa = dfa(
            &[&[(range('a', 'c'), 1), (range('d', 'f'), 2)], &[], &[]],
            &[None, Some(0), Some(1)],
            &[0],
        );

        assert_eq!(dfa.step(StateId::new(0), 'b'), Some(StateId::new(1)));
        assert_eq!(dfa.step(StateId::new(0), 'e'), Some(StateId::new(2)));
        assert_eq!(dfa.step(StateId::new(0), 'g'), None);
    }

    #[test]
    #[should_panic(expected = "state 9 is outside an arena of 3 states")]
    fn a_step_at_a_state_outside_the_arena_panics() {
        chain().step(StateId::new(9), 'a');
    }

    #[test]
    fn the_automaton_counts_its_start_states() {
        let dfa = dfa(&[&[], &[]], &[Some(0), Some(1)], &[0, 1]);

        assert_eq!(dfa.start_count(), 2);
        assert_eq!(dfa.start_state(StartId::new(0)), StateId::new(0));
        assert_eq!(dfa.start_state(StartId::new(1)), StateId::new(1));
    }

    #[test]
    fn the_automaton_gives_each_start_with_its_state() {
        let dfa = dfa(&[&[], &[]], &[Some(0), Some(1)], &[1, 0]);

        assert_eq!(
            dfa.starts().collect::<Vec<_>>(),
            vec![
                (StartId::new(0), StateId::new(1)),
                (StartId::new(1), StateId::new(0)),
            ]
        );
    }

    #[test]
    #[should_panic(expected = "start 2 is outside an automaton with 1 start states")]
    fn a_start_that_the_automaton_does_not_have_panics() {
        chain().start_state(StartId::new(2));
    }
}
