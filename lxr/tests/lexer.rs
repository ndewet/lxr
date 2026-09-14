//! End-to-end tests for generated lexers.

#![deny(dead_code)]

use lxr::{Lexer, ScanError, Spanned};
use std::convert::Infallible;
use std::str::FromStr;

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

#[derive(Debug, PartialEq, Lexer)]
#[lxr(skip = "[ \\t\\n]+")]
#[lxr(skip = "//[^\\n]*")]
enum WithTrivia {
    #[lxr("[a-z]+")]
    Identifier,
}

#[test]
fn derived_lexer_skips_whitespace_and_comments() {
    assert_eq!(
        WithTrivia::scan(" \t// note\nname"),
        Some((WithTrivia::Identifier, 14))
    );
}

#[test]
fn scanner_keeps_spans_and_recovers_after_invalid_input() {
    let scanned: Vec<_> = WithTrivia::scanner("one @ two").collect();

    assert_eq!(
        scanned,
        vec![
            Ok(Spanned {
                token: WithTrivia::Identifier,
                span: 0..3,
            }),
            Err(ScanError::Unrecognized { span: 4..5 }),
            Ok(Spanned {
                token: WithTrivia::Identifier,
                span: 6..9,
            }),
        ]
    );
}

#[derive(Debug, PartialEq)]
struct Identifier(String);

impl FromStr for Identifier {
    type Err = Infallible;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Ok(Self(text.to_owned()))
    }
}

#[derive(Debug, PartialEq, Lexer)]
enum Value {
    #[lxr("[0-9]+")]
    Integer(u64),
    #[lxr("[a-z]+")]
    Identifier(Identifier),
    #[lxr("![a-z]+", strip_bang)]
    Shouted(String),
    #[lxr("#[a-z]+", reject_hash)]
    Rejected(String),
}

fn strip_bang(text: &str) -> Result<String, Infallible> {
    Ok(text[1..].to_uppercase())
}

fn reject_hash(_: &str) -> Result<String, &'static str> {
    Err("hash-prefixed values are not permitted")
}

#[test]
fn derived_lexer_parses_owned_payloads() {
    let scanned: Vec<_> = Value::scanner("42 name !hello").collect();

    assert_eq!(
        scanned,
        vec![
            Ok(Spanned {
                token: Value::Integer(42),
                span: 0..2,
            }),
            Err(ScanError::Unrecognized { span: 2..3 }),
            Ok(Spanned {
                token: Value::Identifier(Identifier("name".to_owned())),
                span: 3..7,
            }),
            Err(ScanError::Unrecognized { span: 7..8 }),
            Ok(Spanned {
                token: Value::Shouted("HELLO".to_owned()),
                span: 8..14,
            }),
        ]
    );
}

#[test]
fn scanner_reports_payload_conversion_errors_and_recovers() {
    let scanned: Vec<_> = Value::scanner("18446744073709551616 7").collect();

    assert!(matches!(
        scanned.first(),
        Some(Err(ScanError::InvalidPayload { span, .. })) if span == &(0..20)
    ));
    assert_eq!(
        scanned.last(),
        Some(&Ok(Spanned {
            token: Value::Integer(7),
            span: 21..22,
        }))
    );
}

#[test]
fn scanner_reports_explicit_converter_errors_and_recovers() {
    let scanned: Vec<_> = Value::scanner("#no 7").collect();

    assert_eq!(
        scanned,
        vec![
            Err(ScanError::InvalidPayload {
                span: 0..3,
                message: "hash-prefixed values are not permitted".to_owned(),
            }),
            Err(ScanError::Unrecognized { span: 3..4 }),
            Ok(Spanned {
                token: Value::Integer(7),
                span: 4..5,
            }),
        ]
    );
}
