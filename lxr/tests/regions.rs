//! Scans a region that no rule ends, and that the scan reads again at each start position.
//!
//! A rule that needs a byte far ahead keeps the automaton alive to that byte. The scan then reads
//! the region again at each start position. The runtime records the states that gave no accept,
//! thus the region costs its length. Without that record, each test of this file needs minutes.

mod common;

use common::{Step, steps};
use lxr::Lexer;

/// The number of the bytes of each region of this test.
const REGION: usize = 256 * 1024;

/// A lexer of a rule that ends a run, and of a rule of one byte.
///
/// `Many` needs a `b` that ends the run. `One` matches the first `a` of the run, thus a run with no
/// `b` gives one token for each byte.
#[derive(Debug, PartialEq, Eq, Lexer)]
enum Token {
    #[lxr(regex = "a")]
    One,
    #[lxr(regex = "a+b")]
    Many,
}

/// A lexer of the rule that ends a run alone. A run with no `b` gives one fault for each byte.
#[derive(Debug, PartialEq, Eq, Lexer)]
enum Ended {
    #[lxr(regex = "a+b")]
    Many,
}

#[test]
fn a_run_that_no_rule_ends_gives_one_token_for_each_byte() {
    let input = "a".repeat(REGION);

    let found = steps::<Token>(&input);

    assert_eq!(found.len(), REGION);
    assert_eq!(found[0], Step::Token(Token::One, 0..1));
    assert_eq!(
        found[REGION - 1],
        Step::Token(Token::One, REGION - 1..REGION)
    );
}

#[test]
fn a_run_that_a_rule_ends_gives_one_token_of_the_whole_run() {
    let mut input = "a".repeat(REGION);
    input.push('b');

    assert_eq!(
        steps::<Token>(&input),
        vec![Step::Token(Token::Many, 0..REGION + 1)]
    );
}

#[test]
fn a_run_that_no_rule_matches_gives_one_fault_for_each_character() {
    let input = "a".repeat(REGION);

    let found = steps::<Ended>(&input);

    assert_eq!(found.len(), REGION);
    assert_eq!(found[0], Step::Fault(0..1));
    assert_eq!(found[REGION - 1], Step::Fault(REGION - 1..REGION));
}

#[test]
fn a_run_after_a_region_gives_its_own_token() {
    let input = format!("{}b", "a".repeat(REGION));
    let input = format!("{input}{input}");

    let found = steps::<Token>(&input);

    assert_eq!(
        found,
        vec![
            Step::Token(Token::Many, 0..REGION + 1),
            Step::Token(Token::Many, REGION + 1..2 * REGION + 2),
        ]
    );
}
