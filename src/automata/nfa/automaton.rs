use super::execution::NondeterministicExecution;
use crate::automata::arena::Arena;
use crate::automata::automaton::{Automaton, Transition};
use crate::automata::execution::Execution;
use crate::automata::id::StateId;
use crate::automata::label::Label;
use crate::automata::scanner::Scanner;
use crate::automata::table::StateTable;

/// A nondeterministic finite automaton.
///
/// The automaton holds states, transitions with a label of type `L`, epsilon transitions, the
/// states that accept, and one or more start states.
///
/// The automaton does not know the alphabet, and it does not know what an accept means. The caller
/// selects the label, and the caller holds the meaning of each state that accepts. A lexer, for
/// example, uses a byte range as the label and holds a token for each state that accepts.
///
/// To make an `Nfa`, use an [`NfaBuilder`](super::NfaBuilder).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NondeterministicFiniteAutomaton<L> {
    table: StateTable<L>,
    epsilons: Arena<StateId>,
}

impl<L> NondeterministicFiniteAutomaton<L> {
    /// Creates an `Nfa` from the transitions, the epsilon transitions, the accepts, and the start
    /// states.
    ///
    /// # Panics
    ///
    /// This function panics for each of these conditions:
    ///
    /// - An arena holds a group for a state that `accepts` does not hold.
    /// - `starts` is empty, or a start state is not in the state arena.
    /// - The target of a transition is not in the state arena.
    pub(super) fn new(
        transitions: Arena<Transition<L>>,
        epsilons: Arena<StateId>,
        accepts: Vec<bool>,
        starts: Vec<StateId>,
    ) -> Self {
        let table = StateTable::new(transitions, accepts, starts);
        let count = table.state_count();
        assert_eq!(
            epsilons.group_count(),
            count,
            "an automaton needs one group of epsilon transitions for each of its {count} states"
        );
        for index in 0..count {
            for &target in epsilons.get(index).into_iter().flatten() {
                table.check_target(index, target);
            }
        }

        Self { table, epsilons }
    }

    /// Returns the states that the state that `id` refers to goes to without a symbol.
    ///
    /// # Panics
    ///
    /// This function panics if `id` is not in the state arena.
    pub fn epsilons(&self, id: StateId) -> &[StateId] {
        self.epsilons
            .get(id.index())
            .unwrap_or_else(|| self.table.outside(id))
    }
}

impl<L: Label> NondeterministicFiniteAutomaton<L> {
    /// Reads `symbol` at each state in `states`, then returns each state that the automaton goes
    /// to.
    ///
    /// The function does not follow the epsilon transitions. To follow them, seed an
    /// [`NondeterministicExecution`] with the result.
    ///
    /// The result holds one state for each transition that matches, thus it can hold a duplicate.
    ///
    /// # Panics
    ///
    /// The result panics at a state in `states` that is not in the state arena. The result is an
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
}

impl<L> Automaton for NondeterministicFiniteAutomaton<L> {
    type Label = L;

    fn state_count(&self) -> usize {
        self.table.state_count()
    }

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

impl<L: Label> Scanner for NondeterministicFiniteAutomaton<L> {
    type Symbol = L::Symbol;
    type Execution<'a>
        = NondeterministicExecution<'a, L>
    where
        Self: 'a;

    fn execute(&self, start: usize) -> NondeterministicExecution<'_, L> {
        let mut execution = NondeterministicExecution::new(self);
        execution.restart(start);
        execution
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::automata::testing::{Symbols, builder, only, range};

    fn stepped(
        nfa: &NondeterministicFiniteAutomaton<Symbols>,
        states: &[StateId],
        symbol: char,
    ) -> Vec<StateId> {
        nfa.step(states, symbol).collect()
    }

    #[test]
    fn a_step_follows_a_transition_that_matches_the_symbol() {
        let mut builder = builder();
        let start = builder.push();
        let accept = builder.push();
        builder.transition(start, range('a', 'z'), accept);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        assert_eq!(stepped(&nfa, &[start], 'm'), vec![accept]);
        assert_eq!(stepped(&nfa, &[start], 'A'), Vec::new());
    }

    #[test]
    fn a_step_does_not_follow_an_epsilon_transition() {
        let mut builder = builder();
        let start = builder.push();
        let accept = builder.push();
        builder.epsilon(start, accept);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        assert_eq!(stepped(&nfa, &[start], 'a'), Vec::new());
    }

    #[test]
    fn a_step_yields_a_shared_target_one_time_for_each_transition() {
        let mut builder = builder();
        let start = builder.push();
        let accept = builder.push();
        builder.transition(start, range('a', 'z'), accept);
        builder.transition(start, only('e'), accept);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        assert_eq!(stepped(&nfa, &[start], 'e'), vec![accept, accept]);
    }

    #[test]
    fn a_state_keeps_its_transitions_in_the_sequence_in_which_they_arrived() {
        let mut builder = builder();
        let start = builder.push();
        let first = builder.push();
        let second = builder.push();
        builder.transition(start, only('b'), second);
        builder.transition(start, only('a'), first);
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
        let start = builder.push();
        let first = builder.push();
        let second = builder.push();
        builder.epsilon(start, second);
        builder.epsilon(start, first);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        assert_eq!(nfa.epsilons(start), &[second, first]);
        assert_eq!(nfa.epsilons(first), &[]);
    }

    #[test]
    fn only_a_state_that_the_builder_marked_accepts() {
        let mut builder = builder();
        let start = builder.push();
        let accept = builder.push();
        builder.accept(accept);
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
        let code = builder.push();
        let string = builder.push();
        let nfa = builder
            .build(&[code, string])
            .expect("the builder is below its capacity");

        assert_eq!(nfa.start_count(), 2);
        assert_eq!(nfa.start_states(), &[code, string]);
        assert_eq!(nfa.start_state(0), code);
        assert_eq!(nfa.start_state(1), string);
    }

    #[test]
    #[should_panic(expected = "start 2 is outside an automaton with 2 start states")]
    fn reading_a_start_outside_the_automaton_panics() {
        let mut builder = builder();
        let code = builder.push();
        let string = builder.push();
        let nfa = builder
            .build(&[code, string])
            .expect("the builder is below its capacity");

        nfa.start_state(2);
    }

    #[test]
    #[should_panic(expected = "state 9 is outside an arena of 2 states")]
    fn reading_the_transitions_of_a_state_outside_the_arena_panics() {
        let mut builder = builder();
        let start = builder.push();
        builder.push();
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        nfa.transitions(StateId::new(9));
    }

    #[test]
    #[should_panic(expected = "state 9 is outside an arena of 2 states")]
    fn reading_the_accept_of_a_state_outside_the_arena_panics() {
        let mut builder = builder();
        let start = builder.push();
        builder.push();
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        nfa.accepts(StateId::new(9));
    }
}
