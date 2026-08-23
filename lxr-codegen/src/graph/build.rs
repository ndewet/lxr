use std::collections::HashMap;

use super::arena::Arena;
use super::id::NodeId;
use super::node::{Arm, Carry, Edge, Fork, Leaf, Node, Rope};
use crate::automata::{Automaton, Label, NondeterministicFiniteAutomaton, Overflow, Part, StateId};
use crate::compiler::{Accepts, ByteRange, Compilation};

/// The maximum number of the nodes of a rule graph.
///
/// One node gives one function of the emitted source. A lexer above this limit gives a source that
/// the compiler of the author reads slowly.
pub const MAX_NODES: usize = 2048;

/// The maximum number of the bytes of one rope.
///
/// A comparison of a longer sequence costs more than one comparison, and a mismatch reads the
/// whole sequence a second time. The limit keeps both costs bounded.
const MAX_ROPE: usize = 16;

/// Builds the rule graph of `compilation`.
///
/// The graph holds one node for each set of positions that the scan can reach, exactly as a
/// determinization does. It differs in one point: a rule whose only continuation is a sequence of
/// forced bytes becomes one rope, and the rules that remain become the miss of that rope. Thus a
/// literal survives a rule that overlaps it, and the scan compares the whole literal at one time.
///
/// The longest match wins, and the earliest rule wins a tie at the same length.
///
/// # Errors
///
/// This function returns an [`Overflow`] if the lexer needs more than [`MAX_NODES`] nodes.
///
/// # Panics
///
/// This function panics if `compilation` holds no start state, or if it holds no owner for a state
/// of its automaton.
pub fn build(compilation: &Compilation<ByteRange, u16>) -> Result<Arena, Overflow> {
    let mut builder = Builder::new(&compilation.nfa, &compilation.accepts, &compilation.owners);
    let starts = builder.explore()?;
    let (mut nodes, markers) = (builder.nodes, builder.markers);
    let mut starts = starts;

    dedupe(&mut nodes, &markers, &mut starts);
    let (nodes, markers, starts) = compact(nodes, markers, &starts);
    let mut nodes = nodes;
    cycles(&mut nodes, &starts);

    Ok(Arena::new(nodes, starts, markers))
}

/// One set of positions of the automaton, and the accept behind them.
///
/// Two scans that reach the same positions with the same accept behind them read the rest of the
/// input in the same manner. Thus the state is the identity of a node.
///
/// [`pending`](Self::pending) is empty when a position of the state accepts. That accept is longer
/// than the one behind it, thus it wins and the one behind it changes nothing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct State {
    /// The positions of the automaton, in ascending sequence and with no duplicate.
    positions: Vec<StateId>,
    /// The rule that accepted before this offset, or `None`.
    pending: Option<u16>,
}

/// The state that an edge goes to, and the marker that the edge carries.
struct Step {
    state: State,
    carry: Carry,
}

/// Where the accept that a step carries comes from.
#[derive(Debug, Clone, Copy)]
enum Origin {
    /// The accept is this number of bytes after the offset of the node.
    Ahead(usize),
    /// The accept is at the offset that the node took as its marker.
    Marker,
}

/// Builds the nodes of a rule graph, one state at a time.
struct Builder<'a> {
    nfa: &'a NondeterministicFiniteAutomaton<ByteRange>,
    accepts: &'a Accepts<u16>,
    owners: &'a [Option<usize>],
    nodes: Vec<Node>,
    markers: Vec<bool>,
    states: HashMap<State, NodeId>,
    leaves: HashMap<(u16, bool), NodeId>,
    fault: Option<NodeId>,
    queue: Vec<(State, NodeId)>,
    seen: Vec<bool>,
}

impl<'a> Builder<'a> {
    fn new(
        nfa: &'a NondeterministicFiniteAutomaton<ByteRange>,
        accepts: &'a Accepts<u16>,
        owners: &'a [Option<usize>],
    ) -> Self {
        let count = nfa.state_count();
        Self {
            nfa,
            accepts,
            owners,
            nodes: Vec::new(),
            markers: Vec::new(),
            states: HashMap::new(),
            leaves: HashMap::new(),
            fault: None,
            queue: Vec::new(),
            seen: vec![false; count],
        }
    }

    /// Builds one node for each state that a start condition reaches.
    ///
    /// # Errors
    ///
    /// This function returns an [`Overflow`] above [`MAX_NODES`] nodes.
    fn explore(&mut self) -> Result<Vec<NodeId>, Overflow> {
        let mut starts = Vec::new();
        for &start in self.nfa.start_states() {
            let positions = self.closure(&[start]);
            let state = State {
                positions,
                pending: None,
            };
            starts.push(self.node_of(state)?);
        }

        while let Some((state, id)) = self.queue.pop() {
            let node = self.expand(&state)?;
            self.nodes[id.index()] = node;
        }

        Ok(starts)
    }

    /// Returns the node of `state`, and adds that node to the queue if it is new.
    ///
    /// # Errors
    ///
    /// This function returns an [`Overflow`] above [`MAX_NODES`] nodes.
    fn node_of(&mut self, state: State) -> Result<NodeId, Overflow> {
        if let Some(&id) = self.states.get(&state) {
            return Ok(id);
        }

        let marker = state.pending.is_some();
        let id = self.push(Node::Fault, marker)?;
        self.states.insert(state.clone(), id);
        self.queue.push((state, id));
        Ok(id)
    }

    /// Returns the node that gives the token of `rule`.
    ///
    /// A leaf of `here` ends its match at the offset of the leaf. Each other leaf takes the offset
    /// of the match as its marker.
    ///
    /// # Errors
    ///
    /// This function returns an [`Overflow`] above [`MAX_NODES`] nodes.
    fn leaf(&mut self, rule: u16, here: bool) -> Result<NodeId, Overflow> {
        if let Some(&id) = self.leaves.get(&(rule, here)) {
            return Ok(id);
        }

        let id = self.push(Node::Leaf(Leaf { rule, here }), !here)?;
        self.leaves.insert((rule, here), id);
        Ok(id)
    }

    /// Returns the node that reports that no rule matched.
    ///
    /// # Errors
    ///
    /// This function returns an [`Overflow`] above [`MAX_NODES`] nodes.
    fn fault(&mut self) -> Result<NodeId, Overflow> {
        if let Some(id) = self.fault {
            return Ok(id);
        }

        let id = self.push(Node::Fault, false)?;
        self.fault = Some(id);
        Ok(id)
    }

    /// Adds `node` to the arena, then returns its identifier.
    ///
    /// # Errors
    ///
    /// This function returns an [`Overflow`] above [`MAX_NODES`] nodes.
    fn push(&mut self, node: Node, marker: bool) -> Result<NodeId, Overflow> {
        if self.nodes.len() >= MAX_NODES {
            return Err(Overflow::new(Part::States, MAX_NODES));
        }

        let id = NodeId::new(self.nodes.len());
        self.nodes.push(node);
        self.markers.push(marker);
        Ok(id)
    }

    /// Returns the node of `state`.
    ///
    /// # Errors
    ///
    /// This function returns an [`Overflow`] above [`MAX_NODES`] nodes.
    fn expand(&mut self, state: &State) -> Result<Node, Overflow> {
        let own = self.own(&state.positions);
        let classes = self.classes(&state.positions);

        if classes.is_empty() {
            return Ok(match own.or(state.pending) {
                Some(rule) => Node::Leaf(Leaf {
                    rule,
                    here: own.is_some(),
                }),
                None => Node::Fault,
            });
        }

        if let Some((bytes, rule)) = self.rope_of(state, &classes, own) {
            return self.rope(state, own, &bytes, rule);
        }

        let mut arms: Vec<Arm> = Vec::new();
        for &(range, symbol) in &classes {
            let step = self.advance(state, own, &[symbol]);
            let carry = step.carry;
            let target = self.node_of(step.state)?;
            match arms.iter_mut().find(|arm| arm.edge.target == target) {
                Some(arm) => arm.ranges.push(range),
                None => arms.push(Arm {
                    ranges: vec![range],
                    edge: Edge {
                        target,
                        carry,
                        back: false,
                    },
                }),
            }
        }

        let miss = self.miss(own, state.pending)?;
        Ok(Node::Fork(Fork { arms, miss }))
    }

    /// Returns the rope of `state`, which reads `bytes` and which leaves the rule at `rule` out of
    /// its miss.
    ///
    /// # Errors
    ///
    /// This function returns an [`Overflow`] above [`MAX_NODES`] nodes.
    fn rope(
        &mut self,
        state: &State,
        own: Option<u16>,
        bytes: &[u8],
        rule: Option<usize>,
    ) -> Result<Node, Overflow> {
        let step = self.advance(state, own, bytes);
        let carry = step.carry;
        let then = self.node_of(step.state)?;

        let positions = match rule {
            Some(rule) => self.without(&state.positions, rule),
            None => Vec::new(),
        };
        let rest = self.own(&positions);
        let pending = if rest.is_some() {
            None
        } else {
            own.or(state.pending)
        };
        let miss = if pending.is_none() {
            Carry::Nothing
        } else if own.is_some() {
            Carry::Ahead(0)
        } else {
            Carry::Marker
        };
        let target = self.node_of(State { positions, pending })?;

        Ok(Node::Rope(Rope {
            bytes: bytes.to_vec(),
            then: Edge {
                target: then,
                carry,
                back: false,
            },
            miss: Edge {
                target,
                carry: miss,
                back: false,
            },
        }))
    }

    /// Returns the bytes of the rope of `state`, and the rule that the rope reads.
    ///
    /// The result is `None` if no rope is applicable. A rope of one rule leaves the rules that
    /// remain in its miss, thus it names that rule. A rope that each rule reads together leaves
    /// nothing, thus it names none.
    fn rope_of(
        &mut self,
        state: &State,
        classes: &[(ByteRange, u8)],
        own: Option<u16>,
    ) -> Option<(Vec<u8>, Option<usize>)> {
        let mut found = None;
        let mut count = 0;
        for rule in self.rules(&state.positions) {
            let positions = self.only(&state.positions, rule);
            let bytes = self.chain(&positions);
            if bytes.len() >= 2 {
                count += 1;
                found = Some((bytes, Some(rule)));
            }
        }
        if count == 1 {
            return found;
        }

        if own.is_none() && classes.len() == 1 && classes[0].0.low == classes[0].0.high {
            let bytes = self.chain(&state.positions);
            if bytes.len() >= 2 {
                return Some((bytes, None));
            }
        }

        None
    }

    /// Returns the bytes that `positions` read with no choice.
    ///
    /// The chain stops at a position that accepts, at a position that reads more than one byte,
    /// and at [`MAX_ROPE`] bytes.
    fn chain(&mut self, positions: &[StateId]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut current = positions.to_vec();

        while bytes.len() < MAX_ROPE {
            if current.is_empty() || self.own(&current).is_some() {
                break;
            }
            let classes = self.classes(&current);
            if classes.len() != 1 {
                break;
            }
            let (range, symbol) = classes[0];
            if range.low != range.high {
                break;
            }
            bytes.push(symbol);
            current = self.step(&current, symbol);
        }

        bytes
    }

    /// Returns the edge that a byte of no arm takes.
    ///
    /// # Errors
    ///
    /// This function returns an [`Overflow`] above [`MAX_NODES`] nodes.
    fn miss(&mut self, own: Option<u16>, pending: Option<u16>) -> Result<Edge, Overflow> {
        let (target, carry) = match (own, pending) {
            (Some(rule), _) => (self.leaf(rule, true)?, Carry::Nothing),
            (None, Some(rule)) => (self.leaf(rule, false)?, Carry::Marker),
            (None, None) => (self.fault()?, Carry::Nothing),
        };

        Ok(Edge {
            target,
            carry,
            back: false,
        })
    }

    /// Returns the state that `state` reaches after it reads `bytes`, and the marker that the edge
    /// carries.
    fn advance(&mut self, state: &State, own: Option<u16>, bytes: &[u8]) -> Step {
        let mut positions = state.positions.clone();
        let mut origin = match own {
            Some(rule) => Some((rule, Origin::Ahead(0))),
            None => state.pending.map(|rule| (rule, Origin::Marker)),
        };

        for (offset, &byte) in bytes.iter().enumerate() {
            positions = self.step(&positions, byte);
            if let Some(rule) = self.own(&positions) {
                origin = Some((rule, Origin::Ahead(offset + 1)));
            }
        }

        let reached = self.own(&positions);
        let pending = if reached.is_some() {
            None
        } else {
            origin.map(|(rule, _)| rule)
        };
        let carry = match (pending, origin) {
            (Some(_), Some((_, Origin::Ahead(offset)))) => Carry::Ahead(offset),
            (Some(_), Some((_, Origin::Marker))) => Carry::Marker,
            _ => Carry::Nothing,
        };

        Step {
            state: State { positions, pending },
            carry,
        }
    }

    /// Returns the positions that `positions` reach after they read `byte`.
    fn step(&mut self, positions: &[StateId], byte: u8) -> Vec<StateId> {
        let nfa = self.nfa;
        let targets: Vec<StateId> = nfa.step(positions, byte).collect();
        self.closure(&targets)
    }

    /// Returns `seeds` and each position that they reach with no byte.
    fn closure(&mut self, seeds: &[StateId]) -> Vec<StateId> {
        let mut found: Vec<StateId> = Vec::new();
        let mut stack: Vec<StateId> = Vec::new();

        for &seed in seeds {
            if !self.seen[seed.index()] {
                self.seen[seed.index()] = true;
                found.push(seed);
                stack.push(seed);
            }
        }
        while let Some(id) = stack.pop() {
            for &next in self.nfa.epsilons(id) {
                if !self.seen[next.index()] {
                    self.seen[next.index()] = true;
                    found.push(next);
                    stack.push(next);
                }
            }
        }
        for &id in &found {
            self.seen[id.index()] = false;
        }

        found.sort_unstable();
        found
    }

    /// Returns the rule of the highest precedence that accepts at `positions`.
    fn own(&self, positions: &[StateId]) -> Option<u16> {
        positions
            .iter()
            .filter_map(|&id| self.accepts.get(id).copied())
            .min()
    }

    /// Returns the byte ranges that `positions` read, divided into disjoint classes.
    fn classes(&self, positions: &[StateId]) -> Vec<(ByteRange, u8)> {
        let labels: Vec<ByteRange> = positions
            .iter()
            .flat_map(|&id| self.nfa.transitions(id))
            .map(|transition| transition.label)
            .collect();

        ByteRange::divide(&labels)
    }

    /// Returns the index of each rule that owns a position of `positions`, in ascending sequence.
    ///
    /// # Panics
    ///
    /// This function panics if a position of `positions` has no owner in the compilation.
    fn rules(&self, positions: &[StateId]) -> Vec<usize> {
        let mut rules: Vec<usize> = positions
            .iter()
            .filter_map(|&id| {
                *self
                    .owners
                    .get(id.index())
                    .unwrap_or_else(|| id.outside(self.owners.len()))
            })
            .collect();
        rules.sort_unstable();
        rules.dedup();
        rules
    }

    /// Returns the positions of `positions` that the rule at `rule` owns.
    fn only(&self, positions: &[StateId], rule: usize) -> Vec<StateId> {
        positions
            .iter()
            .copied()
            .filter(|id| self.owners.get(id.index()) == Some(&Some(rule)))
            .collect()
    }

    /// Returns the positions of `positions` that the rule at `rule` does not own.
    fn without(&self, positions: &[StateId], rule: usize) -> Vec<StateId> {
        positions
            .iter()
            .copied()
            .filter(|id| self.owners.get(id.index()) != Some(&Some(rule)))
            .collect()
    }
}

/// Joins each two nodes that read the same input into one node.
///
/// The construction makes one node for each state that it reaches, and two states can give the
/// same node. A join can make two more nodes equal, thus the pass repeats until nothing changes.
fn dedupe(nodes: &mut [Node], markers: &[bool], starts: &mut [NodeId]) {
    let mut previous: Vec<NodeId> = (0..nodes.len()).map(NodeId::new).collect();

    loop {
        let mut canonical: HashMap<(Node, bool), NodeId> = HashMap::new();
        let mut map: Vec<NodeId> = Vec::with_capacity(nodes.len());
        for (index, node) in nodes.iter().enumerate() {
            let id = *canonical
                .entry((node.clone(), markers[index]))
                .or_insert_with(|| NodeId::new(index));
            map.push(id);
        }
        if map == previous {
            return;
        }

        for node in nodes.iter_mut() {
            for edge in node.edges_mut() {
                edge.target = map[edge.target.index()];
            }
        }
        for start in starts.iter_mut() {
            *start = map[start.index()];
        }
        previous = map;
    }
}

/// Removes each node that no start reaches, then numbers the nodes that remain from zero.
fn compact(
    nodes: Vec<Node>,
    markers: Vec<bool>,
    starts: &[NodeId],
) -> (Vec<Node>, Vec<bool>, Vec<NodeId>) {
    let mut map: Vec<Option<NodeId>> = vec![None; nodes.len()];
    let mut order: Vec<NodeId> = Vec::new();
    let mut stack: Vec<NodeId> = starts.iter().rev().copied().collect();

    while let Some(id) = stack.pop() {
        if map[id.index()].is_some() {
            continue;
        }
        map[id.index()] = Some(NodeId::new(order.len()));
        order.push(id);
        for edge in nodes[id.index()].edges() {
            stack.push(edge.target);
        }
    }

    let mut kept: Vec<Node> = Vec::with_capacity(order.len());
    let mut flags: Vec<bool> = Vec::with_capacity(order.len());
    for &id in &order {
        let mut node = nodes[id.index()].clone();
        for edge in node.edges_mut() {
            edge.target =
                map[edge.target.index()].expect("each edge reaches a node that a start reaches");
        }
        kept.push(node);
        flags.push(markers[id.index()]);
    }

    let starts = starts
        .iter()
        .map(|&id| map[id.index()].expect("a start reaches itself"))
        .collect();

    (kept, flags, starts)
}

/// The colour of a node while the pass looks for the cycles of the graph.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Colour {
    /// The pass has not reached the node.
    White,
    /// The pass is inside the node.
    Grey,
    /// The pass has left the node.
    Black,
}

/// Marks each edge that closes a cycle of the graph.
///
/// The emitted source calls the node of an edge that closes no cycle. Thus the depth of the stack
/// is the length of the longest path that holds no cycle, and not the length of the input.
///
/// An arm of a fork that gives that same fork closes no cycle here. The emitted source reads that
/// arm as a run of bytes in one loop, thus it makes no call.
fn cycles(nodes: &mut [Node], starts: &[NodeId]) {
    let mut colour = vec![Colour::White; nodes.len()];

    for &start in starts {
        if colour[start.index()] != Colour::White {
            continue;
        }
        colour[start.index()] = Colour::Grey;
        let mut stack: Vec<(NodeId, usize)> = vec![(start, 0)];

        while let Some((id, cursor)) = stack.pop() {
            let edges = nodes[id.index()].edges();
            let Some(&edge) = edges.get(cursor) else {
                colour[id.index()] = Colour::Black;
                continue;
            };
            stack.push((id, cursor + 1));

            if runs(&nodes[id.index()], cursor, id) {
                continue;
            }
            match colour[edge.target.index()] {
                Colour::Grey => {
                    if let Some(edge) = nodes[id.index()].edges_mut().into_iter().nth(cursor) {
                        edge.back = true;
                    }
                }
                Colour::White => {
                    colour[edge.target.index()] = Colour::Grey;
                    stack.push((edge.target, 0));
                }
                Colour::Black => {}
            }
        }
    }
}

/// Returns `true` if the edge at `cursor` of `node` is an arm of a fork that gives that same fork.
fn runs(node: &Node, cursor: usize, id: NodeId) -> bool {
    match node {
        Node::Fork(fork) => fork
            .arms
            .get(cursor)
            .is_some_and(|arm| arm.edge.target == id),
        _ => false,
    }
}
