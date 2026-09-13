//! End-to-end tests for generated lexers.

#![deny(dead_code)]

use lxr::Lexer;

#[derive(Debug, PartialEq, Lexer)]
enum Token {
    #[lxr("a+")]
    As,
    #[lxr("b")]
    B,
}

#[test]
fn derived_lexer_returns_the_longest_prefix() {
    assert_eq!(Token::scan("aaab"), Some((Token::As, 3)));
    assert_eq!(Token::scan("b"), Some((Token::B, 1)));
    assert_eq!(Token::scan("c"), None);
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Lexer)]
enum Priority {
    #[lxr("a")]
    First,
    #[lxr("a")]
    Second,
}

#[test]
fn derived_lexer_breaks_equal_length_ties_by_rule_order() {
    assert_eq!(Priority::scan("a"), Some((Priority::First, 1)));
}

#[derive(Debug, PartialEq, Lexer)]
enum Unicode {
    #[lxr("é+")]
    EAcute,
}

#[test]
fn derived_lexer_counts_utf8_bytes() {
    assert_eq!(Unicode::scan("éé!"), Some((Unicode::EAcute, 4)));
}
