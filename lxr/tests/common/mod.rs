//! The parts that each regression test shares.
//!
//! Cargo compiles this module into each test binary that declares it. A binary that uses part of
//! it leaves the remainder unused, thus the module allows dead code.

#![allow(dead_code)]

use lxr::Lexer;
use std::ops::Range;

/// One result of a scan, with the bytes that it covers.
///
/// A scan gives a token or a fault. The span states which bytes the scan read, thus a comparison
/// of two scans covers the language and the division of the input together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step<T> {
    /// The scan read a token.
    Token(T, Range<usize>),
    /// No rule matched the character at this place.
    Fault(Range<usize>),
}

/// Returns each token and each fault of a scan of `input`.
pub fn steps<T: Lexer>(input: &str) -> Vec<Step<T>> {
    T::scan(input)
        .located()
        .map(|result| match result {
            Ok(found) => Step::Token(found.token, found.span),
            Err(error) => Step::Fault(error.span()),
        })
        .collect()
}

/// Returns the tokens of a scan of `input`.
///
/// # Panics
///
/// This function panics if a character of `input` matches no rule.
pub fn tokens<T: Lexer>(input: &str) -> Vec<T> {
    T::scan(input)
        .map(|found| found.expect("each character of the input belongs to a token"))
        .collect()
}

/// Returns `true` if each character of `input` belongs to a token or to a rule that skips.
pub fn accepts<T: Lexer>(input: &str) -> bool {
    T::scan(input).all(|found| found.is_ok())
}

/// Returns each string of `alphabet` of a length up to `length`, the empty string included.
pub fn strings(alphabet: &[char], length: usize) -> Vec<String> {
    let mut all = vec![String::new()];
    let mut level = vec![String::new()];

    for _ in 0..length {
        let mut next = Vec::new();
        for base in &level {
            for &character in alphabet {
                let mut word = base.clone();
                word.push(character);
                next.push(word);
            }
        }
        all.extend(next.iter().cloned());
        level = next;
    }

    all
}

/// Returns the number of the leading bytes of `input` that `holds` accepts.
pub fn leading(input: &str, holds: impl Fn(u8) -> bool) -> usize {
    input.bytes().take_while(|&byte| holds(byte)).count()
}

/// Returns the longest match of `candidates`, and the earliest rule of the longest.
///
/// Each candidate is a length and the index of the rule that gives it. A length of 0 means that
/// the rule does not match. The result is `None` if no rule matches.
pub fn best(candidates: &[(usize, usize)]) -> Option<(usize, usize)> {
    candidates
        .iter()
        .copied()
        .filter(|&(length, _)| length > 0)
        .min_by_key(|&(length, rule)| (std::cmp::Reverse(length), rule))
}

/// Returns the number of the bytes of the first character of `input`.
pub fn character(input: &str) -> usize {
    input.chars().next().map_or(1, char::len_utf8)
}
