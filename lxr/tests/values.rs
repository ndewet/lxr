//! Asserts that a token carries the value that its rule matched.
//!
//! A variant that holds one field takes that field from the text of the match, through
//! [`FromStr`](std::str::FromStr). A variant that holds no field carries nothing.
//!
//! A rule can match text that the field does not hold. `[0-9]+` matches a number of any length,
//! and a `u8` holds a number up to 255. The scan reports that text, and it reads on.

mod common;

use common::steps;
use lxr::{Lexer, ScanError, ScanErrorKind};

/// The tokens of a language in which each value carries its own type.
#[derive(Debug, Clone, PartialEq, Lexer)]
#[lxr(skip = r"[ \t\n]+")]
enum Token {
    #[lxr(token = "let")]
    Let,
    #[lxr(regex = "[a-z][a-z0-9]*")]
    Name(String),
    #[lxr(regex = r"[0-9]+\.[0-9]+")]
    Float(f64),
    #[lxr(regex = "[0-9]+")]
    Int(i64),
    #[lxr(token = "=")]
    Assign,
}

/// A lexer whose field is narrow, thus a long number does not fit it.
#[derive(Debug, PartialEq, Eq, Lexer)]
#[lxr(skip = " +")]
enum Small {
    #[lxr(regex = "[0-9]+")]
    Byte(u8),
}

/// The tokens of `input`. Each character of the input belongs to a token.
fn tokens(input: &str) -> Vec<Token> {
    Token::scan(input)
        .map(|found| found.expect("each character of the input belongs to a token"))
        .collect()
}

#[test]
fn a_variant_of_one_field_carries_the_text_of_its_match() {
    assert_eq!(
        tokens("let width = 80"),
        vec![
            Token::Let,
            Token::Name("width".to_owned()),
            Token::Assign,
            Token::Int(80),
        ]
    );
}

#[test]
fn a_field_of_a_string_carries_the_whole_match() {
    assert_eq!(tokens("abc"), vec![Token::Name("abc".to_owned())]);
    assert_eq!(tokens("a9"), vec![Token::Name("a9".to_owned())]);
}

#[test]
fn a_field_of_a_number_carries_the_number_and_not_the_text() {
    assert_eq!(tokens("0"), vec![Token::Int(0)]);
    assert_eq!(tokens("42"), vec![Token::Int(42)]);
    assert_eq!(tokens("007"), vec![Token::Int(7)], "the parse drops a zero");
}

#[test]
fn a_field_of_a_float_carries_the_number() {
    assert_eq!(tokens("1.5"), vec![Token::Float(1.5)]);
    assert_eq!(tokens("0.25"), vec![Token::Float(0.25)]);
}

#[test]
fn a_variant_of_no_field_carries_nothing() {
    assert_eq!(tokens("let ="), vec![Token::Let, Token::Assign]);
}

#[test]
fn the_longest_match_decides_which_field_takes_the_text() {
    assert_eq!(
        tokens("1.5 2"),
        vec![Token::Float(1.5), Token::Int(2)],
        "the float reads three characters, thus the integer does not read the first one"
    );
}

#[test]
fn each_token_of_a_line_carries_its_own_value() {
    assert_eq!(
        tokens("let a = 1 let b = 2.5"),
        vec![
            Token::Let,
            Token::Name("a".to_owned()),
            Token::Assign,
            Token::Int(1),
            Token::Let,
            Token::Name("b".to_owned()),
            Token::Assign,
            Token::Float(2.5),
        ]
    );
}

#[test]
fn a_token_holds_no_borrow_of_the_input() {
    let held = {
        let source = String::from("width");
        tokens(&source)
    };

    assert_eq!(held, vec![Token::Name("width".to_owned())]);
}

#[test]
fn a_text_that_the_field_does_not_hold_gives_an_error() {
    let error = Small::scan("999")
        .next()
        .expect("the scan gives one result")
        .expect_err("999 does not fit a u8");

    assert_eq!(error.kind(), ScanErrorKind::Value);
    assert_eq!(error.span(), 0..3);
    assert_eq!((error.line(), error.column()), (1, 1));
}

#[test]
fn the_error_of_a_value_covers_the_whole_match() {
    let found: Vec<_> = Small::scan("1 300").collect();

    assert_eq!(found[0], Ok(Small::Byte(1)));
    let error = found[1].as_ref().expect_err("300 does not fit a u8");
    assert_eq!(
        error.span(),
        2..5,
        "the span covers each digit of the match"
    );
    assert_eq!(error.kind(), ScanErrorKind::Value);
}

#[test]
fn the_scan_reads_on_after_a_text_that_the_field_does_not_hold() {
    let found: Vec<_> = Small::scan("300 7").collect();

    assert_eq!(found.len(), 2);
    assert!(found[0].is_err());
    assert_eq!(found[1], Ok(Small::Byte(7)));
}

#[test]
fn a_value_and_a_missing_rule_give_two_kinds_of_error() {
    let found: Vec<_> = Small::scan("300$").collect();

    let kinds: Vec<ScanErrorKind> = found
        .iter()
        .filter_map(|result| result.as_ref().err().map(|error: &ScanError| error.kind()))
        .collect();

    assert_eq!(kinds, vec![ScanErrorKind::Value, ScanErrorKind::NoRule]);
}

#[test]
fn an_error_of_a_value_states_the_correction() {
    let error = Small::scan("999")
        .next()
        .expect("the scan gives one result")
        .expect_err("999 does not fit a u8");

    let help = error.help();
    assert!(help.contains("Correct the input"), "{help}");
    assert!(help.contains("wider field"), "{help}");
    assert_eq!(
        error.to_string(),
        "the text does not fit the field of its token at line 1, column 1"
    );
}

#[test]
fn the_span_of_a_token_still_slices_the_input_back_to_its_text() {
    let mut scan = Token::scan("let width = 80");
    let mut read = Vec::new();

    while let Some(found) = scan.next() {
        found.expect("each character belongs to a token");
        read.push(scan.slice().to_owned());
    }

    assert_eq!(read, vec!["let", "width", "=", "80"]);
}

#[test]
fn a_value_and_the_steps_of_the_scan_agree() {
    use common::Step;

    assert_eq!(
        steps::<Token>("a 1"),
        vec![
            Step::Token(Token::Name("a".to_owned()), 0..1),
            Step::Token(Token::Int(1), 2..3),
        ]
    );
}
