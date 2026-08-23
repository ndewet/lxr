use std::collections::HashMap;

use proc_macro2::{Ident, Literal, TokenStream};
use quote::quote;

use super::rule::Rule;
use crate::graph::{Arena, Node, NodeId};

/// The largest graph that uses direct tests and forced inlining alone.
pub const LARGE_GRAPH: usize = 48;

/// The parts that each node of one lexer writes with.
///
/// [`step`](super::step()) makes it one time, then it writes each node of the graph with it.
pub struct Emitter<'a> {
    /// The graph of the lexer.
    pub arena: &'a Arena,
    /// What each rule gives, in the sequence of the rules.
    pub rules: &'a [Rule],
    /// The name of the enum of the tokens.
    pub token: &'a Ident,
    /// Whether each node takes the resume, in the sequence of the nodes.
    resumes: Vec<bool>,
    /// Whether an edge that closes a cycle carries the offset of an accept.
    carries: bool,
    /// Packed byte-class tables for run nodes.
    tables: Vec<[u8; 256]>,
    /// The table and bit of each run node.
    run_tests: Vec<Option<(usize, u8)>>,
}

impl<'a> Emitter<'a> {
    /// Creates the emitter of the lexer of `arena`.
    pub fn new(arena: &'a Arena, rules: &'a [Rule], token: &'a Ident) -> Self {
        let runs = run_tests(arena);
        Self {
            arena,
            rules,
            token,
            resumes: resumes(arena),
            carries: carries(arena),
            tables: runs.tables,
            run_tests: runs.tests,
        }
    }

    /// Returns the packed lookup tables that test the runs of a large graph.
    pub fn tables(&self) -> TokenStream {
        let tables = self.tables.iter().enumerate().map(|(index, bytes)| {
            let name = quote::format_ident!("RUN_TABLE_{index}");
            let bytes = bytes.iter().map(|&byte| Literal::u8_unsuffixed(byte));
            quote!(const #name: [u8; 256] = [#(#bytes),*];)
        });
        quote!(#(#tables)*)
    }

    /// Returns the lookup-table test for the run at `id`.
    pub fn run_test(&self, id: NodeId) -> Option<TokenStream> {
        self.run_tests[id.index()].map(|(table, mask)| {
            let name = quote::format_ident!("RUN_TABLE_{table}");
            let mask = Literal::u8_unsuffixed(mask);
            quote!((#name[byte as usize] & #mask) != 0)
        })
    }

    /// Returns whether the graph holds an edge that closes a cycle.
    ///
    /// The emitted source holds the driver of the step and the `Resume` only for such a graph.
    pub fn drives(&self) -> bool {
        self.arena.has_back_edge()
    }

    /// Returns whether an edge that closes a cycle carries the offset of an accept.
    ///
    /// The `Resume` holds the marker only for such a graph. A field that no node reads gives a
    /// warning in the crate of the author.
    pub fn carries(&self) -> bool {
        self.carries
    }

    /// Returns whether matcher state carries a start condition.
    pub fn has_condition(&self) -> bool {
        self.arena.start_count() > 1
    }

    /// Returns whether matcher state carries a cached run.
    pub fn has_run(&self) -> bool {
        self.arena.nodes().iter().enumerate().any(|(index, node)| {
            matches!(node, Node::Fork(fork) if fork.arms.iter().any(|arm| arm.edge.target.index() == index))
                && !self.ends_a_match(NodeId::new(index))
        })
    }

    /// Returns the expression that reads the numeric condition from `state`.
    pub fn condition(&self) -> proc_macro2::TokenStream {
        match (self.has_condition(), self.has_run()) {
            (true, true) => quote::quote!(state.0),
            (true, false) => quote::quote!(*state),
            (false, _) => quote::quote!(0),
        }
    }

    /// Returns the place to which a numeric condition is written.
    pub fn condition_place(&self) -> proc_macro2::TokenStream {
        match (self.has_condition(), self.has_run()) {
            (true, true) => quote::quote!(state.0),
            (true, false) => quote::quote!(*state),
            (false, _) => panic!("a lexer without conditions changes no condition"),
        }
    }

    /// Returns the expression that accesses the cached run.
    pub fn run(&self) -> proc_macro2::TokenStream {
        match (self.has_condition(), self.has_run()) {
            (true, true) => quote::quote!(state.1),
            (false, true) => quote::quote!((*state)),
            (_, false) => panic!("a matcher without a cached run does not access one"),
        }
    }

    /// Returns whether the node at `id` ends a match at its own offset.
    ///
    /// A byte that no arm of such a node holds gives the token of a rule, thus the scan goes on
    /// after that offset and it reads the bytes of that node one time. A node that ends no match
    /// keeps the run that it reads, because the scan can read those bytes again.
    ///
    /// # Panics
    ///
    /// This function panics if `id` is not a node of the graph.
    pub fn ends_a_match(&self, id: NodeId) -> bool {
        let Node::Fork(fork) = self.arena.node(id) else {
            return false;
        };

        matches!(
            self.arena.node(fork.miss.target),
            Node::Leaf(leaf) if leaf.here
        )
    }

    /// Returns the rule at `index`.
    ///
    /// # Panics
    ///
    /// This function panics if the lexer holds no rule at `index`.
    pub fn rule(&self, index: u16) -> &Rule {
        self.rules.get(usize::from(index)).unwrap_or_else(|| {
            panic!(
                "rule {index} is outside a lexer of {} rules",
                self.rules.len()
            )
        })
    }

    /// Returns the parameters that the function of the node at `id` takes.
    ///
    /// A parameter that the node does not read costs a register at each call. Thus a node takes
    /// the text only if it reads a value, the marker only if it holds an accept behind it, and the
    /// resume only if it reaches an edge that closes a cycle.
    ///
    /// # Panics
    ///
    /// This function panics if `id` is not a node of the graph, or if the lexer holds no rule that
    /// a leaf of the graph gives.
    pub fn shape(&self, id: NodeId) -> Shape {
        let resume = self.resumes[id.index()];

        match self.arena.node(id) {
            Node::Fork(_) | Node::Rope(_) => Shape {
                input: true,
                marker: self.arena.takes_marker(id),
                resume,
            },
            Node::Leaf(leaf) => Shape {
                input: self.rule(leaf.rule).value.is_some(),
                marker: !leaf.here,
                resume: false,
            },
            Node::Fault => Shape {
                input: false,
                marker: false,
                resume: false,
            },
        }
    }
}

/// The parameters that the function of one node takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shape {
    /// Whether the function takes the text of the input.
    pub input: bool,
    /// Whether the function takes the offset of the last accept.
    pub marker: bool,
    /// Whether the function writes the node at which the driver of the step goes on.
    pub resume: bool,
}

/// Returns whether each node of `arena` takes the resume.
///
/// A node that holds an edge of a cycle writes the resume. A node that calls such a node hands the
/// resume to it, thus it takes the resume as well. The pass repeats until nothing changes, because
/// one node can reach another one along a chain of calls.
fn resumes(arena: &Arena) -> Vec<bool> {
    let mut takes: Vec<bool> = arena
        .nodes()
        .iter()
        .map(|node| node.edges().iter().any(|edge| edge.back))
        .collect();

    let mut moved = true;
    while moved {
        moved = false;
        for (index, node) in arena.nodes().iter().enumerate() {
            if takes[index] {
                continue;
            }
            if node
                .edges()
                .iter()
                .any(|edge| !edge.back && takes[edge.target.index()])
            {
                takes[index] = true;
                moved = true;
            }
        }
    }

    takes
}

/// Returns whether an edge of a cycle carries the offset of an accept.
fn carries(arena: &Arena) -> bool {
    arena
        .nodes()
        .iter()
        .flat_map(Node::edges)
        .any(|edge| edge.back && arena.takes_marker(edge.target))
}

/// The shared byte-class tables and the test of each run node.
struct RunTables {
    tables: Vec<[u8; 256]>,
    tests: Vec<Option<(usize, u8)>>,
}

/// Builds shared byte-class tables for the run nodes of a large graph.
fn run_tests(arena: &Arena) -> RunTables {
    let mut tests = vec![None; arena.node_count()];
    if arena.node_count() <= LARGE_GRAPH {
        return RunTables {
            tables: Vec::new(),
            tests,
        };
    }

    let mut classes: Vec<[bool; 256]> = Vec::new();
    let mut indexes: HashMap<[bool; 256], usize> = HashMap::new();
    for (index, node) in arena.nodes().iter().enumerate() {
        let Node::Fork(fork) = node else { continue };
        let Some(arm) = fork
            .arms
            .iter()
            .find(|arm| arm.edge.target.index() == index)
        else {
            continue;
        };
        let mut class = [false; 256];
        for range in &arm.ranges {
            for byte in range.low..=range.high {
                class[usize::from(byte)] = true;
            }
        }
        let next = indexes.len();
        let class_index = *indexes.entry(class).or_insert_with(|| {
            classes.push(class);
            next
        });
        tests[index] = Some((class_index / 8, 1 << (class_index % 8)));
    }

    let mut tables = vec![[0; 256]; classes.len().div_ceil(8)];
    for (index, class) in classes.iter().enumerate() {
        for (byte, &member) in class.iter().enumerate() {
            if member {
                tables[index / 8][byte] |= 1 << (index % 8);
            }
        }
    }
    RunTables { tables, tests }
}
