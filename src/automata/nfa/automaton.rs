use super::id::{StartId, StateId};
use super::transition::Transition;
use crate::automata::arena::Arena;
use crate::automata::label::Label;

/// A nondeterministic finite automaton.
///
/// The automaton holds states, transitions with a label of type `L`, epsilon transitions, an
/// accept of type `A` for each state that accepts, and one or more start states.
///
/// The automaton does not know the alphabet, and it does not know the meaning of an accept. The
/// caller selects both. A lexer, for example, uses a byte range as the label and a token as the
/// accept.
///
/// Two accepts are equal only if the values are equal. Thus determinization and minimization can
/// divide the accept states correctly. They do not have to know the meaning of an accept.
///
/// To make an `Nfa`, use an [`NfaBuilder`](super::NfaBuilder).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nfa<L, A> {
    transitions: Arena<Transition<L>>,
    epsilons: Arena<StateId>,
    accepts: Vec<Option<A>>,
    starts: Vec<StateId>,
}

impl<L, A> Nfa<L, A> {
    /// Creates an `Nfa` from the transitions, the epsilon transitions, the accepts, and the start
    /// states.
    pub(super) fn new(
        transitions: Arena<Transition<L>>,
        epsilons: Arena<StateId>,
        accepts: Vec<Option<A>>,
        starts: Vec<StateId>,
    ) -> Self {
        Self {
            transitions,
            epsilons,
            accepts,
            starts,
        }
    }

    /// Returns the transitions that leave the state that `id` refers to.
    ///
    /// # Panics
    ///
    /// This function panics if `id` is not in the state arena.
    pub fn transitions(&self, id: StateId) -> &[Transition<L>] {
        self.transitions
            .get(id.index())
            .unwrap_or_else(|| self.outside(id))
    }

    /// Returns the states that the state that `id` refers to goes to without a symbol.
    ///
    /// # Panics
    ///
    /// This function panics if `id` is not in the state arena.
    pub fn epsilons(&self, id: StateId) -> &[StateId] {
        self.epsilons
            .get(id.index())
            .unwrap_or_else(|| self.outside(id))
    }

    /// Returns the accept of the state that `id` refers to, or `None` if the state does not
    /// accept.
    ///
    /// # Panics
    ///
    /// This function panics if `id` is not in the state arena.
    pub fn accept(&self, id: StateId) -> Option<&A> {
        self.accepts
            .get(id.index())
            .unwrap_or_else(|| self.outside(id))
            .as_ref()
    }

    /// Returns the number of the states in the state arena.
    pub fn state_count(&self) -> usize {
        self.accepts.len()
    }

    /// Returns the number of the start states of the automaton.
    pub fn start_count(&self) -> usize {
        self.starts.len()
    }

    /// Returns the state at which the automaton starts a scan under `start`.
    ///
    /// # Panics
    ///
    /// This function panics if `start` is not a start state of this automaton.
    pub fn start_state(&self, start: StartId) -> StateId {
        *self.starts.get(start.index()).unwrap_or_else(|| {
            panic!(
                "start {} is outside an automaton with {} start states",
                start.index(),
                self.starts.len()
            )
        })
    }

    /// Returns an iterator over the start identifiers and their states.
    pub fn starts(&self) -> impl Iterator<Item = (StartId, StateId)> + '_ {
        self.starts
            .iter()
            .enumerate()
            .map(|(index, &state)| (StartId::new(index), state))
    }

    fn outside(&self, id: StateId) -> ! {
        panic!(
            "state {} is outside an arena of {} states",
            id.index(),
            self.state_count()
        )
    }
}

impl<L: Label, A> Nfa<L, A> {
    /// Reads `symbol` from each state in `states`, then writes the new states into `out`.
    ///
    /// The function clears `out` first. It does not follow the epsilon transitions. To follow the
    /// epsilon transitions, use
    /// [`Simulator::epsilon_closure`](super::Simulator::epsilon_closure).
    ///
    /// The result holds one state for each transition that matches, thus it can hold a duplicate.
    ///
    /// # Panics
    ///
    /// This function panics if a state in `states` is not in the state arena.
    pub fn step(&self, states: &[StateId], symbol: L::Symbol, out: &mut Vec<StateId>) {
        out.clear();
        for &id in states {
            out.extend(
                self.transitions(id)
                    .iter()
                    .filter(|transition| transition.label.matches(symbol))
                    .map(|transition| transition.target),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::builder::NfaBuilder;
    use super::super::reference::{Symbols, only, range};
    use super::*;

    fn builder() -> NfaBuilder<Symbols, u32> {
        NfaBuilder::new()
    }

    fn stepped(nfa: &Nfa<Symbols, u32>, states: &[StateId], symbol: char) -> Vec<StateId> {
        let mut out = Vec::new();
        nfa.step(states, symbol, &mut out);
        out
    }

    #[test]
    fn a_step_follows_a_transition_that_matches_the_symbol() {
        let mut builder = builder();
        let start = builder.push();
        let accept = builder.push();
        builder.transition(start, range('a', 'z'), accept);
        builder.accept(accept, 0);
        let nfa = builder.build(&[start]);

        assert_eq!(stepped(&nfa, &[start], 'm'), vec![accept]);
    }

    #[test]
    fn a_step_ignores_a_transition_that_excludes_the_symbol() {
        let mut builder = builder();
        let start = builder.push();
        let accept = builder.push();
        builder.transition(start, range('a', 'z'), accept);
        let nfa = builder.build(&[start]);

        assert_eq!(stepped(&nfa, &[start], 'A'), Vec::new());
    }

    #[test]
    fn a_step_does_not_follow_an_epsilon_transition() {
        let mut builder = builder();
        let start = builder.push();
        let accept = builder.push();
        builder.epsilon(start, accept);
        let nfa = builder.build(&[start]);

        assert_eq!(stepped(&nfa, &[start], 'a'), Vec::new());
    }

    #[test]
    fn a_step_yields_a_shared_target_one_time_for_each_transition() {
        let mut builder = builder();
        let start = builder.push();
        let accept = builder.push();
        builder.transition(start, range('a', 'z'), accept);
        builder.transition(start, only('e'), accept);
        let nfa = builder.build(&[start]);

        assert_eq!(stepped(&nfa, &[start], 'e'), vec![accept, accept]);
    }

    #[test]
    fn stepping_into_a_buffer_replaces_what_it_held() {
        let mut builder = builder();
        let start = builder.push();
        let accept = builder.push();
        builder.transition(start, only('a'), accept);
        let nfa = builder.build(&[start]);

        let mut out = vec![start, start, start];
        nfa.step(&[start], 'a', &mut out);
        assert_eq!(out, vec![accept]);

        nfa.step(&[start], 'b', &mut out);
        assert_eq!(out, Vec::new());
    }

    #[test]
    fn a_state_keeps_its_transitions_in_the_sequence_in_which_they_arrived() {
        let mut builder = builder();
        let start = builder.push();
        let first = builder.push();
        let second = builder.push();
        builder.transition(start, only('b'), second);
        builder.transition(start, only('a'), first);
        let nfa = builder.build(&[start]);

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
        let nfa = builder.build(&[start]);

        assert_eq!(nfa.epsilons(start), &[second, first]);
        assert_eq!(nfa.epsilons(first), &[]);
    }

    #[test]
    fn only_a_state_that_accepts_has_an_accept() {
        let mut builder = builder();
        let start = builder.push();
        let accept = builder.push();
        builder.accept(accept, 7);
        let nfa = builder.build(&[start]);

        assert_eq!(nfa.accept(start), None);
        assert_eq!(nfa.accept(accept), Some(&7));
        assert_eq!(nfa.state_count(), 2);
    }

    #[test]
    fn each_start_names_its_own_state() {
        let mut builder = builder();
        let code = builder.push();
        let string = builder.push();
        let nfa = builder.build(&[code, string]);

        assert_eq!(nfa.start_count(), 2);
        assert_eq!(nfa.start_state(StartId::new(0)), code);
        assert_eq!(nfa.start_state(StartId::new(1)), string);
        assert_eq!(
            nfa.starts().collect::<Vec<_>>(),
            vec![(StartId::new(0), code), (StartId::new(1), string)]
        );
    }

    #[test]
    #[should_panic(expected = "start 2 is outside an automaton with 2 start states")]
    fn reading_a_start_outside_the_automaton_panics() {
        let mut builder = builder();
        let code = builder.push();
        let string = builder.push();
        let nfa = builder.build(&[code, string]);

        nfa.start_state(StartId::new(2));
    }

    #[test]
    #[should_panic(expected = "state 9 is outside an arena of 2 states")]
    fn reading_the_transitions_of_a_state_outside_the_arena_panics() {
        let mut builder = builder();
        let start = builder.push();
        builder.push();
        let nfa = builder.build(&[start]);

        nfa.transitions(StateId::new(9));
    }

    #[test]
    #[should_panic(expected = "state 9 is outside an arena of 2 states")]
    fn reading_the_accept_of_a_state_outside_the_arena_panics() {
        let mut builder = builder();
        let start = builder.push();
        builder.push();
        let nfa = builder.build(&[start]);

        nfa.accept(StateId::new(9));
    }
}
