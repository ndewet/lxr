use super::automaton::Nfa;
use super::state::StateId;

/// Scratch space for taking epsilon closures over an [`Nfa`].
///
/// A simulator holds one allocation per closure it takes, so reusing it across calls avoids
/// reallocating for every closure.
#[derive(Debug)]
pub struct Simulator<'a> {
    nfa: &'a Nfa,
    reached: Vec<bool>,
    pending: Vec<StateId>,
}

impl<'a> Simulator<'a> {
    /// Creates a `Simulator` over `nfa`.
    pub fn new(nfa: &'a Nfa) -> Self {
        Self {
            nfa,
            reached: vec![false; nfa.state_count()],
            pending: Vec::new(),
        }
    }

    /// Returns the automaton this simulator runs over.
    pub fn nfa(&self) -> &'a Nfa {
        self.nfa
    }

    /// Writes the states reached from `seeds` without consuming a byte into `out`, clearing it
    /// first.
    ///
    /// The seeds themselves are included. The result is sorted and holds no duplicates.
    ///
    /// # Panics
    ///
    /// Panics if any of `seeds` is outside the arena.
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
            for successor in nfa.state(id).epsilon_successors() {
                if !self.reached[successor.index()] {
                    self.reached[successor.index()] = true;
                    self.pending.push(successor);
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
    use super::super::state::{AcceptId, State};
    use super::*;

    fn closure(nfa: &Nfa, seeds: &[StateId]) -> Vec<StateId> {
        let mut out = Vec::new();
        Simulator::new(nfa).epsilon_closure(seeds, &mut out);
        out
    }

    fn indices(states: &[StateId]) -> Vec<usize> {
        states.iter().map(|id| id.index()).collect()
    }

    #[test]
    fn the_closure_of_a_terminal_state_is_just_itself() {
        let mut builder = NfaBuilder::new();
        let accept = builder.push(State::Match {
            accept: AcceptId::new(0),
        });
        let nfa = builder.build(&[accept]);

        assert_eq!(closure(&nfa, &[accept]), vec![accept]);
    }

    #[test]
    fn the_closure_follows_both_branches_of_a_split() {
        let mut builder = NfaBuilder::new();
        let left = builder.push(State::Match {
            accept: AcceptId::new(0),
        });
        let right = builder.push(State::Match {
            accept: AcceptId::new(1),
        });
        let split = builder.push(State::Split {
            first: left,
            second: right,
        });
        let nfa = builder.build(&[split]);

        assert_eq!(indices(&closure(&nfa, &[split])), vec![0, 1, 2]);
    }

    #[test]
    fn the_closure_stops_at_a_byte_transition() {
        let mut builder = NfaBuilder::new();
        let accept = builder.push(State::Match {
            accept: AcceptId::new(0),
        });
        let range = builder.push(State::Range {
            low: b'a',
            high: b'a',
            next: accept,
        });
        let nfa = builder.build(&[range]);

        assert_eq!(closure(&nfa, &[range]), vec![range]);
    }

    #[test]
    fn the_closure_terminates_on_an_epsilon_cycle() {
        let mut builder = NfaBuilder::new();
        let left = builder.reserve();
        let right = builder.reserve();
        let accept = builder.push(State::Match {
            accept: AcceptId::new(0),
        });
        builder.fill(
            left,
            State::Split {
                first: right,
                second: accept,
            },
        );
        builder.fill(
            right,
            State::Split {
                first: left,
                second: accept,
            },
        );
        let nfa = builder.build(&[left]);

        assert_eq!(indices(&closure(&nfa, &[left])), vec![0, 1, 2]);
    }

    #[test]
    fn the_closure_is_sorted_and_deduplicated() {
        let mut builder = NfaBuilder::new();
        let accept = builder.push(State::Match {
            accept: AcceptId::new(0),
        });
        let other = builder.push(State::Match {
            accept: AcceptId::new(1),
        });
        let left = builder.push(State::Split {
            first: accept,
            second: other,
        });
        let right = builder.push(State::Split {
            first: other,
            second: accept,
        });
        let nfa = builder.build(&[left]);

        assert_eq!(
            indices(&closure(&nfa, &[right, left, right])),
            vec![0, 1, 2, 3]
        );
    }

    #[test]
    #[should_panic(expected = "state 9 is outside an arena of 2 states")]
    fn a_closure_over_a_seed_outside_the_arena_panics() {
        let mut builder = NfaBuilder::new();
        let accept = builder.push(State::Match {
            accept: AcceptId::new(0),
        });
        builder.push(State::Match {
            accept: AcceptId::new(1),
        });
        let nfa = builder.build(&[accept]);

        closure(&nfa, &[StateId::new(9)]);
    }

    #[test]
    fn a_reused_closure_buffer_does_not_leak_between_calls() {
        let mut builder = NfaBuilder::new();
        let left = builder.push(State::Match {
            accept: AcceptId::new(0),
        });
        let right = builder.push(State::Match {
            accept: AcceptId::new(1),
        });
        let split = builder.push(State::Split {
            first: left,
            second: right,
        });
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
