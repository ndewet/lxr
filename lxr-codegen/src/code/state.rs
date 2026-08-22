use proc_macro2::{Literal, TokenStream};
use quote::quote;

use super::pattern::{covers, patterns, test};
use crate::automata::{Automaton, DeterministicFiniteAutomaton, StateId};
use crate::compiler::{Accepts, ByteRange};

/// Returns the arm of the state that `id` refers to, which holds `depth` states after it.
///
/// The arm holds three parts, and each one is optional:
///
/// 1. The run. A label that gives the state that it starts from reads a run of bytes, thus the arm
///    reads that run in one loop and it leaves the arm one time for the whole run.
/// 2. The accept. The scan keeps the last accept that it reached, thus a state that accepts writes
///    the rule and the length here. The run comes first, because a longer run gives a longer match
///    of the same rule.
/// 3. The dispatch. The arm reads one byte, then it goes to the state of the label that matches.
///    A state that reads no byte stops the scan instead.
///
/// # Panics
///
/// This function panics if `id` is not a state of `dfa`, or if `accepts` holds no accept for it.
pub fn state(
    dfa: &DeterministicFiniteAutomaton<ByteRange>,
    accepts: &Accepts<u16>,
    id: StateId,
    depth: usize,
) -> TokenStream {
    let number = Literal::usize_unsuffixed(id.index());
    let body = body(dfa, accepts, id, depth);

    quote! {
        #number => {
            #body
        }
    }
}

/// Returns the body of the state that `id` refers to, which holds `depth` states after it.
fn body(
    dfa: &DeterministicFiniteAutomaton<ByteRange>,
    accepts: &Accepts<u16>,
    id: StateId,
    depth: usize,
) -> TokenStream {
    let run = run(dfa, id);
    let accept = accept(accepts, id);
    let dispatch = dispatch(dfa, accepts, id, depth);

    quote! {
        #run
        #accept
        #dispatch
    }
}

/// Returns the loop that reads the run of the state that `id` refers to.
///
/// The result is empty if no label of the state gives that same state.
fn run(dfa: &DeterministicFiniteAutomaton<ByteRange>, id: StateId) -> TokenStream {
    let ranges: Vec<ByteRange> = dfa
        .transitions(id)
        .iter()
        .filter(|transition| transition.target == id)
        .map(|transition| transition.label)
        .collect();

    if ranges.is_empty() {
        return TokenStream::new();
    }

    let test = test(&ranges);
    quote! {
        while let ::core::option::Option::Some(&byte) = input.get(index) {
            if #test {
                index += 1;
            } else {
                break;
            }
        }
    }
}

/// Returns the accept of the state that `id` refers to.
///
/// The result is empty if the state does not accept.
///
/// # Panics
///
/// This function panics if `accepts` holds no accept for `id`.
fn accept(accepts: &Accepts<u16>, id: StateId) -> TokenStream {
    let Some(&rule) = accepts.get(id) else {
        return TokenStream::new();
    };

    let accept = Literal::u16_unsuffixed(rule + 1);
    quote! {
        accept = #accept;
        length = index - at;
    }
}

/// Returns the dispatch of the state that `id` refers to, which holds `depth` states after it.
///
/// Two labels of one state that give the same state share one arm, thus the source holds one arm
/// for each state that this state reaches. The labels of one state match no byte in common, thus
/// the sequence of the arms changes nothing.
///
/// A depth above 0 writes the body of the state that each byte gives, and not the number of that
/// state. The scan then reads the next byte with no jump.
fn dispatch(
    dfa: &DeterministicFiniteAutomaton<ByteRange>,
    accepts: &Accepts<u16>,
    id: StateId,
    depth: usize,
) -> TokenStream {
    let mut targets: Vec<(StateId, Vec<ByteRange>)> = Vec::new();
    for transition in dfa.transitions(id) {
        if transition.target == id {
            continue;
        }
        match targets
            .iter_mut()
            .find(|(target, _)| *target == transition.target)
        {
            Some((_, ranges)) => ranges.push(transition.label),
            None => targets.push((transition.target, vec![transition.label])),
        }
    }

    if targets.is_empty() {
        return quote!(break;);
    }

    let read: Vec<ByteRange> = targets
        .iter()
        .flat_map(|(_, ranges)| ranges.iter().copied())
        .collect();
    let arms = targets.iter().map(|(target, ranges)| {
        let patterns = patterns(ranges);
        if depth == 0 {
            let number = Literal::usize_unsuffixed(target.index());
            return quote! {
                #patterns => {
                    index += 1;
                    state = #number;
                }
            };
        }

        let body = body(dfa, accepts, *target, depth - 1);
        quote! {
            #patterns => {
                index += 1;
                #body
            }
        }
    });
    let rest = if covers(&read) {
        TokenStream::new()
    } else {
        quote!(_ => break,)
    };

    quote! {
        let ::core::option::Option::Some(&byte) = input.get(index) else {
            break;
        };
        match byte {
            #(#arms)*
            #rest
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::{Bytes, Lexicon, compile};
    use crate::regex::Node;

    /// The automaton of a lexer, and the accept of each of its states.
    struct Lexer {
        dfa: DeterministicFiniteAutomaton<ByteRange>,
        accepts: Accepts<u16>,
    }

    /// Builds the automaton of the rules of `patterns` under one start condition.
    fn build(patterns: &[&str]) -> Lexer {
        let mut lexicon = Lexicon::new();
        for (index, pattern) in patterns.iter().enumerate() {
            let pattern: Node = pattern.parse().expect("the pattern is valid");
            let rule = u16::try_from(index).expect("a test holds few rules");
            lexicon
                .rule(pattern, rule, &[0])
                .expect("the rule passes each check");
        }

        let (nfa, accepts) = compile(Bytes, lexicon).expect("a test stays below the capacity");
        let determinization = nfa.determinize().expect("a test stays below the capacity");
        let accepts = accepts.determinized(&determinization.subsets);

        Lexer {
            dfa: determinization.dfa,
            accepts,
        }
    }

    /// Returns the arm of the first state of `lexer` that reads a run.
    fn running(lexer: &Lexer) -> String {
        for index in 0..lexer.dfa.state_count() {
            let id = StateId::new(index);
            if lexer
                .dfa
                .transitions(id)
                .iter()
                .any(|transition| transition.target == id)
            {
                return state(&lexer.dfa, &lexer.accepts, id, 0).to_string();
            }
        }
        panic!("no state of the lexer reads a run");
    }

    /// Returns `true` if the text of `source` holds the text of `part`.
    fn holds(source: &str, part: &TokenStream) -> bool {
        source.contains(&part.to_string())
    }

    #[test]
    fn the_state_of_a_run_reads_that_run_in_one_loop() {
        let lexer = build(&["[a-z]+"]);

        let arm = running(&lexer);

        assert!(
            holds(&arm, &quote!(byte.wrapping_sub(97u8) <= 25u8)),
            "{arm}"
        );
        assert!(holds(&arm, &quote!(index += 1;)), "{arm}");
    }

    #[test]
    fn an_arm_of_a_depth_above_zero_holds_the_state_that_a_byte_gives() {
        let lexer = build(&["ab"]);
        let start = lexer.dfa.start_state(0);
        let target = lexer.dfa.transitions(start)[0].target;
        let number = Literal::usize_unsuffixed(target.index());

        let deep = state(&lexer.dfa, &lexer.accepts, start, 1).to_string();
        let flat = state(&lexer.dfa, &lexer.accepts, start, 0).to_string();

        assert!(holds(&flat, &quote!(state = #number;)));
        assert!(!holds(&deep, &quote!(state = #number;)));
        assert!(holds(&deep, &quote!(98u8 =>)), "{deep}");
    }

    #[test]
    fn a_state_that_accepts_writes_the_rule_and_the_length() {
        let lexer = build(&["a"]);
        let start = lexer.dfa.start_state(0);
        let target = lexer.dfa.transitions(start)[0].target;

        let arm = state(&lexer.dfa, &lexer.accepts, target, 0).to_string();

        assert!(holds(&arm, &quote!(accept = 1;)));
        assert!(holds(&arm, &quote!(length = index - at;)));
    }

    #[test]
    fn a_state_that_reads_no_byte_stops_the_scan() {
        let lexer = build(&["a"]);
        let start = lexer.dfa.start_state(0);
        let target = lexer.dfa.transitions(start)[0].target;

        let arm = state(&lexer.dfa, &lexer.accepts, target, 0).to_string();

        assert!(holds(&arm, &quote!(break;)));
        assert!(!holds(&arm, &quote!(input.get(index))));
    }

    #[test]
    fn a_state_that_reads_a_byte_goes_to_the_state_of_that_byte() {
        let lexer = build(&["a"]);
        let start = lexer.dfa.start_state(0);
        let target = lexer.dfa.transitions(start)[0].target;
        let number = Literal::usize_unsuffixed(target.index());

        let arm = state(&lexer.dfa, &lexer.accepts, start, 0).to_string();

        assert!(holds(
            &arm,
            &quote!(97u8 => { index += 1; state = #number; })
        ));
        assert!(holds(&arm, &quote!(_ => break,)));
    }

    #[test]
    fn two_bytes_that_give_one_state_share_one_arm() {
        let lexer = build(&["[ac]b"]);
        let start = lexer.dfa.start_state(0);

        let arm = state(&lexer.dfa, &lexer.accepts, start, 0).to_string();

        assert!(arm.contains(&quote!(97u8 | 99u8 =>).to_string()), "{arm}");
    }
}
