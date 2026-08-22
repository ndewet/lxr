use super::execution::DeterministicExecution;
use crate::automata::arena::Arena;
use crate::automata::automaton::{Automaton, Transition};
use crate::automata::id::StateId;
use crate::automata::label::Label;
use crate::automata::scanner::Scanner;
use crate::automata::table::StateTable;

/// A deterministic finite automaton.
///
/// The automaton holds states, transitions with a label of type `L`, the states that accept, and
/// one or more start states. It holds no epsilon transition. The labels of one state match no
/// symbol in common, and they are in ascending sequence. Thus a scan is in one state, or in no
/// state.
///
/// The automaton holds only the states that a scan reaches. A state that reads a symbol that no
/// label of that state matches goes to no state, and the scan stops. Thus the automaton needs no
/// trap state. A state from which the automaton reaches no accept stays in the automaton.
///
/// To make a `DeterministicFiniteAutomaton`, use
/// [`determinize`](crate::automata::NondeterministicFiniteAutomaton::determinize).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeterministicFiniteAutomaton<L> {
    table: StateTable<L>,
}

impl<L> DeterministicFiniteAutomaton<L> {
    /// Creates a `DeterministicFiniteAutomaton` from the transitions, the accepts, and the start
    /// states.
    ///
    /// The caller gives the transitions of one state in ascending sequence, with labels that match
    /// no symbol in common. The constructor does not check that, because the check costs one
    /// comparison for each pair of the labels. [`step`](Self::step) checks it with a
    /// `debug_assert!`.
    ///
    /// # Panics
    ///
    /// This function panics for each of these conditions:
    ///
    /// - The arena holds a group for a state that `accepts` does not hold.
    /// - `starts` is empty, or a start state is not in the state arena.
    /// - The target of a transition is not in the state arena.
    pub(in crate::automata) fn new(
        transitions: Arena<Transition<L>>,
        accepts: Vec<bool>,
        starts: Vec<StateId>,
    ) -> Self {
        Self {
            table: StateTable::new(transitions, accepts, starts),
        }
    }
}

impl<L: Label> DeterministicFiniteAutomaton<L> {
    /// Reads `symbol` at the state that `from` refers to, then returns the state that the
    /// automaton goes to.
    ///
    /// The result is `None` if no label of that state matches `symbol`.
    ///
    /// The transitions of one state are in ascending sequence, thus the function finds the
    /// transition with a binary search and reads no other transition.
    ///
    /// # Panics
    ///
    /// This function panics if `from` is not in the state arena.
    #[allow(dead_code, reason = "the tests scan an automaton with this API")]
    pub fn step(&self, from: StateId, symbol: L::Symbol) -> Option<StateId> {
        let transitions = self.transitions(from);
        let index = transitions.partition_point(|transition| transition.label.below(symbol));
        let target = transitions
            .get(index)
            .filter(|transition| transition.label.matches(symbol))
            .map(|transition| transition.target);

        debug_assert!(
            {
                let mut matching = transitions
                    .iter()
                    .filter(|transition| transition.label.matches(symbol));
                matching.next().map(|transition| transition.target) == target
                    && matching.next().is_none()
            },
            "state {} holds its transitions out of sequence, or two of them match one symbol",
            from.index()
        );

        target
    }
}

impl<L> Automaton for DeterministicFiniteAutomaton<L> {
    type Label = L;

    fn state_count(&self) -> usize {
        self.table.state_count()
    }

    /// Returns the transitions that leave the state that `id` refers to.
    ///
    /// The labels of the result match no symbol in common.
    fn transitions(&self, id: StateId) -> &[Transition<L>] {
        self.table.transitions(id)
    }

    fn accepts(&self, id: StateId) -> bool {
        self.table.accepts(id)
    }

    fn start_states(&self) -> &[StateId] {
        self.table.start_states()
    }
}

impl<L: Label> Scanner for DeterministicFiniteAutomaton<L> {
    type Symbol = L::Symbol;
    type Execution<'a>
        = DeterministicExecution<'a, L>
    where
        Self: 'a;

    fn execute(&self) -> DeterministicExecution<'_, L> {
        DeterministicExecution::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::testing::{chain, dfa, only, range};

    #[test]
    fn the_automaton_gives_the_transitions_of_a_state() {
        let dfa = chain();

        assert_eq!(dfa.transitions(StateId::new(0)).len(), 1);
        assert_eq!(dfa.transitions(StateId::new(0))[0].target, StateId::new(1));
        assert_eq!(dfa.transitions(StateId::new(0))[0].label, only('a'));
        assert_eq!(dfa.transitions(StateId::new(2)), &[]);
    }

    #[test]
    fn the_automaton_says_which_states_accept() {
        let dfa = chain();

        assert!(!dfa.accepts(StateId::new(0)));
        assert!(dfa.accepts(StateId::new(1)));
        assert_eq!(dfa.state_count(), 3);
    }

    #[test]
    #[should_panic(expected = "state 9 is outside an arena of 3 states")]
    fn a_read_of_the_transitions_of_a_state_outside_the_arena_panics() {
        chain().transitions(StateId::new(9));
    }

    #[test]
    #[should_panic(expected = "state 9 is outside an arena of 3 states")]
    fn a_read_of_the_accept_of_a_state_outside_the_arena_panics() {
        chain().accepts(StateId::new(9));
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
            &[false, true, true],
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
    #[cfg(debug_assertions)]
    #[should_panic(expected = "state 0 holds its transitions out of sequence")]
    fn a_step_at_a_state_that_holds_its_transitions_out_of_sequence_panics() {
        let dfa = dfa(
            &[&[(range('d', 'f'), 1), (range('a', 'c'), 2)], &[], &[]],
            &[false, true, true],
            &[0],
        );

        dfa.step(StateId::new(0), 'b');
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "or two of them match one symbol")]
    fn a_step_at_a_state_that_holds_two_transitions_for_one_symbol_panics() {
        let dfa = dfa(
            &[&[(range('a', 'c'), 1), (range('a', 'f'), 2)], &[], &[]],
            &[false, true, true],
            &[0],
        );

        dfa.step(StateId::new(0), 'b');
    }

    #[test]
    fn each_start_names_its_own_state() {
        let dfa = dfa(&[&[], &[]], &[true, true], &[1, 0]);

        assert_eq!(dfa.start_count(), 2);
        assert_eq!(dfa.start_state(0), StateId::new(1));
        assert_eq!(dfa.start_state(1), StateId::new(0));
    }

    #[test]
    #[should_panic(expected = "start 2 is outside an automaton with 1 start states")]
    fn a_start_that_the_automaton_does_not_have_panics() {
        chain().start_state(2);
    }

    #[test]
    #[should_panic(expected = "state 0 points at 9")]
    fn a_transition_that_points_outside_the_arena_panics() {
        dfa(&[&[(only('a'), 9)]], &[false], &[0]);
    }

    #[test]
    #[should_panic(expected = "at least one start state")]
    fn an_automaton_without_a_start_panics() {
        dfa(&[&[]], &[false], &[]);
    }
}
