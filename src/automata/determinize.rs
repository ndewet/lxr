use std::collections::HashMap;

use super::dfa::Dfa;
#[expect(
    unused_imports,
    reason = "step 6 of the plan builds the automaton with it"
)]
use super::dfa::DfaBuilder;
use super::id::StateId;
use super::label::Label;
use super::nfa::Nfa;
use super::overflow::Overflow;

impl<L, A> Nfa<L, A>
where
    L: Label + Clone,
    A: Ord + Clone,
{
    /// Determinizes the automaton, then returns the automaton that accepts the same input.
    ///
    /// One state of the result is one set of the states of this automaton. The scan of the result
    /// is in the state of the set that the scan of this automaton is in. Thus the two automata
    /// accept the same input, and the result reads each symbol one time.
    ///
    /// The result has one start state for each start state of this automaton, at the same
    /// [`StartId`](super::StartId). Two start states that hold the same set give one state, and
    /// the result keeps both start identifiers.
    ///
    /// The accept of a state is the lowest accept of its set, because
    /// [`longest_match`](super::longest_match) selects the lowest accept. Thus the two automata
    /// select the same rule.
    ///
    /// The result holds no dead state. A set that reads a symbol and reaches no state gives no
    /// transition.
    ///
    /// # Errors
    ///
    /// This function returns an [`Overflow`] if the result needs more states than one automaton
    /// holds. One state of the result is one set of the states of this automaton, thus the number
    /// of the states can grow fast.
    pub fn determinize(&self) -> Result<Dfa<L, A>, Overflow> {
        within(self, StateId::CAPACITY)
    }
}

/// Determinizes `nfa` into an automaton of at most `capacity` states.
///
/// # Errors
///
/// This function returns an [`Overflow`] if the result needs more than `capacity` states.
#[expect(clippy::todo, unused_variables, reason = "step 6 of the plan")]
fn within<L, A>(nfa: &Nfa<L, A>, capacity: usize) -> Result<Dfa<L, A>, Overflow>
where
    L: Label + Clone,
    A: Ord + Clone,
{
    todo!()
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
    #[expect(clippy::todo, reason = "step 6 of the plan")]
    fn new() -> Self {
        todo!()
    }

    /// Returns the state of `subset`, or `None` if the table does not hold that set.
    ///
    /// The states of `subset` are in ascending sequence, and the set holds no duplicate. An
    /// [`NfaExecution`](super::NfaExecution) gives its states in that form.
    #[expect(clippy::todo, unused_variables, reason = "step 6 of the plan")]
    fn get(&self, subset: &[StateId]) -> Option<StateId> {
        todo!()
    }

    /// Adds `subset` as the set of `state`, then queues it.
    ///
    /// # Panics
    ///
    /// This function panics if the table already holds `subset`.
    #[expect(clippy::todo, unused_variables, reason = "step 6 of the plan")]
    fn add(&mut self, subset: &[StateId], state: StateId) {
        todo!()
    }

    /// Returns a set whose transitions determinization did not read, with its state.
    ///
    /// The result is `None` if determinization read the transitions of each set.
    #[expect(clippy::todo, reason = "step 6 of the plan")]
    fn next(&mut self) -> Option<(Vec<StateId>, StateId)> {
        todo!()
    }
}

/// Returns the lowest accept of the states in `subset`, or `None` if no state of `subset` accepts.
#[expect(clippy::todo, unused_variables, reason = "step 6 of the plan")]
fn lowest_accept<L, A>(nfa: &Nfa<L, A>, subset: &[StateId]) -> Option<A>
where
    A: Ord + Clone,
{
    todo!()
}

/// Returns the label of each transition that leaves a state of `subset`.
///
/// The result holds a duplicate if two states carry the same label. [`Label::divide`] removes
/// the duplicate.
#[expect(clippy::todo, unused_variables, reason = "step 6 of the plan")]
fn labels<L, A>(nfa: &Nfa<L, A>, subset: &[StateId]) -> Vec<L>
where
    L: Clone,
{
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::automaton::Automaton;
    use crate::automata::id::StartId;
    use crate::automata::label::Label;
    use crate::automata::nfa::NfaBuilder;
    use crate::automata::overflow::Part;
    use crate::automata::scan::{Match, longest_match};
    use crate::automata::testing::{Symbols, only, range};

    /// Builds an NFA. `build` adds the states, then it returns the start states.
    fn nfa(build: impl FnOnce(&mut NfaBuilder<Symbols, u32>) -> Vec<StateId>) -> Nfa<Symbols, u32> {
        let mut builder = NfaBuilder::new();
        let starts = build(&mut builder);
        builder
            .build(&starts)
            .expect("a test stays below the capacity")
    }

    /// Adds the states that match `text`, then makes the last state accept.
    fn literal(builder: &mut NfaBuilder<Symbols, u32>, text: &str, accept: u32) -> StateId {
        let start = builder.push();
        let end = text.chars().fold(start, |current, symbol| {
            let next = builder.push();
            builder.transition(current, only(symbol), next);
            next
        });
        builder.accept(end, accept);
        start
    }

    fn determinized(nfa: &Nfa<Symbols, u32>) -> Dfa<Symbols, u32> {
        nfa.determinize().expect("a test stays below the capacity")
    }

    /// Returns the longest match at the start of `input`, under `start`.
    fn scan<T>(automaton: &T, start: StartId, input: &str) -> Option<Match<u32>>
    where
        T: Automaton<Symbol = char, Accept = u32>,
    {
        let symbols: Vec<char> = input.chars().collect();
        longest_match(&mut automaton.execute(start), start, &symbols)
    }

    fn matched(accept: u32, length: usize) -> Option<Match<u32>> {
        Some(Match { accept, length })
    }

    fn first() -> StartId {
        StartId::new(0)
    }

    /// Builds an NFA of two rules that share the first symbol.
    fn branches() -> Nfa<Symbols, u32> {
        nfa(|builder| {
            let start = builder.push();
            let left = literal(builder, "ab", 0);
            let right = literal(builder, "ac", 1);
            builder.epsilon(start, left);
            builder.epsilon(start, right);
            vec![start]
        })
    }

    /// Builds an NFA whose start reads one symbol into two states that accept.
    fn two_accepts() -> Nfa<Symbols, u32> {
        nfa(|builder| {
            let start = builder.push();
            let high = builder.push();
            let low = builder.push();
            builder.transition(start, only('a'), high);
            builder.transition(start, only('a'), low);
            builder.accept(high, 7);
            builder.accept(low, 3);
            vec![start]
        })
    }

    /// Builds an NFA whose start carries two labels that share the symbols `d` to `f`.
    fn overlapping() -> Nfa<Symbols, u32> {
        nfa(|builder| {
            let start = builder.push();
            let left = builder.push();
            let right = builder.push();
            builder.transition(start, range('a', 'f'), left);
            builder.transition(start, range('d', 'z'), right);
            builder.accept(left, 0);
            builder.accept(right, 1);
            vec![start]
        })
    }

    /// Builds an NFA in the manner of a lexer: a keyword, an identifier, and a space.
    fn lexer() -> Nfa<Symbols, u32> {
        nfa(|builder| {
            let start = builder.push();
            let keyword = literal(builder, "if", 0);
            let entry = builder.push();
            let rest = builder.push();
            builder.transition(entry, range('a', 'z'), rest);
            builder.transition(rest, range('a', 'z'), rest);
            builder.transition(rest, range('0', '9'), rest);
            builder.accept(rest, 1);
            let space = literal(builder, " ", 2);
            builder.epsilon(start, keyword);
            builder.epsilon(start, entry);
            builder.epsilon(start, space);
            vec![start]
        })
    }

    #[test]
    fn a_chain_gives_one_state_for_each_prefix() {
        let nfa = nfa(|builder| vec![literal(builder, "ab", 0)]);
        let dfa = determinized(&nfa);

        assert_eq!(dfa.state_count(), 3);
        assert_eq!(dfa.start_count(), 1);
        assert_eq!(scan(&dfa, first(), "ab"), matched(0, 2));
        assert_eq!(scan(&dfa, first(), "a"), None);
    }

    #[test]
    fn an_alternation_joins_the_states_of_its_common_prefix() {
        let nfa = branches();
        let dfa = determinized(&nfa);

        assert_eq!(nfa.state_count(), 7);
        assert_eq!(dfa.state_count(), 4);
        assert_eq!(scan(&dfa, first(), "ab"), matched(0, 2));
        assert_eq!(scan(&dfa, first(), "ac"), matched(1, 2));
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
            builder.accept(end, 0);
            vec![start]
        });
        let dfa = determinized(&nfa);

        assert_eq!(dfa.state_count(), 3);
        assert_eq!(dfa.transitions(dfa.start_state(first())).len(), 2);
        assert_eq!(scan(&dfa, first(), "ac"), matched(0, 2));
        assert_eq!(scan(&dfa, first(), "bc"), matched(0, 2));
    }

    #[test]
    fn two_equal_labels_give_one_transition() {
        let nfa = two_accepts();
        let dfa = determinized(&nfa);

        assert_eq!(dfa.transitions(dfa.start_state(first())).len(), 1);
        assert_eq!(dfa.state_count(), 2);
    }

    #[test]
    fn the_accept_of_a_state_is_the_lowest_accept_of_its_states() {
        let nfa = two_accepts();
        let dfa = determinized(&nfa);
        let start = dfa.start_state(first());
        let target = dfa.step(start, 'a').expect("the symbol a reaches a state");

        assert_eq!(dfa.accept(start), None);
        assert_eq!(dfa.accept(target), Some(&3));
        assert_eq!(scan(&dfa, first(), "a"), matched(3, 1));
    }

    #[test]
    fn two_labels_that_share_symbols_divide_into_three_transitions() {
        let nfa = overlapping();
        let dfa = determinized(&nfa);

        assert_eq!(dfa.transitions(dfa.start_state(first())).len(), 3);
        assert_eq!(dfa.state_count(), 4);
        assert_eq!(scan(&dfa, first(), "a"), matched(0, 1));
        assert_eq!(scan(&dfa, first(), "d"), matched(0, 1));
        assert_eq!(scan(&dfa, first(), "g"), matched(1, 1));
        assert_eq!(scan(&dfa, first(), "A"), None);
    }

    #[test]
    fn a_start_that_accepts_gives_a_match_of_no_length() {
        let nfa = nfa(|builder| {
            let start = builder.push();
            builder.accept(start, 5);
            vec![start]
        });
        let dfa = determinized(&nfa);

        assert_eq!(dfa.state_count(), 1);
        assert_eq!(dfa.accept(dfa.start_state(first())), Some(&5));
        assert_eq!(scan(&dfa, first(), ""), matched(5, 0));
        assert_eq!(scan(&dfa, first(), "zz"), matched(5, 0));
    }

    #[test]
    fn each_start_of_the_nfa_gives_a_start_of_the_dfa() {
        let nfa = nfa(|builder| {
            let code = literal(builder, "a", 0);
            let string = literal(builder, "b", 1);
            vec![code, string]
        });
        let dfa = determinized(&nfa);

        assert_eq!(dfa.start_count(), 2);
        assert_eq!(dfa.state_count(), 4);
        assert_eq!(scan(&dfa, StartId::new(0), "a"), matched(0, 1));
        assert_eq!(scan(&dfa, StartId::new(0), "b"), None);
        assert_eq!(scan(&dfa, StartId::new(1), "b"), matched(1, 1));
        assert_eq!(scan(&dfa, StartId::new(1), "a"), None);
    }

    #[test]
    fn two_starts_of_the_same_states_give_one_state() {
        let nfa = nfa(|builder| {
            let start = literal(builder, "a", 0);
            vec![start, start]
        });
        let dfa = determinized(&nfa);

        assert_eq!(dfa.start_count(), 2);
        assert_eq!(dfa.state_count(), 2);
        assert_eq!(
            dfa.start_state(StartId::new(0)),
            dfa.start_state(StartId::new(1))
        );
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
            builder.accept(end, 0);
            vec![start]
        });
        let dfa = determinized(&nfa);

        assert_eq!(dfa.state_count(), 2);
        assert_eq!(scan(&dfa, first(), "a"), matched(0, 1));
    }

    #[test]
    fn a_state_has_a_maximum_of_one_transition_for_one_symbol() {
        for nfa in [branches(), overlapping(), lexer()] {
            let dfa = determinized(&nfa);
            for index in 0..dfa.state_count() {
                let state = StateId::new(index);
                for symbol in "abcdefgijkxyz019 !".chars() {
                    let count = dfa
                        .transitions(state)
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
    fn same_matches(nfa: &Nfa<Symbols, u32>, inputs: &[&str]) {
        let dfa = determinized(nfa);
        for (start, _) in nfa.starts() {
            for input in inputs {
                assert_eq!(
                    scan(&dfa, start, input),
                    scan(nfa, start, input),
                    "input {input:?} under start {}",
                    start.index()
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
        let nfa = nfa(|builder| vec![literal(builder, "ab", 0)]);

        assert_eq!(within(&nfa, 2), Err(Overflow::new(Part::States, 2)));
        assert!(within(&nfa, 3).is_ok());
    }

    fn state(index: usize) -> StateId {
        StateId::new(index)
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
    #[should_panic(expected = "the table already holds the set")]
    fn adding_a_set_that_the_table_holds_panics() {
        let mut subsets = Subsets::new();
        subsets.add(&[state(0)], state(0));
        subsets.add(&[state(0)], state(1));
    }

    #[test]
    fn the_lowest_accept_of_a_set_is_the_lowest_accept_of_its_states() {
        let nfa = two_accepts();

        assert_eq!(lowest_accept(&nfa, &[state(1), state(2)]), Some(3));
        assert_eq!(lowest_accept(&nfa, &[state(1)]), Some(7));
        assert_eq!(lowest_accept(&nfa, &[state(0)]), None);
        assert_eq!(lowest_accept(&nfa, &[]), None);
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
