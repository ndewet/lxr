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

#[derive(Debug, PartialEq, Lexer)]
#[lxr(mode = Comment)]
#[lxr(skip = r"[ \t\r\n]+")]
#[lxr(skip = r"/\*", push = Comment)]
#[lxr(skip = r"/\*", modes = Comment, push = Comment)]
#[lxr(skip = r"\*/", modes = Comment, pop)]
#[lxr(skip = r"[^*/]+|[*/]", modes = Comment)]
enum WithNestedComments {
    #[lxr("[a-z]+")]
    Identifier,
}

#[test]
fn derived_lexer_skips_arbitrarily_nested_block_comments() {
    let input = "one /* outer /* nested */ still outer */ two";
    let scanned: Vec<_> = WithNestedComments::scanner(input).collect();

    assert_eq!(
        scanned,
        vec![
            Ok(Spanned {
                token: WithNestedComments::Identifier,
                span: 0..3
            }),
            Ok(Spanned {
                token: WithNestedComments::Identifier,
                span: 41..44
            }),
        ]
    );
}

#[test]
fn derived_lexer_reports_an_unterminated_nested_comment() {
    let input = "one /* outer /* nested */";
    let scanned: Vec<_> = WithNestedComments::scanner(input).collect();

    assert_eq!(
        scanned,
        vec![
            Ok(Spanned {
                token: WithNestedComments::Identifier,
                span: 0..3
            }),
            Err(ScanError::UnterminatedMode {
                span: 4..25,
                mode: "Comment"
            }),
        ]
    );
}

#[derive(Debug, PartialEq, Lexer)]
#[lxr(mode = String)]
enum ModeTokens {
    #[lxr("[a-z]+")]
    Identifier,
    #[lxr("\"", push = String)]
    StringStart,
    #[lxr(r#"[^"\\]+"#, modes = String)]
    StringText,
    #[lxr("\"", modes = String, pop)]
    StringEnd,
}

#[test]
fn derived_lexer_only_enables_tokens_in_their_declared_mode() {
    let scanned: Vec<_> = ModeTokens::scanner("name \"text\" tail")
        .map(|item| item.map(|spanned| spanned.token))
        .collect();
    assert_eq!(
        scanned,
        vec![
            Ok(ModeTokens::Identifier),
            Err(ScanError::Unrecognized { span: 4..5 }),
            Ok(ModeTokens::StringStart),
            Ok(ModeTokens::StringText),
            Ok(ModeTokens::StringEnd),
            Err(ScanError::Unrecognized { span: 11..12 }),
            Ok(ModeTokens::Identifier),
        ]
    );
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

#[derive(Debug, PartialEq, Lexer)]
enum Repetition {
    #[lxr("a{2,}b")]
    OpenEnded,
    #[lxr("(a?)*b")]
    NullableLoop,
}

#[test]
fn derived_lexer_handles_open_ended_and_nullable_repetition() {
    assert_eq!(Repetition::scan("aaab"), Some((Repetition::OpenEnded, 4)));
    assert_eq!(Repetition::scan("b"), Some((Repetition::NullableLoop, 1)));
    assert_eq!(Repetition::scan("aaa"), None);
}

#[derive(Debug, PartialEq, Lexer)]
enum UnicodeBoundaries {
    #[lxr(r"[\x7f-\x{80}]")]
    Boundary,
    #[lxr("(é|€|𐐷)+")]
    Scalar,
}

#[test]
fn derived_lexer_matches_utf8_boundaries_and_keeps_byte_spans() {
    let scanned: Vec<_> = UnicodeBoundaries::scanner("\u{7f}\u{80}é€𐐷@").collect();

    assert_eq!(
        scanned,
        vec![
            Ok(Spanned {
                token: UnicodeBoundaries::Boundary,
                span: 0..1,
            }),
            Ok(Spanned {
                token: UnicodeBoundaries::Boundary,
                span: 1..3,
            }),
            Ok(Spanned {
                token: UnicodeBoundaries::Scalar,
                span: 3..12,
            }),
            Err(ScanError::Unrecognized { span: 12..13 }),
        ]
    );
}

#[derive(Debug, PartialEq, Lexer)]
enum NegatedClass {
    #[lxr("a")]
    A,
    #[lxr(r"[^a]")]
    NotA,
}

#[test]
fn derived_lexer_matches_negated_classes_including_newlines_and_unicode() {
    let scanned: Vec<_> = NegatedClass::scanner("a\né").collect();

    assert_eq!(
        scanned,
        vec![
            Ok(Spanned {
                token: NegatedClass::A,
                span: 0..1,
            }),
            Ok(Spanned {
                token: NegatedClass::NotA,
                span: 1..2,
            }),
            Ok(Spanned {
                token: NegatedClass::NotA,
                span: 2..4,
            }),
        ]
    );
}

#[derive(Debug, Clone, PartialEq, Lexer)]
#[lxr(skip = r"[ \t\r\n]+")]
#[lxr(skip = r"//[^\n]*")]
enum RustToken {
    #[lxr("fn")]
    Fn,
    #[lxr("let")]
    Let,
    #[lxr("match")]
    Match,
    #[lxr("[A-Za-z_][A-Za-z0-9_]*")]
    Identifier,
    #[lxr("[0-9][0-9_]*")]
    Integer,
    #[lxr(r#""([^"\\]|\\.)*""#)]
    String,
    #[lxr("->")]
    Arrow,
    #[lxr("::")]
    PathSeparator,
    #[lxr(":")]
    Colon,
    #[lxr("==")]
    EqualEqual,
    #[lxr("=")]
    Equal,
    #[lxr("=>")]
    FatArrow,
    #[lxr(r"\.\.=")]
    RangeInclusive,
    #[lxr(r"\.\.")]
    Range,
    #[lxr(r"\.")]
    Dot,
    #[lxr(r"\(")]
    LeftParen,
    #[lxr(r"\)")]
    RightParen,
    #[lxr(r"\{")]
    LeftBrace,
    #[lxr(r"\}")]
    RightBrace,
    #[lxr(",")]
    Comma,
    #[lxr(";")]
    Semicolon,
}

fn rust_tokens(input: &str) -> Vec<Spanned<RustToken>> {
    RustToken::scanner(input)
        .map(|result| result.expect("the Rust-like input is recognized"))
        .collect()
}

#[test]
fn rust_like_lexer_recognizes_common_tokens() {
    assert_eq!(RustToken::scan("fn"), Some((RustToken::Fn, 2)));
    assert_eq!(
        RustToken::scan("fn_name"),
        Some((RustToken::Identifier, 7)),
        "a keyword must not steal an identifier prefix"
    );
    assert_eq!(
        RustToken::scan(r#""say: \"é\"""#),
        Some((RustToken::String, 13)),
        "escaped quotes and multi-byte characters stay inside one string literal"
    );
}

#[test]
fn rust_like_lexer_handles_comments_ranges_and_compound_punctuation() {
    let input = "fn crate::main(a: u32) -> String { let value = 12_345; // note\nmatch value { 0..=10 => \"é\", _ => \"other\" } }";
    let scanned = rust_tokens(input);

    assert_eq!(
        scanned
            .iter()
            .map(|spanned| &spanned.token)
            .collect::<Vec<_>>(),
        vec![
            &RustToken::Fn,
            &RustToken::Identifier,
            &RustToken::PathSeparator,
            &RustToken::Identifier,
            &RustToken::LeftParen,
            &RustToken::Identifier,
            &RustToken::Colon,
            &RustToken::Identifier,
            &RustToken::RightParen,
            &RustToken::Arrow,
            &RustToken::Identifier,
            &RustToken::LeftBrace,
            &RustToken::Let,
            &RustToken::Identifier,
            &RustToken::Equal,
            &RustToken::Integer,
            &RustToken::Semicolon,
            &RustToken::Match,
            &RustToken::Identifier,
            &RustToken::LeftBrace,
            &RustToken::Integer,
            &RustToken::RangeInclusive,
            &RustToken::Integer,
            &RustToken::FatArrow,
            &RustToken::String,
            &RustToken::Comma,
            &RustToken::Identifier,
            &RustToken::FatArrow,
            &RustToken::String,
            &RustToken::RightBrace,
            &RustToken::RightBrace,
        ]
    );
    assert_eq!(
        scanned
            .iter()
            .map(
                |spanned| &input[usize::try_from(spanned.span.start).expect("string offset")
                    ..usize::try_from(spanned.span.end).expect("string offset")]
            )
            .collect::<Vec<_>>(),
        vec![
            "fn",
            "crate",
            "::",
            "main",
            "(",
            "a",
            ":",
            "u32",
            ")",
            "->",
            "String",
            "{",
            "let",
            "value",
            "=",
            "12_345",
            ";",
            "match",
            "value",
            "{",
            "0",
            "..=",
            "10",
            "=>",
            "\"é\"",
            ",",
            "_",
            "=>",
            "\"other\"",
            "}",
            "}",
        ]
    );
}

#[test]
fn rust_like_lexer_prefers_the_longest_compound_punctuation() {
    let input = "a===b..c...d";
    let scanned = rust_tokens(input);

    assert_eq!(
        scanned
            .iter()
            .map(|spanned| &spanned.token)
            .collect::<Vec<_>>(),
        vec![
            &RustToken::Identifier,
            &RustToken::EqualEqual,
            &RustToken::Equal,
            &RustToken::Identifier,
            &RustToken::Range,
            &RustToken::Identifier,
            &RustToken::Range,
            &RustToken::Dot,
            &RustToken::Identifier,
        ]
    );
    assert_eq!(
        scanned
            .iter()
            .map(
                |spanned| &input[usize::try_from(spanned.span.start).expect("string offset")
                    ..usize::try_from(spanned.span.end).expect("string offset")]
            )
            .collect::<Vec<_>>(),
        vec!["a", "==", "=", "b", "..", "c", "..", ".", "d"]
    );
}
