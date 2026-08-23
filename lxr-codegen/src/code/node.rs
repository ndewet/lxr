use proc_macro2::{Ident, Literal, TokenStream};
use quote::{format_ident, quote};

use super::emitter::{Emitter, LARGE_GRAPH};
use super::pattern::{covers, patterns, test};
use crate::compiler::ByteRange;
use crate::graph::{Carry, Edge, Fork, Leaf, Node, NodeId, Rope};

/// Returns the name of the function of the node that `id` refers to.
pub fn name(id: NodeId) -> Ident {
    format_ident!("node_{}", id.index())
}

/// Returns the function of the node that `id` refers to.
///
/// The function takes `#[inline(always)]`. A node holds few instructions, and the compiler leaves
/// ten of them as a call in the lexer of JSON without it. One call for each node costs 30 percent
/// of the scan. The graph holds no cycle of calls, because each cycle carries one edge that writes
/// the node and returns, thus the attribute terminates.
///
/// The top-level `step` function is inlined with the compact result protocol. This lets the
/// caller remove result plumbing while this attribute removes calls between hot nodes.
///
/// [`Scan::next`]: https://docs.rs/lxr/latest/lxr/struct.Scan.html
///
/// # Panics
///
/// This function panics if `id` is not a node of the graph of `emitter`, or if the lexer holds no
/// rule that a leaf of the graph gives.
pub fn function(emitter: &Emitter<'_>, id: NodeId) -> TokenStream {
    let shape = emitter.shape(id);
    let token = emitter.token;
    let name = name(id);
    let input = parameter(shape.input, quote!(input: &str));
    let marker = parameter(shape.marker, quote!(marker: usize));
    let resume = parameter(shape.resume, quote!(resume: &mut Resume));
    let body = match emitter.arena.node(id) {
        Node::Fork(fork) => fork_body(emitter, id, fork),
        Node::Rope(rope) => rope_body(emitter, rope),
        Node::Leaf(leaf) => leaf_body(emitter, *leaf),
        Node::Fault => quote! {
            *found = ::core::option::Option::Some(::lxr::Match::None);
        },
    };
    let attributes = if matches!(emitter.arena.node(id), Node::Fault) {
        quote!(#[cold] #[inline(never)])
    } else if emitter.arena.node_count() > LARGE_GRAPH {
        quote!(#[inline])
    } else {
        quote!(#[inline(always)])
    };

    quote! {
        #attributes
        fn #name(
            #input
            at: usize,
            index: usize,
            #marker
            state: &mut <#token as ::lxr::Lexer>::State,
            found: &mut ::core::option::Option<::lxr::Match<#token>>,
            #resume
        ) {
            #body
        }
    }
}

/// Returns `parameter` if `takes` is true, and nothing if it is not.
fn parameter(takes: bool, parameter: TokenStream) -> TokenStream {
    if takes {
        quote!(#parameter,)
    } else {
        TokenStream::new()
    }
}

/// Returns the body of a fork.
///
/// The body holds two parts. An arm that gives this same fork reads a run of bytes in one loop,
/// thus the fork leaves that arm one time for the whole run. The dispatch then reads the byte that
/// stopped the run, and it goes to the node of that byte.
fn fork_body(emitter: &Emitter<'_>, id: NodeId, fork: &Fork) -> TokenStream {
    let run = fork
        .arms
        .iter()
        .find(|arm| arm.edge.target == id)
        .map(|arm| run(emitter, id, &arm.ranges, fork.arms.len() == 1))
        .unwrap_or_default();

    let miss = action(emitter, fork.miss, &quote!(index));
    let arms = fork
        .arms
        .iter()
        .filter(|arm| arm.edge.target != id)
        .map(|arm| {
            let patterns = patterns(&arm.ranges);
            let action = action(emitter, arm.edge, &quote!(index + 1));
            quote!(#patterns => { #action })
        });
    let ranges: Vec<ByteRange> = fork
        .arms
        .iter()
        .filter(|arm| arm.edge.target != id)
        .flat_map(|arm| arm.ranges.iter().copied())
        .collect();

    if emitter.arena.node_count() > LARGE_GRAPH && arms_count(fork, id) > 2 {
        return table_fork(emitter, id, fork, run);
    }

    if ranges.is_empty() {
        return quote! {
            let bytes = input.as_bytes();
            #run
            #miss
        };
    }

    let rest = if covers(&ranges) {
        TokenStream::new()
    } else {
        quote!(_ => { #miss })
    };

    quote! {
        let bytes = input.as_bytes();
        #run
        let ::core::option::Option::Some(&byte) = bytes.get(index) else {
            #miss
            return;
        };
        match byte {
            #(#arms)*
            #rest
        }
    }
}

/// Returns the table dispatch of a wide fork in a large graph.
fn table_fork(emitter: &Emitter<'_>, id: NodeId, fork: &Fork, run: TokenStream) -> TokenStream {
    let arms: Vec<&crate::graph::Arm> = fork
        .arms
        .iter()
        .filter(|arm| arm.edge.target != id)
        .collect();
    assert!(arms.len() < 256, "a byte table holds at most 255 arms");

    let mut table = [0_u8; 256];
    for (index, arm) in arms.iter().enumerate() {
        let value = u8::try_from(index + 1).expect("a byte table holds at most 255 arms");
        for range in &arm.ranges {
            for byte in range.low..=range.high {
                table[usize::from(byte)] = value;
            }
        }
    }
    let table = table.iter().map(|&value| Literal::u8_unsuffixed(value));
    let choices = arms.iter().enumerate().map(|(index, arm)| {
        let value = Literal::u8_unsuffixed(
            u8::try_from(index + 1).expect("a byte table holds at most 255 arms"),
        );
        let action = action(emitter, arm.edge, &quote!(index + 1));
        quote!(#value => { #action })
    });
    let miss = action(emitter, fork.miss, &quote!(index));
    let name = format_ident!("FORK_TABLE_{}", id.index());

    quote! {
        const #name: [u8; 256] = [#(#table),*];
        let bytes = input.as_bytes();
        #run
        let ::core::option::Option::Some(&byte) = bytes.get(index) else {
            #miss
            return;
        };
        match #name[byte as usize] {
            #(#choices)*
            _ => { #miss }
        }
    }
}

/// Returns the number of the arms that do not read a run.
fn arms_count(fork: &Fork, id: NodeId) -> usize {
    fork.arms.iter().filter(|arm| arm.edge.target != id).count()
}

/// Returns the loop that reads the run of the node at `id`, which holds the bytes of `ranges`.
///
/// A node that ends a match reads its run one time, because the scan goes on after that match. A
/// node that ends no match keeps the run that it read. The scan reads a region that no rule ends
/// again at each start position, thus a later step reaches this node inside a run that it already
/// read. The record turns that read into one comparison.
///
/// `only` states that the run is the one arm of the fork. Such a node reads a run and nothing
/// else, thus [`block`] reads that run in blocks. A node of more arms reads one byte at a time,
/// because a block there costs each of those arms.
fn run(emitter: &Emitter<'_>, id: NodeId, ranges: &[ByteRange], only: bool) -> TokenStream {
    let test = emitter.run_test(id).unwrap_or_else(|| test(ranges));
    let bytes_loop = quote! {
        while let ::core::option::Option::Some(&byte) = bytes.get(index) {
            if #test {
                index += 1;
            } else {
                break;
            }
        }
    };
    let loops = if only {
        block(id, &test, &bytes_loop)
    } else {
        bytes_loop
    };

    if emitter.ends_a_match(id) {
        return quote! {
            let mut index = index;
            #loops
        };
    }

    let number = Literal::u32_unsuffixed(id.number());
    let record = emitter.run();
    quote! {
        let mut index = index;
        if #record.node == #number && index >= #record.low && index <= #record.high {
            index = #record.high;
        } else {
            let low = index;
            #loops
            #record.node = #number;
            #record.low = low;
            #record.high = index;
        }
    }
}

/// The number of the bytes that one block of a long run reads.
///
/// A wider block reads a long run faster, and it holds one test for each of its bytes. A block of
/// 32 bytes reads a name of 40000 bytes 12 percent faster than a block of 16, and it reads a
/// document of source code 15 percent slower.
const BLOCK: usize = 16;

/// Returns the loops that read a run of bytes in blocks.
///
/// The first loop reads [`BLOCK`] bytes one at a time. A run that stops inside that loop is a
/// short run, and it costs one comparison more than a plain loop. A run that fills the loop goes
/// on in a function that reads one block at a time.
///
/// That function takes `#[inline(never)]`. The tests of one block are the code of [`BLOCK`] bytes,
/// and each node of the graph shares the registers of the step. A block inside the step costs 40
/// percent of the scan of a document of source code, and a block behind a call costs nothing.
fn block(id: NodeId, test: &TokenStream, bytes_loop: &TokenStream) -> TokenStream {
    let count = Literal::usize_unsuffixed(BLOCK);
    let checks = (0..BLOCK).map(|offset| {
        let offset = Literal::usize_unsuffixed(offset);
        quote!({
            let byte = block[#offset];
            #test
        })
    });
    let long = format_ident!("long_{}", id.index());

    quote! {
        #[inline(never)]
        fn #long(bytes: &[u8], index: usize) -> usize {
            let mut index = index;
            while let ::core::option::Option::Some(block) =
                bytes[index..].first_chunk::<#count>()
            {
                if !(#(#checks)&*) {
                    break;
                }
                index += #count;
            }
            #bytes_loop
            index
        }

        let bound = ::core::cmp::min(index + #count, bytes.len());
        while index < bound {
            let byte = bytes[index];
            if #test {
                index += 1;
            } else {
                break;
            }
        }
        if index == bound && bound < bytes.len() {
            index = #long(bytes, index);
        }
    }
}

/// Returns the body of a rope.
fn rope_body(emitter: &Emitter<'_>, rope: &Rope) -> TokenStream {
    let count = Literal::usize_unsuffixed(rope.bytes.len());
    let bytes = Literal::byte_string(&rope.bytes);
    let then = action(emitter, rope.then, &quote!(index + #count));
    let miss = action(emitter, rope.miss, &quote!(index));

    quote! {
        let bytes = input.as_bytes();
        if bytes[index..].first_chunk::<#count>() == ::core::option::Option::Some(#bytes) {
            #then
        } else {
            #miss
        }
    }
}

/// Returns the body of a leaf.
///
/// # Panics
///
/// This function panics if the lexer holds no rule at the rule of `leaf`.
fn leaf_body(emitter: &Emitter<'_>, leaf: Leaf) -> TokenStream {
    let token = emitter.token;
    let rule = emitter.rule(leaf.rule);
    let length = if leaf.here {
        quote!(index - at)
    } else {
        quote!(marker - at)
    };
    let go = rule.go.map(|condition| {
        let condition = Literal::u16_unsuffixed(condition);
        let place = emitter.condition_place();
        quote!(#place = #condition;)
    });

    let outcome = match (&rule.token, &rule.value) {
        (None, _) => quote!(::lxr::Match::Skip(length)),
        (Some(variant), None) => {
            quote!(::lxr::Match::Token(#token::#variant, length))
        }
        (Some(variant), Some(value)) => quote! {
            match <#value as ::core::str::FromStr>::from_str(
                &input[at..at + length]
            ) {
                ::core::result::Result::Ok(value) => {
                    ::lxr::Match::Token(#token::#variant(value), length)
                }
                ::core::result::Result::Err(_) => ::lxr::Match::Value(length),
            }
        },
    };

    quote! {
        let length = #length;
        *found = ::core::option::Option::Some(#outcome);
        #go
    }
}

/// Returns the statements that one edge of the graph gives.
///
/// An edge that closes no cycle calls the function of its node, and a call at the end of a
/// function becomes a jump. An edge that closes a cycle writes the node and the offsets, then it
/// returns to the driver of the step. Thus the depth of the stack is the length of the longest
/// path of the graph that holds no cycle.
///
/// # Panics
///
/// This function panics if the node of `edge` is not a node of the graph, or if a node that takes
/// a marker gets none from `edge`.
pub fn action(emitter: &Emitter<'_>, edge: Edge, index: &TokenStream) -> TokenStream {
    if edge.back {
        let number = Literal::u32_unsuffixed(edge.target.number() + 1);
        let marker = emitter.carries().then(|| {
            let marker = carried(edge.carry).unwrap_or_else(|| quote!(0));
            quote!(resume.marker = #marker;)
        });
        return quote! {
            resume.node = #number;
            resume.index = #index;
            #marker
        };
    }

    let shape = emitter.shape(edge.target);
    let name = name(edge.target);
    let input = shape.input.then(|| quote!(input,));
    let marker = shape.marker.then(|| {
        let marker = carried(edge.carry)
            .expect("a node that takes a marker gets one from each edge into it");
        quote!(#marker,)
    });
    let resume = shape.resume.then(|| quote!(resume,));

    quote!(#name(#input at, #index, #marker state, found, #resume);)
}

/// Returns the offset of the last accept that `carry` gives, or `None` if it gives none.
fn carried(carry: Carry) -> Option<TokenStream> {
    match carry {
        Carry::Nothing => None,
        Carry::Ahead(0) => Some(quote!(index)),
        Carry::Ahead(ahead) => {
            let ahead = Literal::usize_unsuffixed(ahead);
            Some(quote!(index + #ahead))
        }
        Carry::Marker => Some(quote!(marker)),
    }
}
