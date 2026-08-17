//! Asserts that a derived lexer accepts the expected language, and no other language.
//!
//! A test of a few inputs shows that a lexer reads those inputs. It does not show that the lexer
//! reads nothing else. These tests read every string of a small alphabet up to a length, and they
//! compare each one against a matcher that this file holds.
//!
//! The matcher reads the rules one at a time with plain string operations. It holds no automaton,
//! thus it agrees with the lexer only if the whole pipeline is correct: the parser, the
//! construction, the determinization, the tables, the emitted source, and the scan.
//!
//! A comparison covers the token and the bytes that it reads. Thus it covers the language, the
//! longest match, the precedence, and the place of each fault together.

mod common;

use common::{Step, best, character, leading, steps, strings};
use lxr::Lexer;

/// A lexer of three rules that overlap, and one rule that skips.
///
/// `Ab` and `As` both match at `ab`, thus the longest match decides. `Ab` and `As` both match one
/// `a` at `a`, and `Ab` needs a `b`, thus `As` wins there.
///
/// The rules are at these indexes of the precedence: `Ab` 0, `As` 1, `Bs` 2, and the rule that
/// skips 3. A rule of a variant comes before a rule that skips.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Lexer)]
#[lxr(skip = " +")]
enum Small {
    #[lxr(token = "ab")]
    Ab,
    #[lxr(regex = "a+")]
    As,
    #[lxr(regex = "b+")]
    Bs,
}

/// Reads `input` with the rules of [`Small`], and gives what the lexer must give.
fn small(input: &str) -> Vec<Step<Small>> {
    let mut at = 0;
    let mut found = Vec::new();

    while at < input.len() {
        let rest = &input[at..];
        let candidates = [
            (usize::from(rest.starts_with("ab")) * 2, 0),
            (leading(rest, |byte| byte == b'a'), 1),
            (leading(rest, |byte| byte == b'b'), 2),
            (leading(rest, |byte| byte == b' '), 3),
        ];

        match best(&candidates) {
            Some((length, rule)) => {
                let span = at..at + length;
                match rule {
                    0 => found.push(Step::Token(Small::Ab, span)),
                    1 => found.push(Step::Token(Small::As, span)),
                    2 => found.push(Step::Token(Small::Bs, span)),
                    _ => {}
                }
                at += length;
            }
            None => {
                let length = character(rest);
                found.push(Step::Fault(at..at + length));
                at += length;
            }
        }
    }

    found
}

/// The start conditions of [`Pair`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Where {
    Code,
    Text,
}

/// A lexer of two start conditions. A quote opens a string, and a quote inside it closes the
/// string.
///
/// The rules are at these indexes of the precedence: `Open` 0, `Word` 1, `Body` 2, and `Close` 3.
/// `Open` and `Word` are applicable under `Code`, and `Body` and `Close` under `Text`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Lexer)]
#[lxr(condition = Where::Code)]
enum Pair {
    #[lxr(token = "\"", go = Where::Text)]
    Open,
    #[lxr(regex = "[ab]+")]
    Word,
    #[lxr(regex = "[^\"]+", in = [Where::Text])]
    Body,
    #[lxr(token = "\"", in = [Where::Text], go = Where::Code)]
    Close,
}

/// Reads `input` with the rules of [`Pair`], and gives what the lexer must give.
///
/// The matcher holds the start condition. A rule changes it after the match, thus the next token
/// reads the rules of the new condition.
fn pair(input: &str) -> Vec<Step<Pair>> {
    let mut at = 0;
    let mut inside = false;
    let mut found = Vec::new();

    while at < input.len() {
        let rest = &input[at..];
        let candidates = if inside {
            [
                (leading(rest, |byte| byte != b'"'), 2),
                (usize::from(rest.starts_with('"')), 3),
            ]
        } else {
            [
                (usize::from(rest.starts_with('"')), 0),
                (leading(rest, |byte| byte == b'a' || byte == b'b'), 1),
            ]
        };

        match best(&candidates) {
            Some((length, rule)) => {
                let span = at..at + length;
                match rule {
                    0 => {
                        found.push(Step::Token(Pair::Open, span));
                        inside = true;
                    }
                    1 => found.push(Step::Token(Pair::Word, span)),
                    2 => found.push(Step::Token(Pair::Body, span)),
                    _ => {
                        found.push(Step::Token(Pair::Close, span));
                        inside = false;
                    }
                }
                at += length;
            }
            None => {
                let length = character(rest);
                found.push(Step::Fault(at..at + length));
                at += length;
            }
        }
    }

    found
}

#[test]
fn the_lexer_of_one_condition_reads_the_expected_language_and_no_other() {
    let inputs = strings(&['a', 'b', ' ', 'z'], 5);
    assert_eq!(inputs.len(), 1365);

    for input in &inputs {
        assert_eq!(
            steps::<Small>(input),
            small(input),
            "the lexer disagrees on {input:?}"
        );
    }
}

#[test]
fn the_lexer_of_two_conditions_reads_the_expected_language_and_no_other() {
    let inputs = strings(&['a', '"', 'z', 'é'], 4);
    assert_eq!(inputs.len(), 341);

    for input in &inputs {
        assert_eq!(
            steps::<Pair>(input),
            pair(input),
            "the lexer disagrees on {input:?}"
        );
    }
}

#[test]
fn the_matcher_and_the_lexer_agree_on_a_longer_input() {
    let inputs = [
        "aaabbb aa b",
        "abababab",
        "  ab  ba  ",
        "zzz",
        "a z b",
        "abba",
        "aaaaaaaaaaab",
    ];

    for input in inputs {
        assert_eq!(steps::<Small>(input), small(input), "input {input:?}");
    }
}

#[test]
fn the_matcher_and_the_lexer_agree_on_a_longer_string() {
    let inputs = [
        "ab\"a quoted body\"ba",
        "\"\"",
        "\"unterminated",
        "\"a\"\"b\"",
        "z\"z\"z",
        "\"é\"a",
    ];

    for input in inputs {
        assert_eq!(steps::<Pair>(input), pair(input), "input {input:?}");
    }
}

#[test]
fn the_matcher_gives_the_result_that_the_rules_state() {
    assert_eq!(
        small("ab"),
        vec![Step::Token(Small::Ab, 0..2)],
        "the literal wins the tie at two bytes"
    );
    assert_eq!(
        small("a"),
        vec![Step::Token(Small::As, 0..1)],
        "the literal needs a b, thus the repetition wins"
    );
    assert_eq!(
        small("aab"),
        vec![Step::Token(Small::As, 0..2), Step::Token(Small::Bs, 2..3)],
        "the longest match reads two letters a, thus no rule reads ab"
    );
    assert_eq!(small(" "), vec![], "a rule that skips gives no token");
    assert_eq!(small("z"), vec![Step::Fault(0..1)], "no rule reads a z");
}
