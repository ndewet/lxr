use super::{Execution, Matcher};
use crate::automata::Transition;
use crate::automata::adjacency::AdjacencyList;
use crate::automata::id::StateId;
use crate::automata::label::Label;
use crate::automata::table::StateTable;

/// A nondeterministic finite automaton.
///
/// The automaton holds states, transitions with a label of type `L`, epsilon transitions, the
/// states that accept, and one or more start states.
///
/// The automaton does not know the alphabet, and it does not interpret an
/// accept value. The caller selects the label and accept types. A lexer, for
/// example, uses a byte range as the label and a rule identifier as the accept.
///
/// The default accept type is `()`. It records only membership in the set of
/// accepting states.
///
/// To make an `Nfa`, use a [`Builder`](super::Builder).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nfa<L, A = ()> {
    table: StateTable<L, A>,
    epsilons: AdjacencyList<StateId>,
}

impl<L, A> Nfa<L, A> {
    /// Creates an NFA from a state table and epsilon transitions.
    ///
    /// # Panics
    ///
    /// This function panics for each of these conditions:
    ///
    /// - The epsilon list does not have one group for each state.
    /// - An epsilon-transition target is outside the table.
    pub(super) fn new(table: StateTable<L, A>, epsilons: AdjacencyList<StateId>) -> Self {
        let count = table.state_count();
        assert_eq!(
            epsilons.state_count(),
            count,
            "an automaton needs one group of epsilon transitions for each of its {count} states"
        );
        for index in 0..count {
            let state = StateId::new(index);
            for &target in epsilons.get(state).into_iter().flatten() {
                table.check_target(state, target);
            }
        }

        Self { table, epsilons }
    }

    /// Returns the targets of the epsilon transitions from `state`.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not in the automaton.
    pub fn epsilon_targets(&self, state: StateId) -> &[StateId] {
        self.epsilons
            .get(state)
            .unwrap_or_else(|| state.outside(self.state_count()))
    }

    /// Returns the number of states in the automaton.
    pub fn state_count(&self) -> usize {
        self.table.state_count()
    }

    /// Returns the labeled transitions that leave `state`.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not in the automaton.
    pub fn transitions(&self, state: StateId) -> &[Transition<L>] {
        self.table.transitions(state)
    }

    /// Returns `true` if `state` accepts.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not in the automaton.
    pub fn accepts(&self, state: StateId) -> bool {
        self.table.accepts(state)
    }

    /// Returns the accept value of `state`, if the state accepts.
    ///
    /// # Panics
    ///
    /// This function panics if `state` is not in the automaton.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr_codegen::automata::encoding::ByteRange;
    /// use lxr_codegen::automata::nfa::Builder;
    ///
    /// let mut builder = Builder::<ByteRange, &str>::new();
    /// let state = builder.add_state();
    /// builder.set_accept(state, "identifier");
    /// let nfa = builder.build(&[state]).unwrap();
    ///
    /// assert_eq!(nfa.accept(state), Some(&"identifier"));
    /// ```
    pub fn accept(&self, state: StateId) -> Option<&A> {
        self.table.accept(state)
    }

    /// Returns the start states in their declared sequence.
    pub fn start_states(&self) -> &[StateId] {
        self.table.start_states()
    }

    /// Returns the start state at `index`.
    ///
    /// # Panics
    ///
    /// This function panics if `index` is outside the start states.
    pub fn start_state(&self, index: usize) -> StateId {
        self.table.start_state(index)
    }
}

impl<L: Label, A> Nfa<L, A> {
    /// Reads `symbol` at each state in `states`, then returns each state that the automaton goes
    /// to.
    ///
    /// The function does not follow epsilon transitions. [`Execution::step`]
    /// follows the epsilon closure after each symbol.
    ///
    /// The result holds one state for each transition that matches, thus it can hold a duplicate.
    ///
    /// # Panics
    ///
    /// The result panics at a state in `states` that is not in the automaton. The result is an
    /// iterator, thus the panic comes when the caller reads that state.
    pub fn step<'a>(
        &'a self,
        states: &'a [StateId],
        symbol: L::Symbol,
    ) -> impl Iterator<Item = StateId> + 'a {
        states.iter().flat_map(move |&id| {
            self.transitions(id)
                .iter()
                .filter(move |transition| transition.label.matches(symbol))
                .map(|transition| transition.target)
        })
    }

    /// Creates an execution of this automaton in no state.
    ///
    /// The automaton is read only. An [`Execution`] holds one epsilon-closed
    /// state set and moves it one symbol at a time.
    pub fn execution(&self) -> Execution<'_, L, A> {
        Execution::new(self)
    }

    /// Creates a reusable longest-match scanner for this automaton.
    ///
    /// A [`Matcher`] applies the longest-match policy to an [`Execution`]. It
    /// reuses the execution buffers between calls.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr_codegen::automata::encoding::ByteRange;
    /// use lxr_codegen::automata::nfa::Builder;
    ///
    /// let mut builder = Builder::<ByteRange>::new();
    /// let start = builder.add_state();
    /// let accept = builder.add_state();
    /// builder.add_transition(
    ///     start,
    ///     ByteRange {
    ///         low: b'a',
    ///         high: b'a',
    ///     },
    ///     accept,
    /// );
    /// builder.mark_accept(accept);
    /// let nfa = builder.build(&[start]).unwrap();
    ///
    /// let found = nfa.matcher().longest_match(0, b"ab", |_| "a").unwrap();
    /// assert_eq!(found.accept, "a");
    /// assert_eq!(found.length, 1);
    /// ```
    pub fn matcher(&self) -> Matcher<'_, L, A> {
        Matcher::new(self)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::automata::testing::{Symbols, builder, only, range};

    fn stepped(nfa: &Nfa<Symbols>, states: &[StateId], symbol: char) -> Vec<StateId> {
        nfa.step(states, symbol).collect()
    }

    #[test]
    fn a_step_follows_a_transition_that_matches_the_symbol() {
        let mut builder = builder();
        let start = builder.add_state();
        let accept = builder.add_state();
        builder.add_transition(start, range('a', 'z'), accept);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        assert_eq!(stepped(&nfa, &[start], 'm'), vec![accept]);
        assert_eq!(stepped(&nfa, &[start], 'A'), Vec::new());
    }

    #[test]
    fn a_step_does_not_follow_an_epsilon_transition() {
        let mut builder = builder();
        let start = builder.add_state();
        let accept = builder.add_state();
        builder.add_epsilon_transition(start, accept);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        assert_eq!(stepped(&nfa, &[start], 'a'), Vec::new());
    }

    #[test]
    fn a_step_yields_a_shared_target_one_time_for_each_transition() {
        let mut builder = builder();
        let start = builder.add_state();
        let accept = builder.add_state();
        builder.add_transition(start, range('a', 'z'), accept);
        builder.add_transition(start, only('e'), accept);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        assert_eq!(stepped(&nfa, &[start], 'e'), vec![accept, accept]);
    }

    #[test]
    fn a_state_keeps_its_transitions_in_the_sequence_in_which_they_arrived() {
        let mut builder = builder();
        let start = builder.add_state();
        let first = builder.add_state();
        let second = builder.add_state();
        builder.add_transition(start, only('b'), second);
        builder.add_transition(start, only('a'), first);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        assert_eq!(
            nfa.transitions(start),
            &[
                Transition {
                    label: only('b'),
                    target: second
                },
                Transition {
                    label: only('a'),
                    target: first
                },
            ]
        );
        assert_eq!(nfa.transitions(first), &[]);
    }

    #[test]
    fn a_state_keeps_its_epsilon_transitions() {
        let mut builder = builder();
        let start = builder.add_state();
        let first = builder.add_state();
        let second = builder.add_state();
        builder.add_epsilon_transition(start, second);
        builder.add_epsilon_transition(start, first);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        assert_eq!(nfa.epsilon_targets(start), &[second, first]);
        assert_eq!(nfa.epsilon_targets(first), &[]);
    }

    #[test]
    fn only_a_state_that_the_builder_marked_accepts() {
        let mut builder = builder();
        let start = builder.add_state();
        let accept = builder.add_state();
        builder.mark_accept(accept);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        assert!(!nfa.accepts(start));
        assert!(nfa.accepts(accept));
        assert_eq!(nfa.state_count(), 2);
    }

    #[test]
    fn each_start_names_its_own_state() {
        let mut builder = builder();
        let code = builder.add_state();
        let string = builder.add_state();
        let nfa = builder
            .build(&[code, string])
            .expect("the builder is below its capacity");

        assert_eq!(nfa.start_states(), &[code, string]);
        assert_eq!(nfa.start_state(0), code);
        assert_eq!(nfa.start_state(1), string);
    }

    #[test]
    #[should_panic(expected = "start 2 is outside an automaton with 2 start states")]
    fn reading_a_start_outside_the_automaton_panics() {
        let mut builder = builder();
        let code = builder.add_state();
        let string = builder.add_state();
        let nfa = builder
            .build(&[code, string])
            .expect("the builder is below its capacity");

        nfa.start_state(2);
    }

    #[test]
    #[should_panic(expected = "state 9 is outside an automaton of 2 states")]
    fn reading_the_transitions_of_a_state_outside_the_arena_panics() {
        let mut builder = builder();
        let start = builder.add_state();
        builder.add_state();
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        nfa.transitions(StateId::new(9));
    }

    #[test]
    #[should_panic(expected = "state 9 is outside an automaton of 2 states")]
    fn reading_the_accept_of_a_state_outside_the_arena_panics() {
        let mut builder = builder();
        let start = builder.add_state();
        builder.add_state();
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        nfa.accepts(StateId::new(9));
    }
}
