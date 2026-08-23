use proc_macro2::{Ident, Literal, TokenStream};
use quote::quote;

use super::emitter::Emitter;
use super::node::{function, name};
use super::rule::Rule;
use crate::graph::{Arena, Node, NodeId};

/// Returns the `step` function of the graph of `arena`.
///
/// The function holds one nested function for each node. A fork reads one byte and it calls the
/// node of that byte, a rope compares a sequence of bytes at one time, and a leaf writes the
/// token, the length, and the start condition. Thus the state of the scan lives in the program
/// counter, and no step reads a table.
///
/// A nested function names no generic parameter. It names the enum of the tokens instead, thus
/// the source of one lexer holds no bound of the runtime.
///
/// # Panics
///
/// This function panics if `rules` holds no rule that a leaf of `arena` gives.
pub fn step(arena: &Arena, rules: &[Rule], token: &Ident) -> TokenStream {
    let emitter = Emitter::new(arena, rules, token);
    let functions = (0..arena.node_count()).map(|index| function(&emitter, NodeId::new(index)));
    let resume = resume(&emitter);
    let enter = enter(&emitter);
    let driver = driver(&emitter);

    quote! {
        fn step(input: &str, at: usize, step: &mut ::lxr::Step<#token>) {
            #resume
            #(#functions)*
            #enter
            #driver
        }
    }
}

/// Returns the declaration of the `Resume`, and the value that the step starts with.
///
/// The result is empty if no edge of the graph closes a cycle.
fn resume(emitter: &Emitter<'_>) -> TokenStream {
    if !emitter.drives() {
        return TokenStream::new();
    }

    let field = emitter.carries().then(|| quote!(marker: usize,));
    let value = emitter.carries().then(|| quote!(marker: 0,));

    quote! {
        struct Resume {
            node: u32,
            index: usize,
            #field
        }

        let mut resume = Resume { node: 0, index: 0, #value };
    }
}

/// Returns the call that enters the graph under the start condition of the step.
///
/// A lexer of one start condition enters one node, thus it reads no condition.
///
/// # Panics
///
/// This function panics if the graph holds no start node.
fn enter(emitter: &Emitter<'_>) -> TokenStream {
    let arena = emitter.arena;
    let calls: Vec<TokenStream> = (0..arena.start_count())
        .map(|condition| call(emitter, arena.start(condition)))
        .collect();

    if let [only] = calls.as_slice() {
        return only.clone();
    }

    let indexes: Vec<Literal> = (0..arena.start_count())
        .map(Literal::usize_unsuffixed)
        .collect();
    quote! {
        let condition = step.condition;
        match condition {
            #(#indexes => { #calls })*
            condition => panic!(
                "condition {condition} is not a start condition of this lexer"
            ),
        }
    }
}

/// Returns the loop that reads each node at which an edge of a cycle stopped.
///
/// The result is empty if no edge of the graph closes a cycle.
fn driver(emitter: &Emitter<'_>) -> TokenStream {
    if !emitter.drives() {
        return TokenStream::new();
    }

    let arms = resumed(emitter.arena).into_iter().map(|target| {
        let number = Literal::u32_unsuffixed(target.number() + 1);
        let shape = emitter.shape(target);
        let name = name(target);
        let input = shape.input.then(|| quote!(input,));
        let take = shape.marker.then(|| quote!(let marker = resume.marker;));
        let marker = shape.marker.then(|| quote!(marker,));
        let carry = shape.resume.then(|| quote!(&mut resume,));
        quote! {
            #number => {
                let index = resume.index;
                #take
                resume.node = 0;
                #name(#input at, index, #marker step, #carry);
            }
        }
    });

    quote! {
        while resume.node != 0 {
            match resume.node {
                #(#arms)*
                node => panic!("node {node} is not a node of this lexer"),
            }
        }
    }
}

/// Returns each node that an edge of a cycle goes to, in ascending sequence.
fn resumed(arena: &Arena) -> Vec<NodeId> {
    let mut targets: Vec<NodeId> = arena
        .nodes()
        .iter()
        .flat_map(Node::edges)
        .filter(|edge| edge.back)
        .map(|edge| edge.target)
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

/// Returns the call that enters the node at `id` at the start of the match.
///
/// A start node holds no accept behind it, thus the call carries no marker. The resume lives in
/// the step, thus this call gives a reference to it and a node hands that reference on.
fn call(emitter: &Emitter<'_>, id: NodeId) -> TokenStream {
    let shape = emitter.shape(id);
    let name = name(id);
    let input = shape.input.then(|| quote!(input,));
    let resume = shape.resume.then(|| quote!(&mut resume,));

    quote!(#name(#input at, at, step, #resume);)
}
