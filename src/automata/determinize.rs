use std::collections::HashMap;

use super::arena::{Arena, ArenaBuilder};
use super::automaton::{Automaton, Transition};
use super::dfa::DeterministicFiniteAutomaton;
use super::execution::Execution;
use super::id::StateId;
use super::label::Label;
use super::nfa::{NondeterministicExecution, NondeterministicFiniteAutomaton};
use super::overflow::{Overflow, Part};

/// The automaton that determinization made, and the set behind each of its states.
///
/// One state of `dfa` is one set of the states of the nondeterministic automaton. `subsets` holds
/// that set, in one group for each state of `dfa`. The states of one group are in ascending
/// sequence, and the group holds no duplicate.
///
/// The automaton says which states accept. It does not say what an accept means. A caller that
/// holds a meaning for each state of the nondeterministic automaton reads `subsets` to get the
/// meaning of each state of `dfa`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Determinization<L> {
    /// The automaton that accepts the same input, and that reads each symbol one time.
    pub dfa: DeterministicFiniteAutomaton<L>,
    /// The states behind each state of [`dfa`](Self::dfa), one group for each state.
    pub subsets: Arena<StateId>,
}

impl<L: Label> NondeterministicFiniteAutomaton<L> {
    /// Determinizes the automaton, then returns the automaton that accepts the same input.
    ///
    /// One state of the result is one set of the states of this automaton. The scan of the result
    /// is in the state of the set that the scan of this automaton is in. Thus the two automata
    /// accept the same input, and the result reads each symbol one time.
    ///
    /// The result has one start state for each start state of this automaton, at the same
    /// start index. Two start states that hold the same set give one state, and
    /// the result keeps both start identifiers.
    ///
    /// A state of the result accepts if a state of its set accepts.
    ///
    /// The result holds no dead state. A set that reads a symbol and reaches no state gives no
    /// transition.
    ///
    /// # Errors
    ///
    /// This function returns an [`Overflow`] if the result needs more states than one automaton
    /// holds. One state of the result is one set of the states of this automaton, thus the number
    /// of the states can grow fast.
    pub fn determinize(&self) -> Result<Determinization<L>, Overflow> {
        subset_construction(self, StateId::CAPACITY)
    }
}

/// Determinizes `nfa` into an automaton of at most `capacity` states.
///
/// One state of the result is one set of the states of `nfa`. The function reads the transitions
/// of each set one time, and it stops because the number of the sets is finite.
///
/// [`determinize`](NondeterministicFiniteAutomaton::determinize) gives the capacity of one
/// automaton. A test gives a lower capacity to reach the [`Overflow`].
///
/// # Errors
///
/// This function returns an [`Overflow`] if the result needs more than `capacity` states.
fn subset_construction<L: Label>(
    nfa: &NondeterministicFiniteAutomaton<L>,
    capacity: usize,
) -> Result<Determinization<L>, Overflow> {
    let mut subsets = Subsets::new();
    let mut execution = NondeterministicExecution::new(nfa);
    let mut transitions = ArenaBuilder::new();

    let mut starts = Vec::with_capacity(nfa.start_count());
    for &state in nfa.start_states() {
        execution.seed(&[state]);
        starts.push(state_of(&mut subsets, capacity, execution.states())?);
    }

    while let Some((subset, state)) = subsets.next() {
        for (class, symbol) in L::divide(&labels(nfa, &subset)) {
            execution.seed(&subset);
            if !execution.step(symbol) {
                continue;
            }
            let target = state_of(&mut subsets, capacity, execution.states())?;
            transitions.push(
                state.index(),
                Transition {
                    label: class,
                    target,
                },
            );
        }
    }

    let sets = subsets.into_sets();
    let count = sets.len();
    let accepts = sets
        .iter()
        .map(|subset| subset.iter().any(|&id| nfa.accepts(id)))
        .collect();

    let mut groups = ArenaBuilder::new();
    for (index, subset) in sets.iter().enumerate() {
        for &id in subset {
            groups.push(index, id);
        }
    }

    Ok(Determinization {
        dfa: DeterministicFiniteAutomaton::new(transitions.build(count)?, accepts, starts),
        subsets: groups.build(count)?,
    })
}

/// Returns the state of `subset`, and adds the set to `subsets` if the table does not hold it.
///
/// The function numbers each new state from 0. Thus the states of the table are the states of the
/// deterministic automaton, and [`Subsets::into_sets`] gives one set for each of them.
///
/// # Errors
///
/// This function returns an [`Overflow`] if a new set needs a state at or above `capacity`.
fn state_of(
    subsets: &mut Subsets,
    capacity: usize,
    subset: &[StateId],
) -> Result<StateId, Overflow> {
    if let Some(state) = subsets.get(subset) {
        return Ok(state);
    }
    if subsets.count() >= capacity {
        return Err(Overflow::new(Part::States, capacity));
    }
    let state = StateId::new(subsets.count());
    subsets.add(subset, state);
    Ok(state)
}

/// The sets of the states of the NFA that determinization reached.
///
/// One set is one state of the DFA. The table gives the state of a set that it already holds. Thus
/// determinization stops, because the number of the sets is finite.
///
/// The table also queues each new set. Determinization reads the transitions of a set one time,
/// and it reads them after the set has its state.
#[derive(Debug)]
struct Subsets {
    states: HashMap<Vec<StateId>, StateId>,
    pending: Vec<(Vec<StateId>, StateId)>,
}

impl Subsets {
    /// Creates a table that holds no set.
    fn new() -> Self {
        Self {
            states: HashMap::new(),
            pending: Vec::new(),
        }
    }

    /// Returns the state of `subset`, or `None` if the table does not hold that set.
    ///
    /// The states of `subset` are in ascending sequence, and the set holds no duplicate. An
    /// [`NfaExecution`](super::NondeterministicExecution) gives its states in that form.
    fn get(&self, subset: &[StateId]) -> Option<StateId> {
        self.states.get(subset).copied()
    }

    /// Returns the number of the sets in the table.
    fn count(&self) -> usize {
        self.states.len()
    }

    /// Adds `subset` as the set of `state`, then queues it.
    ///
    /// # Panics
    ///
    /// This function panics if the table already holds `subset`.
    fn add(&mut self, subset: &[StateId], state: StateId) {
        let subset = subset.to_vec();
        let held = self.states.insert(subset.clone(), state);
        assert!(held.is_none(), "the table already holds the set");
        self.pending.push((subset, state));
    }

    /// Returns a set whose transitions determinization did not read, with its state.
    ///
    /// The result is `None` if determinization read the transitions of each set.
    fn next(&mut self) -> Option<(Vec<StateId>, StateId)> {
        self.pending.pop()
    }

    /// Returns the set of each state, the set of the first state first.
    ///
    /// [`state_of`] numbers the states from 0, one for each set. Thus each state of the table is
    /// below the number of the sets.
    fn into_sets(self) -> Vec<Vec<StateId>> {
        let count = self.states.len();
        let mut sets = vec![Vec::new(); count];
        for (subset, state) in self.states {
            let slot = sets
                .get_mut(state.index())
                .expect("the table numbers its states from 0, one for each set");
            *slot = subset;
        }
        sets
    }
}

/// Returns the label of each transition that leaves a state of `subset`.
///
/// The result holds a duplicate if two states carry the same label. [`Label::divide`] removes
/// the duplicate.
fn labels<L: Label>(nfa: &NondeterministicFiniteAutomaton<L>, subset: &[StateId]) -> Vec<L> {
    subset
        .iter()
        .flat_map(|&id| nfa.transitions(id))
        .map(|transition| transition.label.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::automaton::Automaton;

    use crate::automata::nfa::NfaBuilder;
    use crate::automata::overflow::Part;
    use crate::automata::testing::{Symbols, literal, only, range, scan, state};

    /// Builds an NFA. `build` adds the states, then it returns the start states.
    fn nfa(
        build: impl FnOnce(&mut NfaBuilder<Symbols>) -> Vec<StateId>,
    ) -> NondeterministicFiniteAutomaton<Symbols> {
        let mut builder = NfaBuilder::new();
        let starts = build(&mut builder);
        builder
            .build(&starts)
            .expect("a test stays below the capacity")
    }

    fn determinized(
        nfa: &NondeterministicFiniteAutomaton<Symbols>,
    ) -> DeterministicFiniteAutomaton<Symbols> {
        nfa.determinize()
            .expect("a test stays below the capacity")
            .dfa
    }

    fn first() -> usize {
        0
    }

    /// Builds an NFA of two rules that share the first symbol.
    fn branches() -> NondeterministicFiniteAutomaton<Symbols> {
        nfa(|builder| {
            let start = builder.push();
            let left = literal(builder, "ab");
            let right = literal(builder, "ac");
            builder.epsilon(start, left.entry);
            builder.epsilon(start, right.entry);
            vec![start]
        })
    }

    /// Builds an NFA whose start reads one symbol into two states that accept.
    fn two_accepts() -> NondeterministicFiniteAutomaton<Symbols> {
        nfa(|builder| {
            let start = builder.push();
            let high = builder.push();
            let low = builder.push();
            builder.transition(start, only('a'), high);
            builder.transition(start, only('a'), low);
            builder.accept(high);
            builder.accept(low);
            vec![start]
        })
    }

    /// Builds an NFA whose start carries two labels that share the symbols `d` to `f`.
    fn overlapping() -> NondeterministicFiniteAutomaton<Symbols> {
        nfa(|builder| {
            let start = builder.push();
            let left = builder.push();
            let right = builder.push();
            builder.transition(start, range('a', 'f'), left);
            builder.transition(start, range('d', 'z'), right);
            builder.accept(left);
            builder.accept(right);
            vec![start]
        })
    }

    /// Builds an NFA in the manner of a lexer: a keyword, an identifier, and a space.
    fn lexer() -> NondeterministicFiniteAutomaton<Symbols> {
        nfa(|builder| {
            let start = builder.push();
            let keyword = literal(builder, "if");
            let entry = builder.push();
            let rest = builder.push();
            builder.transition(entry, range('a', 'z'), rest);
            builder.transition(rest, range('a', 'z'), rest);
            builder.transition(rest, range('0', '9'), rest);
            builder.accept(rest);
            let space = literal(builder, " ");
            builder.epsilon(start, keyword.entry);
            builder.epsilon(start, entry);
            builder.epsilon(start, space.entry);
            vec![start]
        })
    }

    #[test]
    fn a_chain_gives_one_state_for_each_prefix() {
        let nfa = nfa(|builder| vec![literal(builder, "ab").entry]);
        let dfa = determinized(&nfa);

        assert_eq!(dfa.state_count(), 3);
        assert_eq!(dfa.start_count(), 1);
        assert_eq!(scan(&dfa, first(), "ab"), Some(2));
        assert_eq!(scan(&dfa, first(), "a"), None);
    }

    #[test]
    fn an_alternation_joins_the_states_of_its_common_prefix() {
        let nfa = branches();
        let dfa = determinized(&nfa);

        assert_eq!(nfa.state_count(), 7);
        assert_eq!(dfa.state_count(), 4);
        assert_eq!(scan(&dfa, first(), "ab"), Some(2));
        assert_eq!(scan(&dfa, first(), "ac"), Some(2));
        assert_eq!(scan(&dfa, first(), "ad"), None);
    }

    #[test]
    fn two_symbols_that_reach_the_same_states_give_one_state() {
        let nfa = nfa(|builder| {
            let start = builder.push();
            let middle = builder.push();
            let end = builder.push();
            builder.transition(start, only('a'), middle);
            builder.transition(start, only('b'), middle);
            builder.transition(middle, only('c'), end);
            builder.accept(end);
            vec![start]
        });
        let dfa = determinized(&nfa);

        assert_eq!(dfa.state_count(), 3);
        assert_eq!(dfa.transitions(dfa.start_state(first())).len(), 2);
        assert_eq!(scan(&dfa, first(), "ac"), Some(2));
        assert_eq!(scan(&dfa, first(), "bc"), Some(2));
    }

    #[test]
    fn two_equal_labels_give_one_transition() {
        let dfa = determinized(&two_accepts());

        assert_eq!(dfa.transitions(dfa.start_state(first())).len(), 1);
        assert_eq!(dfa.state_count(), 2);
    }

    #[test]
    fn a_state_accepts_if_a_state_of_its_set_accepts() {
        let nfa = two_accepts();
        let dfa = determinized(&nfa);
        let start = dfa.start_state(first());
        let target = dfa.step(start, 'a').expect("the symbol a reaches a state");

        assert!(!dfa.accepts(start));
        assert!(dfa.accepts(target));
        assert_eq!(scan(&dfa, first(), "a"), Some(1));
    }

    #[test]
    fn the_subset_of_a_state_holds_the_states_that_it_stands_for() {
        let nfa = two_accepts();
        let determinization = nfa.determinize().expect("a test stays below the capacity");
        let start = determinization.dfa.start_state(first());
        let target = determinization
            .dfa
            .step(start, 'a')
            .expect("the symbol a reaches a state");

        assert_eq!(
            determinization.subsets.get(start.index()),
            Some(&[state(0)][..])
        );
        assert_eq!(
            determinization.subsets.get(target.index()),
            Some(&[state(1), state(2)][..])
        );
        assert_eq!(
            determinization.subsets.group_count(),
            determinization.dfa.state_count()
        );
    }

    #[test]
    fn two_labels_that_share_symbols_divide_into_three_transitions() {
        let dfa = determinized(&overlapping());

        assert_eq!(dfa.transitions(dfa.start_state(first())).len(), 3);
        assert_eq!(dfa.state_count(), 4);
        assert_eq!(scan(&dfa, first(), "a"), Some(1));
        assert_eq!(scan(&dfa, first(), "d"), Some(1));
        assert_eq!(scan(&dfa, first(), "g"), Some(1));
        assert_eq!(scan(&dfa, first(), "A"), None);
    }

    #[test]
    fn a_start_that_accepts_gives_a_match_of_no_length() {
        let nfa = nfa(|builder| {
            let start = builder.push();
            builder.accept(start);
            vec![start]
        });
        let dfa = determinized(&nfa);

        assert_eq!(dfa.state_count(), 1);
        assert!(dfa.accepts(dfa.start_state(first())));
        assert_eq!(scan(&dfa, first(), ""), Some(0));
        assert_eq!(scan(&dfa, first(), "zz"), Some(0));
    }

    #[test]
    fn each_start_of_the_nfa_gives_a_start_of_the_dfa() {
        let nfa = nfa(|builder| {
            let code = literal(builder, "a");
            let string = literal(builder, "b");
            vec![code.entry, string.entry]
        });
        let dfa = determinized(&nfa);

        assert_eq!(dfa.start_count(), 2);
        assert_eq!(dfa.state_count(), 4);
        assert_eq!(scan(&dfa, 0, "a"), Some(1));
        assert_eq!(scan(&dfa, 0, "b"), None);
        assert_eq!(scan(&dfa, 1, "b"), Some(1));
        assert_eq!(scan(&dfa, 1, "a"), None);
    }

    #[test]
    fn two_starts_of_the_same_states_give_one_state() {
        let nfa = nfa(|builder| {
            let start = literal(builder, "a").entry;
            vec![start, start]
        });
        let dfa = determinized(&nfa);

        assert_eq!(dfa.start_count(), 2);
        assert_eq!(dfa.state_count(), 2);
        assert_eq!(dfa.start_state(0), dfa.start_state(1));
    }

    #[test]
    fn a_state_that_reaches_nothing_gives_no_transition() {
        let nfa = nfa(|builder| {
            let start = builder.push();
            let end = builder.push();
            builder.transition(start, only('a'), end);
            vec![start]
        });
        let dfa = determinized(&nfa);
        let start = dfa.start_state(first());
        let target = dfa.step(start, 'a').expect("the symbol a reaches a state");

        assert_eq!(dfa.state_count(), 2);
        assert_eq!(dfa.transitions(target), &[]);
        assert_eq!(dfa.step(start, 'z'), None);
        assert_eq!(scan(&dfa, first(), "a"), None);
    }

    #[test]
    fn an_epsilon_cycle_stops() {
        let nfa = nfa(|builder| {
            let start = builder.push();
            let other = builder.push();
            let end = builder.push();
            builder.epsilon(start, other);
            builder.epsilon(other, start);
            builder.transition(other, only('a'), end);
            builder.accept(end);
            vec![start]
        });
        let dfa = determinized(&nfa);

        assert_eq!(dfa.state_count(), 2);
        assert_eq!(scan(&dfa, first(), "a"), Some(1));
    }

    #[test]
    fn a_state_has_a_maximum_of_one_transition_for_one_symbol() {
        for nfa in [branches(), overlapping(), lexer()] {
            let dfa = determinized(&nfa);
            for index in 0..dfa.state_count() {
                for symbol in "abcdefgijkxyz019 !".chars() {
                    let count = dfa
                        .transitions(state(index))
                        .iter()
                        .filter(|transition| transition.label.matches(symbol))
                        .count();
                    assert!(
                        count <= 1,
                        "state {index} has {count} transitions for {symbol:?}"
                    );
                }
            }
        }
    }

    /// Asserts that the DFA of `nfa` gives the same match as `nfa`, for each input and each start.
    fn same_matches(nfa: &NondeterministicFiniteAutomaton<Symbols>, inputs: &[&str]) {
        let dfa = determinized(nfa);
        for index in 0..nfa.start_count() {
            let start = index;
            for input in inputs {
                assert_eq!(
                    scan(&dfa, start, input),
                    scan(nfa, start, input),
                    "input {input:?} under start {index}"
                );
            }
        }
    }

    #[test]
    fn the_dfa_gives_the_same_match_as_the_nfa() {
        let inputs = [
            "", "a", "b", "c", "d", "g", "i", "z", "0", "9", " ", "!", "ab", "ac", "ad", "if",
            "iff", "ifx", "if9", "i9", "z0z", "if if", "  ", "abc", "9if",
        ];

        same_matches(&branches(), &inputs);
        same_matches(&overlapping(), &inputs);
        same_matches(&two_accepts(), &inputs);
        same_matches(&lexer(), &inputs);
    }

    #[test]
    fn a_dfa_above_the_capacity_reports_an_overflow() {
        let nfa = nfa(|builder| vec![literal(builder, "ab").entry]);

        assert_eq!(
            subset_construction(&nfa, 2),
            Err(Overflow::new(Part::States, 2))
        );
        assert!(subset_construction(&nfa, 3).is_ok());
    }

    #[test]
    fn a_new_table_holds_no_set() {
        assert_eq!(Subsets::new().get(&[state(0)]), None);
    }

    #[test]
    fn the_table_gives_the_state_of_a_set_that_it_holds() {
        let mut subsets = Subsets::new();
        subsets.add(&[state(0), state(2)], state(9));

        assert_eq!(subsets.get(&[state(0), state(2)]), Some(state(9)));
        assert_eq!(subsets.get(&[state(0)]), None);
        assert_eq!(subsets.get(&[state(0), state(1), state(2)]), None);
    }

    #[test]
    fn the_table_gives_each_set_that_it_queued_one_time() {
        let mut subsets = Subsets::new();
        subsets.add(&[state(0)], state(0));
        subsets.add(&[state(1)], state(1));

        let mut queued = vec![
            subsets.next().expect("the table queued two sets"),
            subsets.next().expect("the table queued two sets"),
        ];
        queued.sort();

        assert_eq!(
            queued,
            vec![(vec![state(0)], state(0)), (vec![state(1)], state(1))]
        );
        assert_eq!(subsets.next(), None);
    }

    #[test]
    fn the_table_gives_the_set_of_each_state_in_the_sequence_of_the_states() {
        let mut subsets = Subsets::new();
        subsets.add(&[state(1), state(2)], state(1));
        subsets.add(&[state(0)], state(0));

        assert_eq!(
            subsets.into_sets(),
            vec![vec![state(0)], vec![state(1), state(2)]]
        );
    }

    #[test]
    fn a_new_table_gives_no_set() {
        assert_eq!(Subsets::new().into_sets(), Vec::<Vec<StateId>>::new());
    }

    #[test]
    #[should_panic(expected = "the table already holds the set")]
    fn adding_a_set_that_the_table_holds_panics() {
        let mut subsets = Subsets::new();
        subsets.add(&[state(0)], state(0));
        subsets.add(&[state(0)], state(1));
    }

    #[test]
    fn the_labels_of_a_set_hold_the_label_of_each_transition() {
        let nfa = overlapping();

        assert_eq!(
            labels(&nfa, &[state(0)]),
            vec![range('a', 'f'), range('d', 'z')]
        );
        assert_eq!(labels(&nfa, &[state(1), state(2)]), Vec::new());
        assert_eq!(labels(&nfa, &[]), Vec::new());
    }
}
