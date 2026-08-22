use proc_macro2::{Literal, TokenStream};
use quote::quote;

use super::state::state;
use crate::automata::{Automaton, DeterministicFiniteAutomaton, StateId};
use crate::compiler::{Accepts, ByteRange};

/// The maximum number of the states of an automaton that this module writes as code.
///
/// One state gives one arm, and one arm holds a comparison for each label of that state. A lexer
/// above this limit thus gives a function that costs more compile time than the scan saves. Such a
/// lexer keeps the scan of the tables, which is the default of the runtime.
pub const MAX_CODE_STATES: usize = 1024;

/// The number of the states below which one arm holds two states after the state of that arm.
///
/// An arm that holds the states after it reads a token of a few bytes with no jump. Each state
/// that it holds is one copy of that state, thus the source of a lexer of many states grows too
/// far and the compiler is too slow. A lexer of 530 states builds in 8.7 seconds at a depth of 2,
/// in 2.1 seconds at a depth of 0, and a lexer of 193 states builds in 1.3 seconds at a depth of
/// 2.
const DEEP_STATES: usize = 128;

/// The number of the states below which one arm holds one state after the state of that arm.
const SHALLOW_STATES: usize = 512;

/// Returns the `find` function of `dfa`, in which each state accepts the rule that `accepts`
/// gives.
///
/// The result is `None` if `dfa` holds more than [`MAX_CODE_STATES`] states.
///
/// The function keeps the state in the program counter, and not in a table. One arm holds one
/// state, one comparison holds one label, and the state that a byte gives is a constant. Thus a
/// step waits for no read of memory.
///
/// The scan keeps the last accept that it reached. Thus the longest match wins, and the earliest
/// rule wins a tie, exactly as the scan of the tables gives them.
///
/// # Panics
///
/// This function panics if a start state of `dfa` accepts. Such a state means that a rule matches
/// the empty string, and the lexicon rejects such a rule.
pub fn find(
    dfa: &DeterministicFiniteAutomaton<ByteRange>,
    accepts: &Accepts<u16>,
) -> Option<TokenStream> {
    with_limit(dfa, accepts, MAX_CODE_STATES)
}

/// Returns the `find` function of `dfa` inside `states` states.
///
/// [`find`](find()) gives [`MAX_CODE_STATES`]. A test gives a lower limit to reach the lexer that
/// gets no code.
///
/// # Panics
///
/// This function panics if a start state of `dfa` accepts.
fn with_limit(
    dfa: &DeterministicFiniteAutomaton<ByteRange>,
    accepts: &Accepts<u16>,
    states: usize,
) -> Option<TokenStream> {
    if dfa.state_count() > states {
        return None;
    }

    assert!(
        dfa.start_states()
            .iter()
            .all(|&start| accepts.get(start).is_none()),
        "a start state of the lexer accepts, thus a rule matches the empty string"
    );

    let conditions = (0..dfa.start_count()).map(Literal::usize_unsuffixed);
    let starts = dfa
        .start_states()
        .iter()
        .map(|start| Literal::usize_unsuffixed(start.index()));
    let depth = depth(dfa.state_count());
    let arms = (0..dfa.state_count()).map(|index| state(dfa, accepts, StateId::new(index), depth));

    Some(quote! {
        fn find(input: &[u8], at: usize, condition: u16) -> ::lxr::Matched {
            let mut state: u32 = match condition {
                #(#conditions => #starts,)*
                condition => panic!(
                    "condition {condition} is not a start condition of this lexer"
                ),
            };
            let mut accept: u16 = 0;
            let mut length: usize = 0;
            let mut index: usize = at;

            loop {
                match state {
                    #(#arms)*
                    state => panic!("state {state} is not a state of this lexer"),
                }
            }

            ::lxr::Matched {
                accept,
                length,
                read: index - at,
            }
        }
    })
}

/// Returns the number of the states that one arm holds after the state of that arm.
///
/// A lexer of few states holds two states in each arm, and a lexer of many states holds none. The
/// source grows with the depth, thus the depth falls as the lexer grows.
fn depth(states: usize) -> usize {
    match states {
        0..DEEP_STATES => 2,
        DEEP_STATES..SHALLOW_STATES => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::{Bytes, Lexicon, compile};
    use crate::regex::Node;

    /// Builds the automaton of the rules of `patterns`, and the accept of each of its states.
    ///
    /// Each rule reads under the first start condition, and the accept of a rule is its index.
    fn build(patterns: &[&str]) -> (DeterministicFiniteAutomaton<ByteRange>, Accepts<u16>) {
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
        (determinization.dfa, accepts)
    }

    /// Returns `true` if the text of `source` holds the text of `part`.
    fn holds(source: &str, part: &TokenStream) -> bool {
        source.contains(&part.to_string())
    }

    #[test]
    fn the_function_gives_the_match_of_the_runtime() {
        let (dfa, accepts) = build(&["[a-z]+"]);

        let source = find(&dfa, &accepts).expect("a test is small").to_string();

        assert!(holds(
            &source,
            &quote!(fn find(input: &[u8], at: usize, condition: u16) -> ::lxr::Matched)
        ));
        assert!(holds(
            &source,
            &quote!(::lxr::Matched {
                accept,
                length,
                read: index - at,
            })
        ));
    }

    #[test]
    fn each_start_condition_gives_its_own_state() {
        let mut lexicon = Lexicon::new();
        lexicon.condition();
        let first: Node = "a".parse().expect("the pattern is valid");
        let second: Node = "b".parse().expect("the pattern is valid");
        lexicon.rule(first, 0, &[0]).expect("the rule is valid");
        lexicon.rule(second, 1, &[1]).expect("the rule is valid");
        let (nfa, accepts) = compile(Bytes, lexicon).expect("a test stays below the capacity");
        let determinization = nfa.determinize().expect("a test stays below the capacity");
        let accepts = accepts.determinized(&determinization.subsets);

        let source = find(&determinization.dfa, &accepts)
            .expect("a test is small")
            .to_string();

        assert!(holds(&source, &quote!(0 => 0,)));
        assert!(holds(&source, &quote!(1 => 1,)));
        assert!(holds(
            &source,
            &quote!(condition => panic!(
                "condition {condition} is not a start condition of this lexer"
            ))
        ));
    }

    #[test]
    fn a_lexer_above_the_limit_gets_no_code() {
        let (dfa, accepts) = build(&["[a-z]+"]);
        let states = dfa.state_count();

        assert!(states > 1, "a lexer of one state proves nothing");
        assert!(with_limit(&dfa, &accepts, states - 1).is_none());
        assert!(with_limit(&dfa, &accepts, states).is_some());
    }
}
