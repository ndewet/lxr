use proc_macro2::{Ident, Literal, TokenStream};
use quote::quote;

use super::emitter::Emitter;
use super::node::{function, fused_body, name};
use super::rule::Rule;
use crate::graph::{Arena, Node, NodeId};

/// Returns the `step` function of the graph of `arena`.
///
/// Hot nodes are emitted together as structured control flow. A fork reads one byte and enters
/// the block of that byte, a rope compares a sequence at one time, and a leaf returns the match
/// directly. Only paths outside the hot region retain nested helper functions.
///
/// A nested function names no generic parameter. It names the enum of the tokens instead, thus
/// the source of one lexer holds no bound of the runtime.
///
/// # Panics
///
/// This function panics if `rules` holds no rule that a leaf of `arena` gives.
pub fn step(arena: &Arena, rules: &[Rule], token: &Ident) -> TokenStream {
    let emitter = Emitter::new(arena, rules, token);
    let functions = (0..arena.node_count())
        .map(NodeId::new)
        .filter(|&id| emitter.is_outlined(id))
        .map(|id| function(&emitter, id));
    let resume = resume(&emitter);
    let driver = driver(&emitter);
    let state = state(&emitter);
    let tables = emitter.tables();

    quote! {
        #state

        #[inline(always)]
        fn step(input: &str, at: usize, state: &mut Self::State) -> ::lxr::Match<Self> {
            #tables
            #resume
            #(#functions)*
            let mut at = at;
            #driver
        }
    }
}

/// Returns the associated matcher state and its accessors.
fn state(emitter: &Emitter<'_>) -> TokenStream {
    let (kind, initial) = match (emitter.has_condition(), emitter.has_run()) {
        (false, false) => (quote!(()), quote!({})),
        (true, false) => (quote!(u16), quote!(0)),
        (false, true) => (quote!(::lxr::Run), quote!(::lxr::Run::new())),
        (true, true) => (quote!((u16, ::lxr::Run)), quote!((0, ::lxr::Run::new()))),
    };
    let condition = emitter.condition();
    quote! {
        type State = #kind;
        fn initial() -> Self::State { #initial }
        fn state_condition(state: &Self::State) -> u16 { #condition }
    }
}

/// Returns the resume cursor used only after an edge closes a cycle.
fn resume(emitter: &Emitter<'_>) -> TokenStream {
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

/// Returns the loop that reads each node at which an edge of a cycle stopped.
///
/// The result is empty if no edge of the graph closes a cycle.
fn driver(emitter: &Emitter<'_>) -> TokenStream {
    let arms = resumed(emitter.arena).into_iter().map(|target| {
        let number = Literal::u32_unsuffixed(target.number() + 1);
        let shape = emitter.shape(target);
        let take = shape.marker.then(|| quote!(let marker = resume.marker;));
        let body = if emitter.is_hot(target) {
            fused_body(emitter, target)
        } else {
            let name = name(target);
            let input = shape.input.then(|| quote!(input,));
            let marker = shape.marker.then(|| quote!(marker,));
            let carry = shape.resume.then(|| quote!(&mut resume,));
            quote! {
                match #name(#input at, index, #marker state, #carry) {
                    ::core::option::Option::Some(found) => return found,
                    ::core::option::Option::None => continue 'matcher,
                }
            }
        };
        quote! {
            #number => {
                let index = resume.index;
                #take
                resume.node = 0;
                #body
            }
        }
    });

    let entries: Vec<(Literal, TokenStream)> = (0..emitter.arena.start_count())
        .map(|condition| {
            let target = emitter.arena.start(condition);
            let body = entry_body(emitter, target);
            let condition = Literal::usize_unsuffixed(condition);
            (condition, body)
        })
        .collect();
    let entry = if let [(_, only)] = entries.as_slice() {
        quote!(#only)
    } else {
        let condition = emitter.condition();
        let indexes = entries.iter().map(|(index, _)| index);
        let bodies = entries.iter().map(|(_, body)| body);
        quote! {
            let condition = #condition;
            match condition {
                #(#indexes => { #bodies })*
                condition => panic!(
                    "condition {condition} is not a start condition of this lexer"
                ),
            }
        }
    };

    quote! {
        'matcher: loop {
            if resume.node == 0 {
                #entry
            }
            match resume.node {
                #(#arms)*
                node => panic!("node {node} is not a node of this lexer"),
            }
        }
    }
}

/// Returns direct entry into a start node, before any resume dispatch.
fn entry_body(emitter: &Emitter<'_>, target: NodeId) -> TokenStream {
    if emitter.is_hot(target) {
        let body = fused_body(emitter, target);
        return quote! {
            let index = at;
            #body
        };
    }
    let shape = emitter.shape(target);
    let name = name(target);
    let input = shape.input.then(|| quote!(input,));
    let carry = shape.resume.then(|| quote!(&mut resume,));
    quote! {
        match #name(#input at, at, state, #carry) {
            ::core::option::Option::Some(found) => return found,
            ::core::option::Option::None => continue 'matcher,
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
