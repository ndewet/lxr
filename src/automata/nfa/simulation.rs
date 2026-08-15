use super::automaton::Nfa;
use super::id::StateId;

/// The scratch space that an epsilon closure over an [`Nfa`] uses.
///
/// A simulator holds one allocation for each closure that it makes. Use the same simulator for
/// each call. Thus the calls do not make a new allocation for each closure.
#[derive(Debug)]
pub struct Simulator<'a, L, A> {
    nfa: &'a Nfa<L, A>,
    reached: Vec<bool>,
    pending: Vec<StateId>,
}

impl<'a, L, A> Simulator<'a, L, A> {
    /// Creates a `Simulator` that runs over `nfa`.
    pub fn new(nfa: &'a Nfa<L, A>) -> Self {
        Self {
            nfa,
            reached: vec![false; nfa.state_count()],
            pending: Vec::new(),
        }
    }

    /// Returns the automaton that this simulator runs over.
    pub fn nfa(&self) -> &'a Nfa<L, A> {
        self.nfa
    }

    /// Finds each state that `seeds` goes to without a symbol, then writes the states into `out`.
    ///
    /// The function clears `out` first. The result holds the seeds. The result is in ascending
    /// sequence and holds no duplicate.
    ///
    /// # Panics
    ///
    /// This function panics if a state in `seeds` is not in the state arena.
    pub fn epsilon_closure(&mut self, seeds: &[StateId], out: &mut Vec<StateId>) {
        let nfa = self.nfa;

        for &seed in seeds {
            assert!(
                seed.index() < nfa.state_count(),
                "state {} is outside an arena of {} states",
                seed.index(),
                nfa.state_count()
            );
        }

        out.clear();
        self.pending.clear();

        for &seed in seeds {
            if !self.reached[seed.index()] {
                self.reached[seed.index()] = true;
                self.pending.push(seed);
            }
        }

        while let Some(id) = self.pending.pop() {
            out.push(id);
            for &target in nfa.epsilons(id) {
                if !self.reached[target.index()] {
                    self.reached[target.index()] = true;
                    self.pending.push(target);
                }
            }
        }

        for &id in out.iter() {
            self.reached[id.index()] = false;
        }

        out.sort_unstable();
    }
}

#[cfg(test)]
mod tests {
    use super::super::builder::NfaBuilder;
    use super::super::reference::{Symbols, only};
    use super::*;

    fn builder() -> NfaBuilder<Symbols, u32> {
        NfaBuilder::new()
    }

    fn closure(nfa: &Nfa<Symbols, u32>, seeds: &[StateId]) -> Vec<StateId> {
        let mut out = Vec::new();
        Simulator::new(nfa).epsilon_closure(seeds, &mut out);
        out
    }

    fn indices(states: &[StateId]) -> Vec<usize> {
        states.iter().map(|id| id.index()).collect()
    }

    #[test]
    fn the_closure_of_a_state_without_an_epsilon_transition_is_just_itself() {
        let mut builder = builder();
        let accept = builder.push();
        builder.accept(accept, 0);
        let nfa = builder.build(&[accept]);

        assert_eq!(closure(&nfa, &[accept]), vec![accept]);
    }

    #[test]
    fn the_closure_follows_each_epsilon_transition_of_a_state() {
        let mut builder = builder();
        let start = builder.push();
        let left = builder.push();
        let middle = builder.push();
        let right = builder.push();
        builder.epsilon(start, left);
        builder.epsilon(start, middle);
        builder.epsilon(start, right);
        let nfa = builder.build(&[start]);

        assert_eq!(indices(&closure(&nfa, &[start])), vec![0, 1, 2, 3]);
    }

    #[test]
    fn the_closure_stops_at_a_transition_with_a_label() {
        let mut builder = builder();
        let start = builder.push();
        let accept = builder.push();
        builder.transition(start, only('a'), accept);
        let nfa = builder.build(&[start]);

        assert_eq!(closure(&nfa, &[start]), vec![start]);
    }

    #[test]
    fn the_closure_terminates_on_an_epsilon_cycle() {
        let mut builder = builder();
        let left = builder.push();
        let right = builder.push();
        let accept = builder.push();
        builder.epsilon(left, right);
        builder.epsilon(left, accept);
        builder.epsilon(right, left);
        builder.epsilon(right, accept);
        let nfa = builder.build(&[left]);

        assert_eq!(indices(&closure(&nfa, &[left])), vec![0, 1, 2]);
    }

    #[test]
    fn the_closure_is_sorted_and_deduplicated() {
        let mut builder = builder();
        let first = builder.push();
        let second = builder.push();
        let left = builder.push();
        let right = builder.push();
        builder.epsilon(left, first);
        builder.epsilon(left, second);
        builder.epsilon(right, second);
        builder.epsilon(right, first);
        let nfa = builder.build(&[left]);

        assert_eq!(
            indices(&closure(&nfa, &[right, left, right])),
            vec![0, 1, 2, 3]
        );
    }

    #[test]
    #[should_panic(expected = "state 9 is outside an arena of 2 states")]
    fn a_closure_over_a_seed_outside_the_arena_panics() {
        let mut builder = builder();
        let start = builder.push();
        builder.push();
        let nfa = builder.build(&[start]);

        closure(&nfa, &[StateId::new(9)]);
    }

    #[test]
    fn a_reused_closure_buffer_does_not_leak_between_calls() {
        let mut builder = builder();
        let left = builder.push();
        let right = builder.push();
        let split = builder.push();
        builder.epsilon(split, left);
        builder.epsilon(split, right);
        let nfa = builder.build(&[split]);

        let mut simulator = Simulator::new(&nfa);
        let mut closure = Vec::new();

        simulator.epsilon_closure(&[split], &mut closure);
        assert_eq!(closure, vec![left, right, split]);

        simulator.epsilon_closure(&[left], &mut closure);
        assert_eq!(closure, vec![left]);

        simulator.epsilon_closure(&[split], &mut closure);
        assert_eq!(closure, vec![left, right, split]);
    }
}
