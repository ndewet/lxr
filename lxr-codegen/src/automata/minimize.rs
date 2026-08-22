use std::collections::HashMap;
use std::hash::Hash;

use super::arena::ArenaBuilder;
use super::automaton::{Automaton, Transition};
use super::dfa::DeterministicFiniteAutomaton;
use super::id::StateId;
use super::label::Label;

/// The state that stands for a target that no transition holds.
///
/// A state of the automaton reads a symbol that no label of that state matches, and it goes to no
/// state. The signature of a state holds this value at that symbol, thus a state that stops and a
/// state that goes on are two states.
const DEAD: usize = usize::MAX;

/// The automaton that minimization made, and the state of each state before it.
///
/// One state of `dfa` stands for one group of the states of the automaton before. `states` holds
/// the state of `dfa` that each of those states belongs to, at the index of that state.
///
/// The automaton says which states accept. It does not say what an accept means. A caller that
/// holds a meaning for each state of the automaton before reads `states` to move each meaning to
/// the state that carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Minimization<L> {
    /// The automaton of the fewest states that accepts the same input.
    pub dfa: DeterministicFiniteAutomaton<L>,
    /// The state of `dfa` that each state of the automaton before belongs to.
    pub states: Vec<StateId>,
}

impl<L: Label> DeterministicFiniteAutomaton<L> {
    /// Minimizes the automaton, then returns the automaton that accepts the same input.
    ///
    /// Two states join if they accept the same, and if each symbol takes them to two states that
    /// join. The result holds one state for each group of such states. Thus the result reads the
    /// same input as this automaton, and it holds fewer states or the same number of states.
    ///
    /// `accept` gives the meaning of the accept of a state. Two states of a different meaning
    /// stay apart, thus a lexer that accepts one rule at a state keeps that rule. Give the same
    /// meaning for each state that does not accept, for example `None`.
    ///
    /// The result has one start state for each start state of this automaton, at the same start
    /// index. Two start states that join give one state, and the result keeps both start
    /// identifiers.
    ///
    /// The transitions of one state of the result are in ascending sequence, because the result
    /// takes the transitions of one state of the group.
    ///
    /// # Panics
    ///
    /// This function panics if the result needs more items than an arena holds. The result holds
    /// no more transitions than this automaton, thus an automaton that exists gives no panic.
    pub fn minimize<K, F>(&self, accept: F) -> Minimization<L>
    where
        K: Eq + Hash,
        F: Fn(StateId) -> K,
    {
        let symbols = class_symbols(self);
        let (mut blocks, mut count) = initial_blocks(self, accept);

        loop {
            let (refined, refined_count) = refine(self, &blocks, &symbols);
            blocks = refined;
            if refined_count == count {
                break;
            }
            count = refined_count;
        }

        build(self, &blocks, count)
    }
}

/// Returns one symbol of each class of the symbols that the labels of `dfa` match.
///
/// Two states answer the same for each symbol of one class, because [`Label::divide`] gives a
/// class that each label matches whole or not at all. Thus the signature of a state holds one
/// answer for each class, and not one answer for each symbol of the alphabet.
///
/// A symbol that no label matches takes each state to no state, thus it separates no pair of the
/// states and the classes leave it out.
fn class_symbols<L: Label>(dfa: &DeterministicFiniteAutomaton<L>) -> Vec<L::Symbol> {
    let mut labels = Vec::new();
    for index in 0..dfa.state_count() {
        labels.extend(
            dfa.transitions(StateId::new(index))
                .iter()
                .map(|transition| transition.label.clone()),
        );
    }

    L::divide(&labels)
        .into_iter()
        .map(|(_, symbol)| symbol)
        .collect()
}

/// Returns the first group of each state, and the number of the groups.
///
/// Two states are in one group if they accept the same, and if `accept` gives the same meaning for
/// both. [`refine`] divides a group that the symbols separate.
///
/// The function numbers the groups from 0, in the sequence of the states. Thus the group of the
/// first state is 0.
fn initial_blocks<L, K, F>(dfa: &DeterministicFiniteAutomaton<L>, accept: F) -> (Vec<usize>, usize)
where
    K: Eq + Hash,
    F: Fn(StateId) -> K,
{
    let mut numbers = HashMap::new();
    let mut blocks = Vec::with_capacity(dfa.state_count());

    for index in 0..dfa.state_count() {
        let id = StateId::new(index);
        let next = numbers.len();
        blocks.push(*numbers.entry((dfa.accepts(id), accept(id))).or_insert(next));
    }

    let count = numbers.len();
    (blocks, count)
}

/// Returns the groups that the symbols make from `blocks`, and the number of the groups.
///
/// The signature of a state is its group, and the group of the state that each class takes it to.
/// Two states of one signature stay in one group, and each other pair divides. The pass stops when
/// a round makes the same number of groups as the round before it.
///
/// The function numbers the groups from 0, in the sequence of the states.
fn refine<L: Label>(
    dfa: &DeterministicFiniteAutomaton<L>,
    blocks: &[usize],
    symbols: &[L::Symbol],
) -> (Vec<usize>, usize) {
    let mut numbers: HashMap<Vec<usize>, usize> = HashMap::new();
    let mut refined = Vec::with_capacity(blocks.len());
    let mut signature = Vec::with_capacity(symbols.len() + 1);

    for (index, &block) in blocks.iter().enumerate() {
        signature.clear();
        signature.push(block);
        signature.extend(symbols.iter().map(|&symbol| {
            dfa.step(StateId::new(index), symbol)
                .map_or(DEAD, |target| blocks[target.index()])
        }));

        let next = numbers.len();
        refined.push(*numbers.entry(signature.clone()).or_insert(next));
    }

    let count = numbers.len();
    (refined, count)
}

/// Builds the automaton of `count` groups, in which `blocks` gives the group of each state.
///
/// One state of the result is one group. The result takes the transitions and the accept of the
/// first state of the group, and it points each transition at the group of its target. Each state
/// of one group answers the same for each symbol, thus the result reads the same input.
///
/// # Panics
///
/// This function panics if a group holds no state, or if the transitions need more items than an
/// arena holds.
fn build<L: Label>(
    dfa: &DeterministicFiniteAutomaton<L>,
    blocks: &[usize],
    count: usize,
) -> Minimization<L> {
    let mut representatives: Vec<Option<usize>> = vec![None; count];
    for (index, &block) in blocks.iter().enumerate() {
        let slot = representatives
            .get_mut(block)
            .expect("a group is below the number of the groups");
        slot.get_or_insert(index);
    }

    let mut transitions = ArenaBuilder::new();
    let mut accepts = Vec::with_capacity(count);
    for (block, representative) in representatives.iter().enumerate() {
        let id = StateId::new(representative.expect("a group holds at least one state"));
        for transition in dfa.transitions(id) {
            transitions.push(
                block,
                Transition {
                    label: transition.label.clone(),
                    target: StateId::new(blocks[transition.target.index()]),
                },
            );
        }
        accepts.push(dfa.accepts(id));
    }

    let starts = dfa
        .start_states()
        .iter()
        .map(|start| StateId::new(blocks[start.index()]))
        .collect();
    let transitions = transitions
        .build(count)
        .expect("the result holds no more transitions than the automaton that it reads");

    Minimization {
        dfa: DeterministicFiniteAutomaton::new(transitions, accepts, starts),
        states: blocks.iter().map(|&block| StateId::new(block)).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::testing::{Symbols, dfa, only, range, scan, state};

    /// Minimizes `automaton`, and gives each state that accepts the same meaning.
    fn minimized(
        automaton: &DeterministicFiniteAutomaton<Symbols>,
    ) -> DeterministicFiniteAutomaton<Symbols> {
        automaton.minimize(|_| ()).dfa
    }

    /// Builds an automaton that reads `"ab"` and `"cb"` through two chains that do not join.
    ///
    /// The two chains hold the same states, thus minimization joins them.
    fn twins() -> DeterministicFiniteAutomaton<Symbols> {
        dfa(
            &[
                &[(only('a'), 1), (only('c'), 3)],
                &[(only('b'), 2)],
                &[],
                &[(only('b'), 4)],
                &[],
            ],
            &[false, false, true, false, true],
            &[0],
        )
    }

    /// Builds an automaton of two states that accept, and that read the same input after them.
    fn two_accepts() -> DeterministicFiniteAutomaton<Symbols> {
        dfa(
            &[&[(only('a'), 1), (only('b'), 2)], &[], &[]],
            &[false, true, true],
            &[0],
        )
    }

    /// Asserts that `automaton` and its minimization give the same match for each input.
    fn same_matches(automaton: &DeterministicFiniteAutomaton<Symbols>, inputs: &[&str]) {
        let minimal = minimized(automaton);
        for index in 0..automaton.start_count() {
            for input in inputs {
                assert_eq!(
                    scan(&minimal, index, input),
                    scan(automaton, index, input),
                    "input {input:?} under start {index}"
                );
            }
        }
    }

    #[test]
    fn two_states_that_read_the_same_input_give_one_state() {
        let minimal = minimized(&twins());

        assert_eq!(minimal.state_count(), 3);
        assert_eq!(scan(&minimal, 0, "ab"), Some(2));
        assert_eq!(scan(&minimal, 0, "cb"), Some(2));
        assert_eq!(scan(&minimal, 0, "a"), None);
    }

    #[test]
    fn a_minimal_automaton_keeps_each_state() {
        let automaton = dfa(&[&[(only('a'), 1)], &[]], &[false, true], &[0]);
        let minimal = minimized(&automaton);

        assert_eq!(minimal.state_count(), 2);
        assert_eq!(minimal, automaton);
    }

    #[test]
    fn a_state_holds_the_group_of_each_state_of_the_automaton_before() {
        let minimization = twins().minimize(|_| ());

        assert_eq!(
            minimization.states,
            vec![state(0), state(1), state(2), state(1), state(2)]
        );
    }

    #[test]
    fn two_states_of_a_different_accept_stay_apart() {
        let automaton = two_accepts();
        let minimal = automaton.minimize(|id| id.index()).dfa;

        assert_eq!(minimized(&automaton).state_count(), 2);
        assert_eq!(minimal.state_count(), 3);
        assert_eq!(scan(&minimal, 0, "a"), Some(1));
        assert_eq!(scan(&minimal, 0, "b"), Some(1));
    }

    #[test]
    fn a_state_that_accepts_stays_apart_from_a_state_that_does_not() {
        let automaton = dfa(
            &[&[(only('a'), 1), (only('b'), 2)], &[], &[]],
            &[false, true, false],
            &[0],
        );
        let minimal = minimized(&automaton);

        assert_eq!(minimal.state_count(), 3);
        assert_eq!(scan(&minimal, 0, "a"), Some(1));
        assert_eq!(scan(&minimal, 0, "b"), None);
    }

    #[test]
    fn a_state_that_stops_stays_apart_from_a_state_that_reads_on() {
        let automaton = dfa(
            &[&[(only('a'), 1), (only('b'), 2)], &[(only('a'), 1)], &[]],
            &[false, false, false],
            &[0],
        );
        let minimal = minimized(&automaton);

        assert_eq!(minimal.state_count(), 3);
    }

    #[test]
    fn a_loop_joins_the_states_that_read_it() {
        let automaton = dfa(
            &[
                &[(range('a', 'z'), 1)],
                &[(range('a', 'z'), 2)],
                &[(range('a', 'z'), 2)],
            ],
            &[false, true, true],
            &[0],
        );
        let minimal = minimized(&automaton);

        assert_eq!(minimal.state_count(), 2);
        assert_eq!(scan(&minimal, 0, "abcde"), Some(5));
    }

    #[test]
    fn each_start_of_the_automaton_gives_a_start_of_the_result() {
        let automaton = dfa(
            &[&[(only('a'), 2)], &[(only('b'), 3)], &[], &[]],
            &[false, false, true, true],
            &[0, 1],
        );
        let minimal = minimized(&automaton);

        assert_eq!(minimal.start_count(), 2);
        assert_eq!(minimal.state_count(), 3);
        assert_eq!(scan(&minimal, 0, "a"), Some(1));
        assert_eq!(scan(&minimal, 0, "b"), None);
        assert_eq!(scan(&minimal, 1, "b"), Some(1));
    }

    #[test]
    fn two_starts_that_read_the_same_input_give_one_state() {
        let automaton = dfa(
            &[&[(only('a'), 2)], &[(only('a'), 3)], &[], &[]],
            &[false, false, true, true],
            &[0, 1],
        );
        let minimal = minimized(&automaton);

        assert_eq!(minimal.start_count(), 2);
        assert_eq!(minimal.state_count(), 2);
        assert_eq!(minimal.start_state(0), minimal.start_state(1));
        assert_eq!(scan(&minimal, 1, "a"), Some(1));
    }

    #[test]
    fn the_minimal_automaton_gives_the_same_match_as_the_automaton() {
        let inputs = [
            "", "a", "b", "c", "d", "z", "ab", "cb", "ac", "abc", "cbb", "zzz",
        ];

        same_matches(&twins(), &inputs);
        same_matches(&two_accepts(), &inputs);
        same_matches(
            &dfa(
                &[
                    &[(range('a', 'z'), 1)],
                    &[(range('a', 'z'), 1)],
                    &[(only('a'), 0)],
                ],
                &[false, true, true],
                &[0, 2],
            ),
            &inputs,
        );
    }
}
