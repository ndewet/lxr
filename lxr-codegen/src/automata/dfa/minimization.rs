use crate::automata::{
    BuildError, StateId,
    dfa::{Builder, Dfa},
    label::Partitionable,
};

impl<L, A> Dfa<L, A>
where
    L: Partitionable,
    A: Eq,
{
    pub(crate) fn minimize(self) -> Result<Self, BuildError> {
        minimize(self)
    }
}

fn minimize<L, A>(dfa: Dfa<L, A>) -> Result<Dfa<L, A>, BuildError>
where
    L: Partitionable,
    A: Eq,
{
    let classes = global_classes(&dfa);
    let dead_state = StateId::new(dfa.state_count());
    let mut partition = initial_partition(&dfa, dead_state);

    loop {
        let next = refine(&classes, &partition, dead_state);
        if next.states_by_block.len() == partition.states_by_block.len() {
            break;
        }
        partition = next;
    }

    build_minimized(dfa, &classes, partition, dead_state)
}

struct GlobalClass<L> {
    label: L,
    target_by_source: Vec<Option<StateId>>,
}

fn global_classes<L, A>(dfa: &Dfa<L, A>) -> Vec<GlobalClass<L>>
where
    L: Partitionable,
{
    let mut labels = Vec::new();
    let mut transitions = Vec::new();

    for index in 0..dfa.state_count() {
        let source = StateId::new(index);

        for transition in dfa.transitions(source) {
            labels.push(transition.label.clone());
            let target = transition.target;
            transitions.push((source, target));
        }
    }

    L::partition(&labels)
        .into_iter()
        .map(|class| {
            let mut target_by_source = vec![None; dfa.state_count()];

            for &index in class.get_matching_labels() {
                let (source, target) = transitions[index];

                debug_assert!(
                    target_by_source[source.index()].replace(target).is_none(),
                    "invariant broken: transitions can not overlap"
                );
            }
            GlobalClass {
                label: class.get_label().clone(),
                target_by_source,
            }
        })
        .collect()
}

struct Partition {
    blocks_by_state: Vec<usize>,
    states_by_block: Vec<Vec<StateId>>,
}

fn initial_partition<L, A>(dfa: &Dfa<L, A>, dead_state: StateId) -> Partition
where
    A: Eq,
{
    let mut states_by_block: Vec<Vec<StateId>> = Vec::new();

    for index in 0..=dead_state.index() {
        let state = StateId::new(index);
        let block = states_by_block
            .iter()
            .position(|members| {
                let representative = members[0];
                accept_of(dfa, state, dead_state) == accept_of(dfa, representative, dead_state)
            })
            .unwrap_or_else(|| {
                states_by_block.push(Vec::new());
                states_by_block.len() - 1
            });
        states_by_block[block].push(state);
    }

    let mut blocks_by_state = vec![0; dead_state.index() + 1];
    for (block, states) in states_by_block.iter().enumerate() {
        for &state in states {
            blocks_by_state[state.index()] = block;
        }
    }

    Partition {
        blocks_by_state,
        states_by_block,
    }
}

fn refine<L>(classes: &[GlobalClass<L>], partition: &Partition, dead_state: StateId) -> Partition {
    let mut states_by_block = Vec::new();

    for states in &partition.states_by_block {
        let mut groups: Vec<(Vec<usize>, Vec<StateId>)> = Vec::new();

        for &state in states {
            let signature = classes
                .iter()
                .map(|class| {
                    let target = (state != dead_state)
                        .then(|| class.target_by_source[state.index()])
                        .flatten()
                        .unwrap_or(dead_state);
                    partition.blocks_by_state[target.index()]
                })
                .collect();
            let group = groups
                .iter()
                .position(|(other, _)| *other == signature)
                .unwrap_or_else(|| {
                    groups.push((signature, Vec::new()));
                    groups.len() - 1
                });
            groups[group].1.push(state);
        }

        states_by_block.extend(groups.into_iter().map(|(_, states)| states));
    }

    Partition::new(states_by_block, dead_state)
}

fn build_minimized<L, A>(
    dfa: Dfa<L, A>,
    classes: &[GlobalClass<L>],
    partition: Partition,
    dead_state: StateId,
) -> Result<Dfa<L, A>, BuildError>
where
    L: Partitionable,
{
    let dead_block = partition.blocks_by_state[dead_state.index()];
    let starts_by_block: Vec<usize> = dfa
        .start_states()
        .iter()
        .map(|state| partition.blocks_by_state[state.index()])
        .collect();
    let emits_dead = starts_by_block.contains(&dead_block);
    let mut builder = Builder::new();
    let mut state_by_block = vec![None; partition.states_by_block.len()];

    for (block, _) in partition.states_by_block.iter().enumerate() {
        if block != dead_block || emits_dead {
            state_by_block[block] = Some(builder.add_state());
        }
    }

    let mut accepts = dfa.into_accepts();
    for (block, states) in partition.states_by_block.iter().enumerate() {
        let Some(output) = state_by_block[block] else {
            continue;
        };
        let Some(&representative) = states.iter().find(|&&state| state != dead_state) else {
            continue;
        };
        if let Some(accept) = accepts[representative.index()].take() {
            builder.set_accept(output, accept);
        }
        if block == dead_block {
            continue;
        }

        for class in classes {
            let target = class.target_by_source[representative.index()].unwrap_or(dead_state);
            let target_block = partition.blocks_by_state[target.index()];
            if target_block != dead_block {
                builder.add_transition(
                    output,
                    class.label.clone(),
                    state_by_block[target_block].expect("a non-dead block has an output state"),
                );
            }
        }
    }

    let starts = starts_by_block
        .into_iter()
        .map(|block| state_by_block[block].expect("start blocks have output states"))
        .collect::<Vec<_>>();
    builder.build(&starts)
}

impl Partition {
    fn new(states_by_block: Vec<Vec<StateId>>, dead_state: StateId) -> Self {
        let mut blocks_by_state = vec![0; dead_state.index() + 1];
        for (block, states) in states_by_block.iter().enumerate() {
            for &state in states {
                blocks_by_state[state.index()] = block;
            }
        }
        Self {
            blocks_by_state,
            states_by_block,
        }
    }
}

fn accept_of<L, A>(dfa: &Dfa<L, A>, state: StateId, dead_state: StateId) -> Option<&A> {
    (state != dead_state).then(|| dfa.accept(state)).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::{dfa::Builder, encoding::ByteRange};

    #[test]
    fn global_classes_are_empty_without_labeled_transitions() {
        let mut builder: Builder<ByteRange> = Builder::new();
        let start = builder.add_state();
        let dfa = builder
            .build(&[start])
            .expect("the test DFA is below its capacity");

        assert!(global_classes(&dfa).is_empty());
    }

    #[test]
    fn a_global_class_records_each_source_target_or_dead_state() {
        let mut builder: Builder<ByteRange> = Builder::new();
        let first = builder.add_state();
        let second = builder.add_state();
        let target = builder.add_state();
        builder.add_transition(first, ByteRange::new(b'a', b'c'), target);
        let dfa = builder
            .build(&[first, second])
            .expect("the test DFA is below its capacity");

        let classes = global_classes(&dfa);

        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].label, ByteRange::new(b'a', b'c'));
        assert_eq!(classes[0].target_by_source, vec![Some(target), None, None]);
    }

    #[test]
    fn global_classes_split_overlaps_between_dfa_states() {
        let mut builder: Builder<ByteRange> = Builder::new();
        let first = builder.add_state();
        let second = builder.add_state();
        let left = builder.add_state();
        let right = builder.add_state();
        builder.add_transition(first, ByteRange::new(b'a', b'f'), left);
        builder.add_transition(second, ByteRange::new(b'd', b'z'), right);
        let dfa = builder
            .build(&[first, second])
            .expect("the test DFA is below its capacity");

        let classes = global_classes(&dfa);

        assert_eq!(
            classes
                .iter()
                .map(|class| (class.label, class.target_by_source.clone()))
                .collect::<Vec<_>>(),
            vec![
                (
                    ByteRange::new(b'a', b'c'),
                    vec![Some(left), None, None, None],
                ),
                (
                    ByteRange::new(b'd', b'f'),
                    vec![Some(left), Some(right), None, None],
                ),
                (
                    ByteRange::new(b'g', b'z'),
                    vec![None, Some(right), None, None],
                ),
            ]
        );
    }

    #[test]
    fn initial_partition_groups_non_accepting_states_with_dead() {
        let mut builder: Builder<ByteRange> = Builder::new();
        let first = builder.add_state();
        let second = builder.add_state();
        let dfa = builder
            .build(&[first])
            .expect("the test DFA is below its capacity");
        let dead = StateId::new(dfa.state_count());

        let partition = initial_partition(&dfa, dead);

        assert_eq!(partition.blocks_by_state.len(), dfa.state_count() + 1);
        assert_eq!(
            partition.blocks_by_state[first.index()],
            partition.blocks_by_state[second.index()]
        );
        assert_eq!(
            partition.blocks_by_state[first.index()],
            partition.blocks_by_state[dead.index()]
        );
        let block = partition.blocks_by_state[dead.index()];
        assert!(partition.states_by_block[block].contains(&dead));
    }

    #[test]
    fn initial_partition_groups_equal_accept_values_and_separates_distinct_ones() {
        let mut builder: Builder<ByteRange, u32> = Builder::new();
        let first = builder.add_state();
        let second = builder.add_state();
        let different = builder.add_state();
        let non_accepting = builder.add_state();
        builder.set_accept(first, 7);
        builder.set_accept(second, 7);
        builder.set_accept(different, 3);
        let dfa = builder
            .build(&[first])
            .expect("the test DFA is below its capacity");
        let dead = StateId::new(dfa.state_count());

        let partition = initial_partition(&dfa, dead);

        let first_block = partition.blocks_by_state[first.index()];
        assert_eq!(first_block, partition.blocks_by_state[second.index()]);
        assert_ne!(first_block, partition.blocks_by_state[different.index()]);
        assert_ne!(
            first_block,
            partition.blocks_by_state[non_accepting.index()]
        );
        assert_eq!(
            partition.blocks_by_state[non_accepting.index()],
            partition.blocks_by_state[dead.index()]
        );
    }

    #[test]
    fn minimization_merges_identical_suffixes_after_a_choice() {
        let mut builder: Builder<ByteRange> = Builder::new();
        let start = builder.add_state();
        let mut left = builder.add_state();
        let mut right = builder.add_state();
        builder.add_transition(start, ByteRange::new(b'a', b'a'), left);
        builder.add_transition(start, ByteRange::new(b'b', b'b'), right);

        for _ in 0..3 {
            let next_left = builder.add_state();
            let next_right = builder.add_state();
            builder.add_transition(left, ByteRange::new(b'a', b'a'), next_left);
            builder.add_transition(right, ByteRange::new(b'a', b'a'), next_right);
            left = next_left;
            right = next_right;
        }

        let accept = builder.add_state();
        builder.add_transition(left, ByteRange::new(b'a', b'a'), accept);
        builder.add_transition(right, ByteRange::new(b'a', b'a'), accept);
        builder.mark_accept(accept);
        let dfa = builder
            .build(&[start])
            .expect("the test DFA is below its capacity");

        let dfa = dfa.minimize().expect("the minimized DFA is below capacity");

        let start = dfa.start_state(0);
        let after_a = dfa.step(start, b'a').expect("the A branch is present");
        let after_b = dfa.step(start, b'b').expect("the B branch is present");
        assert_eq!(dfa.state_count(), 6);
        assert_eq!(after_a, after_b);

        let mut state = after_a;
        for _ in 0..4 {
            state = dfa.step(state, b'a').expect("the A suffix is present");
        }
        assert!(dfa.accepts(state));
    }

    #[test]
    fn minimization_keeps_distinct_accept_values_separate() {
        let mut builder: Builder<ByteRange, &'static str> = Builder::new();
        let start = builder.add_state();
        let first = builder.add_state();
        let second = builder.add_state();
        builder.add_transition(start, ByteRange::new(b'a', b'a'), first);
        builder.add_transition(start, ByteRange::new(b'b', b'b'), second);
        builder.set_accept(first, "first");
        builder.set_accept(second, "second");
        let dfa = builder
            .build(&[start])
            .expect("the test DFA is below its capacity");

        let dfa = dfa.minimize().expect("the minimized DFA is below capacity");

        let start = dfa.start_state(0);
        let first = dfa
            .step(start, b'a')
            .expect("the first transition is present");
        let second = dfa
            .step(start, b'b')
            .expect("the second transition is present");
        assert_ne!(first, second);
        assert_eq!(dfa.accept(first), Some(&"first"));
        assert_eq!(dfa.accept(second), Some(&"second"));
    }

    #[test]
    fn minimization_keeps_a_dead_equivalent_start_state() {
        let mut builder: Builder<ByteRange> = Builder::new();
        let start = builder.add_state();
        let dfa = builder
            .build(&[start])
            .expect("the test DFA is below its capacity");

        let dfa = dfa.minimize().expect("the minimized DFA is below capacity");

        assert_eq!(dfa.state_count(), 1);
        assert!(dfa.transitions(dfa.start_state(0)).is_empty());
    }
}
