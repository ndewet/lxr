//! Language-preservation tests for the automaton construction pipeline.

use super::{
    dfa::{self, Dfa},
    encoding::{ByteRange, Utf8},
    nfa::{self, Nfa},
};
use crate::regex::Expression;
use std::collections::BTreeSet;
use std::str::FromStr;

struct Case {
    pattern: &'static str,
    alphabet: &'static [char],
    maximum_length: usize,
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            pattern: "([a-c]|é){1,3}(d|x)*",
            alphabet: &['a', 'b', 'c', 'd', 'x', 'é', '\n'],
            maximum_length: 4,
        },
        // The repeated expression can match the empty string, so an NFA must
        // terminate epsilon closure correctly while preserving the `b` suffix.
        Case {
            pattern: "(a?)*b",
            alphabet: &['a', 'b', 'x'],
            maximum_length: 5,
        },
        // The two alternatives share a prefix and can both reach acceptance.
        Case {
            pattern: "(a|ab)*b?",
            alphabet: &['a', 'b', 'x'],
            maximum_length: 5,
        },
        // UTF-8 changes length at this boundary.
        Case {
            pattern: r"[\x7f-\x{80}]",
            alphabet: &['a', '\u{7f}', '\u{80}', 'é'],
            maximum_length: 2,
        },
        Case {
            pattern: r"[^a]",
            alphabet: &['a', '\n', 'é', '𐐷'],
            maximum_length: 2,
        },
        Case {
            pattern: "(é|€|𐐷)+",
            alphabet: &['a', 'é', '€', '𐐷'],
            maximum_length: 3,
        },
        Case {
            pattern: "a{2,}b",
            alphabet: &['a', 'b', 'x'],
            maximum_length: 5,
        },
    ]
}

fn nfa_for(expression: &Expression) -> Nfa<ByteRange> {
    let mut builder = nfa::Builder::new();
    let fragment = nfa::thompson::fragment(expression, &Utf8, &mut builder);
    builder.mark_accept(fragment.exit());
    builder
        .build(&[fragment.entry()])
        .expect("the test NFA is below the builder capacity")
}

fn accepts_nfa(nfa: &Nfa<ByteRange>, input: &[u8]) -> bool {
    let mut execution = nfa.execution();
    execution.restart(0);
    for &byte in input {
        execution.step(byte);
    }
    execution.accepts()
}

fn accepts_dfa(dfa: &Dfa<ByteRange>, input: &[u8]) -> bool {
    let mut state = dfa.start_state(0);
    for &byte in input {
        let Some(next) = dfa.step(state, byte) else {
            return false;
        };
        state = next;
    }
    dfa.accepts(state)
}

fn accepts_expression(expression: &Expression, input: &str) -> bool {
    matched_ends(expression, &input.chars().collect::<Vec<_>>(), 0).contains(&input.chars().count())
}

fn matched_ends(expression: &Expression, input: &[char], start: usize) -> BTreeSet<usize> {
    match expression {
        Expression::Epsilon => [start].into(),
        Expression::Class(set) => input
            .get(start)
            .filter(|character| {
                set.ranges()
                    .any(|(low, high)| (low..=high).contains(character))
            })
            .map(|_| [start + 1].into())
            .unwrap_or_default(),
        Expression::Concatenation(parts) => parts.iter().fold([start].into(), |ends, part| {
            ends.into_iter()
                .flat_map(|end| matched_ends(part, input, end))
                .collect()
        }),
        Expression::Alternation(branches) => branches
            .iter()
            .flat_map(|branch| matched_ends(branch, input, start))
            .collect(),
        Expression::Repetition(inner, quantifier) => {
            let maximum = quantifier
                .maximum()
                .unwrap_or(input.len().saturating_add(quantifier.minimum()));
            let mut ends = BTreeSet::new();
            let mut frontier = BTreeSet::from([start]);

            for count in 0..=maximum {
                if count >= quantifier.minimum() {
                    ends.extend(&frontier);
                }
                frontier = frontier
                    .into_iter()
                    .flat_map(|end| matched_ends(inner, input, end))
                    .collect();
            }
            ends
        }
    }
}

fn inputs(case: &Case) -> Vec<String> {
    let mut inputs = vec![String::new()];
    for _ in 0..case.maximum_length {
        let next: Vec<_> = inputs
            .iter()
            .flat_map(|prefix| {
                case.alphabet.iter().map(move |&character| {
                    let mut input = prefix.clone();
                    input.push(character);
                    input
                })
            })
            .collect();
        inputs.extend(next);
    }
    inputs
}

const MALFORMED_UTF8: &[&[u8]] = &[
    &[0x80],
    &[0xC0, 0x80],
    &[0xC3],
    &[0xC3, 0xC3],
    &[0xED, 0xA0, 0x80],
    &[0xF5, 0x80, 0x80, 0x80],
];

#[test]
fn a_minimized_dfa_accepts_the_same_language_as_its_non_minimized_dfa() {
    for case in cases() {
        let expression = Expression::from_str(case.pattern).expect("the test pattern is valid");
        let nfa = nfa_for(&expression);
        let dfa = dfa::subset::construct(&nfa, |_| ()).expect("the test DFA is below capacity");
        let inputs = inputs(&case);
        let expected: Vec<_> = inputs
            .iter()
            .map(|input| accepts_dfa(&dfa, input.as_bytes()))
            .collect();
        let malformed_expected: Vec<_> = MALFORMED_UTF8
            .iter()
            .map(|input| accepts_dfa(&dfa, input))
            .collect();
        let minimized = dfa.minimize().expect("the minimized DFA is below capacity");

        for (input, expected) in inputs.iter().zip(expected) {
            assert_eq!(
                accepts_dfa(&minimized, input.as_bytes()),
                expected,
                "minimization changed acceptance of {input:?} for {:?}",
                case.pattern,
            );
        }
        for (input, expected) in MALFORMED_UTF8.iter().zip(malformed_expected) {
            assert_eq!(
                accepts_dfa(&minimized, input),
                expected,
                "minimization changed acceptance of malformed UTF-8 {input:?} for {:?}",
                case.pattern,
            );
        }
    }
}

#[test]
fn a_non_minimized_dfa_accepts_the_same_language_as_its_nfa() {
    for case in cases() {
        let expression = Expression::from_str(case.pattern).expect("the test pattern is valid");
        let nfa = nfa_for(&expression);
        let dfa = dfa::subset::construct(&nfa, |_| ()).expect("the test DFA is below capacity");

        for input in inputs(&case) {
            assert_eq!(
                accepts_dfa(&dfa, input.as_bytes()),
                accepts_nfa(&nfa, input.as_bytes()),
                "subset construction changed acceptance of {input:?} for {:?}",
                case.pattern,
            );
        }
        for input in MALFORMED_UTF8 {
            assert_eq!(
                accepts_dfa(&dfa, input),
                accepts_nfa(&nfa, input),
                "subset construction changed acceptance of malformed UTF-8 {input:?} for {:?}",
                case.pattern,
            );
        }
        for input in MALFORMED_UTF8 {
            assert!(
                !accepts_nfa(&nfa, input),
                "Thompson construction accepted malformed UTF-8 {input:?} for {:?}",
                case.pattern,
            );
        }
    }
}

#[test]
fn an_nfa_accepts_the_same_language_as_its_regular_expression() {
    for case in cases() {
        let expression = Expression::from_str(case.pattern).expect("the test pattern is valid");
        let nfa = nfa_for(&expression);

        for input in inputs(&case) {
            assert_eq!(
                accepts_nfa(&nfa, input.as_bytes()),
                accepts_expression(&expression, &input),
                "Thompson construction changed acceptance of {input:?} for {:?}",
                case.pattern,
            );
        }
    }
}
