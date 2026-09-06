//! Computes NFA epsilon closures.

use super::Nfa;
use crate::automata::StateId;
use crate::automata::state_set::StateSet;

/// Replaces `states` with the epsilon closure of `seeds`.
///
/// The result is sorted and contains no duplicates.
///
/// # Panics
///
/// This function panics if a seed is outside the NFA.
pub(crate) fn epsilon_closure<L, A>(nfa: &Nfa<L, A>, seeds: &[StateId], states: &mut StateSet) {
    states.clear();
    for &seed in seeds {
        states.insert(seed);
    }

    let mut pending = 0;
    while let Some(state) = states.get(pending) {
        pending += 1;
        for &target in nfa.epsilon_targets(state) {
            states.insert(target);
        }
    }
    states.sort();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::testing::{Symbols, builder, only};

    fn closure(nfa: &Nfa<Symbols>, seeds: &[StateId]) -> Vec<usize> {
        let mut states = StateSet::new(nfa.state_count());
        epsilon_closure(nfa, seeds, &mut states);
        states.members().iter().map(|id| id.index()).collect()
    }

    #[test]
    fn the_closure_of_a_state_without_an_epsilon_transition_is_just_itself() {
        let mut builder = builder();
        let accept = builder.add_state();
        let nfa = builder
            .build(&[accept])
            .expect("the builder is below its capacity");

        assert_eq!(closure(&nfa, &[accept]), vec![0]);
    }

    #[test]
    fn the_closure_follows_each_epsilon_transition_of_a_state() {
        let mut builder = builder();
        let start = builder.add_state();
        let left = builder.add_state();
        let middle = builder.add_state();
        let right = builder.add_state();
        builder.add_epsilon_transition(start, left);
        builder.add_epsilon_transition(start, middle);
        builder.add_epsilon_transition(start, right);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        assert_eq!(closure(&nfa, &[start]), vec![0, 1, 2, 3]);
    }

    #[test]
    fn the_closure_stops_at_a_transition_with_a_label() {
        let mut builder = builder();
        let start = builder.add_state();
        let accept = builder.add_state();
        builder.add_transition(start, only('a'), accept);
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        assert_eq!(closure(&nfa, &[start]), vec![0]);
    }

    #[test]
    fn the_closure_terminates_on_an_epsilon_cycle() {
        let mut builder = builder();
        let left = builder.add_state();
        let right = builder.add_state();
        let accept = builder.add_state();
        builder.add_epsilon_transition(left, right);
        builder.add_epsilon_transition(left, accept);
        builder.add_epsilon_transition(right, left);
        builder.add_epsilon_transition(right, accept);
        let nfa = builder
            .build(&[left])
            .expect("the builder is below its capacity");

        assert_eq!(closure(&nfa, &[left]), vec![0, 1, 2]);
    }

    #[test]
    fn the_closure_is_sorted_and_deduplicated() {
        let mut builder = builder();
        let first = builder.add_state();
        let second = builder.add_state();
        let left = builder.add_state();
        let right = builder.add_state();
        builder.add_epsilon_transition(left, first);
        builder.add_epsilon_transition(left, second);
        builder.add_epsilon_transition(right, second);
        builder.add_epsilon_transition(right, first);
        let nfa = builder
            .build(&[left])
            .expect("the builder is below its capacity");

        assert_eq!(closure(&nfa, &[right, left, right]), vec![0, 1, 2, 3]);
    }

    #[test]
    #[should_panic(expected = "state 9 is outside an automaton of 2 states")]
    fn a_closure_over_a_seed_outside_the_arena_panics() {
        let mut builder = builder();
        let start = builder.add_state();
        builder.add_state();
        let nfa = builder
            .build(&[start])
            .expect("the builder is below its capacity");

        closure(&nfa, &[StateId::new(9)]);
    }
}
