//! Scanning-throughput benchmarks for representative and extreme inputs.
//!
//! Comparable cases use identical rules and input for lxr and Logos. Run with
//! `cargo bench --package lxr --bench scan`.

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use logos::Logos;
use lxr::Lexer;
use std::{hint::black_box, time::Duration};

#[derive(Clone, Copy, Debug, Eq, Logos, PartialEq, Lexer)]
#[logos(skip r"[ \t\r\n]+")]
#[lxr(skip = r"[ \t\r\n]+")]
enum Json {
    #[token("{")]
    #[lxr(token = "{")]
    ObjectOpen,
    #[token("}")]
    #[lxr(token = "}")]
    ObjectClose,
    #[token("[")]
    #[lxr(token = "[")]
    ArrayOpen,
    #[token("]")]
    #[lxr(token = "]")]
    ArrayClose,
    #[token(":")]
    #[lxr(token = ":")]
    Colon,
    #[token(",")]
    #[lxr(token = ",")]
    Comma,
    #[token("true")]
    #[lxr(token = "true")]
    True,
    #[token("false")]
    #[lxr(token = "false")]
    False,
    #[token("null")]
    #[lxr(token = "null")]
    Null,
    #[regex(r#""([^"\\]|\\.)*""#)]
    #[lxr(regex = r#""([^"\\]|\\.)*""#)]
    String,
    #[regex(r"-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?")]
    #[lxr(regex = r"-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?")]
    Number,
}

#[derive(Clone, Copy, Debug, Eq, Logos, PartialEq, Lexer)]
#[logos(skip r"[ \t\r\n]+")]
#[logos(skip(r"//[^\n]*", allow_greedy = true))]
#[lxr(skip = r"[ \t\r\n]+")]
#[lxr(skip = r"//[^\n]*")]
enum Code {
    #[token("fn")]
    #[lxr(token = "fn")]
    Function,
    #[token("let")]
    #[lxr(token = "let")]
    Let,
    #[token("mut")]
    #[lxr(token = "mut")]
    Mut,
    #[token("if")]
    #[lxr(token = "if")]
    If,
    #[token("else")]
    #[lxr(token = "else")]
    Else,
    #[token("while")]
    #[lxr(token = "while")]
    While,
    #[token("return")]
    #[lxr(token = "return")]
    Return,
    #[token("->")]
    #[lxr(token = "->")]
    Arrow,
    #[token("==")]
    #[lxr(token = "==")]
    Equal,
    #[token("=")]
    #[lxr(token = "=")]
    Assign,
    #[token("+")]
    #[lxr(token = "+")]
    Plus,
    #[token("-")]
    #[lxr(token = "-")]
    Minus,
    #[token("*")]
    #[lxr(token = "*")]
    Star,
    #[token("/")]
    #[lxr(token = "/")]
    Slash,
    #[token("(")]
    #[lxr(token = "(")]
    ParenOpen,
    #[token(")")]
    #[lxr(token = ")")]
    ParenClose,
    #[token("{")]
    #[lxr(token = "{")]
    BraceOpen,
    #[token("}")]
    #[lxr(token = "}")]
    BraceClose,
    #[token("[")]
    #[lxr(token = "[")]
    BracketOpen,
    #[token("]")]
    #[lxr(token = "]")]
    BracketClose,
    #[token(":")]
    #[lxr(token = ":")]
    Colon,
    #[token(",")]
    #[lxr(token = ",")]
    Comma,
    #[token(";")]
    #[lxr(token = ";")]
    Semicolon,
    #[regex(r#""([^"\\]|\\.)*""#)]
    #[lxr(regex = r#""([^"\\]|\\.)*""#)]
    String,
    #[regex(r"[0-9]+(\.[0-9]+)?")]
    #[lxr(regex = r"[0-9]+(\.[0-9]+)?")]
    Number,
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    #[lxr(regex = r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,
}

#[derive(Clone, Copy, Debug, Eq, Logos, PartialEq, Lexer)]
#[logos(skip r"[ \t\r\n]+")]
#[lxr(skip = r"[ \t\r\n]+")]
enum Synthetic {
    #[token("transformation")]
    #[lxr(token = "transformation")]
    LongKeyword,
    #[token("transform")]
    #[lxr(token = "transform")]
    Keyword,
    #[token("trans")]
    #[lxr(token = "trans")]
    ShortKeyword,
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    #[lxr(regex = r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,
    #[regex(r"[0-9]+")]
    #[lxr(regex = r"[0-9]+")]
    Number,
    #[regex(r#""([^"\\]|\\.)*""#)]
    #[lxr(regex = r#""([^"\\]|\\.)*""#)]
    String,
    #[token("+")]
    #[lxr(token = "+")]
    Plus,
    #[token("-")]
    #[lxr(token = "-")]
    Minus,
    #[token("*")]
    #[lxr(token = "*")]
    Star,
    #[token("/")]
    #[lxr(token = "/")]
    Slash,
    #[token("=")]
    #[lxr(token = "=")]
    Equal,
    #[token(";")]
    #[lxr(token = ";")]
    Semicolon,
    #[token("(")]
    #[lxr(token = "(")]
    Open,
    #[token(")")]
    #[lxr(token = ")")]
    Close,
}

fn repeat_to_size(seed: &str, minimum: usize) -> String {
    seed.repeat(minimum.div_ceil(seed.len()))
}

fn compact_json() -> String {
    repeat_to_size(
        r#"{"id":8342,"active":true,"score":-12.75e3,"owner":null,"tags":["lexer","rust","fast"],"escaped":"quote: \\"}"#,
        128 * 1024,
    )
}

fn pretty_json() -> String {
    repeat_to_size(
        "{\n  \"id\": 8342,\n  \"active\": true,\n  \"score\": -12.75e3,\n  \"owner\": null,\n  \"tags\": [\"lexer\", \"rust\", \"fast\"]\n}\n",
        128 * 1024,
    )
}

fn rust_source() -> String {
    repeat_to_size(
        r#"
// A representative mixture of code, comments, strings, and whitespace.
fn accumulate(values: [number]) -> number {
    let mut total = 0;
    while total == 0 { total = total + values[0]; }
    if total == 42 { return "forty-two"; } else { return "other"; }
}
"#,
        128 * 1024,
    )
}

fn lxr_tokens<T: Lexer>(input: &str) -> (usize, usize) {
    T::scan(input).fold((0, 0), |(tokens, errors), item| {
        black_box(&item);
        if item.is_ok() {
            (tokens + 1, errors)
        } else {
            (tokens, errors + 1)
        }
    })
}

fn lxr_spans<T: Lexer>(input: &str) -> (usize, usize) {
    T::scan(input)
        .located()
        .fold((0, 0), |(tokens, errors), item| {
            black_box(&item);
            if item.is_ok() {
                (tokens + 1, errors)
            } else {
                (tokens, errors + 1)
            }
        })
}

fn logos_tokens<'a, T: Logos<'a, Source = str>>(input: &'a str) -> (usize, usize)
where
    T::Extras: Default,
{
    T::lexer(input).fold((0, 0), |(tokens, errors), item| {
        black_box(&item);
        if item.is_ok() {
            (tokens + 1, errors)
        } else {
            (tokens, errors + 1)
        }
    })
}

fn logos_spans<'a, T: Logos<'a, Source = str>>(input: &'a str) -> (usize, usize)
where
    T::Extras: Default,
{
    T::lexer(input)
        .spanned()
        .fold((0, 0), |(tokens, errors), (item, span)| {
            black_box(&span);
            if item.is_ok() {
                (tokens + 1, errors)
            } else {
                (tokens, errors + 1)
            }
        })
}

fn compare<T>(criterion: &mut Criterion, name: &str, input: String)
where
    T: Lexer,
    for<'a> T: Logos<'a, Source = str>,
    for<'a> <T as Logos<'a>>::Extras: Default,
{
    let expected = lxr_tokens::<T>(&input);
    assert_eq!(
        expected,
        logos_tokens::<T>(&input),
        "lexer results for {name}"
    );
    let mut group = criterion.benchmark_group(name);
    group.throughput(Throughput::Bytes(input.len() as u64));
    group.bench_function("lxr", |b| b.iter(|| lxr_tokens::<T>(black_box(&input))));
    group.bench_function("logos", |b| b.iter(|| logos_tokens::<T>(black_box(&input))));
    group.finish();
}

fn realistic(criterion: &mut Criterion) {
    compare::<Json>(criterion, "realistic/json_compact", compact_json());
    compare::<Json>(criterion, "realistic/json_pretty", pretty_json());
    compare::<Code>(criterion, "realistic/rust_source", rust_source());
}

fn synthetic(criterion: &mut Criterion) {
    for (name, seed) in [
        ("dense_punctuation", "()+-*/=;"),
        ("short_identifiers", "a b c d e f g h "),
        (
            "long_identifiers",
            "a_single_identifier_with_enough_bytes_to_stress_long_matches ",
        ),
        (
            "keyword_prefixes",
            "trans transformation transform transformed transformer ",
        ),
        (
            "mostly_whitespace",
            "                                name\n",
        ),
        ("unicode_strings", "\"København 東京 🚲 naïve café\" "),
        ("error_recovery", "valid @ valid € valid ? "),
    ] {
        compare::<Synthetic>(
            criterion,
            &format!("synthetic/{name}"),
            repeat_to_size(seed, 128 * 1024),
        );
    }
}

fn spans(criterion: &mut Criterion) {
    let input = rust_source();
    assert_eq!(lxr_spans::<Code>(&input), logos_spans::<Code>(&input));
    let mut group = criterion.benchmark_group("api/spans");
    group.throughput(Throughput::Bytes(input.len() as u64));
    group.bench_function("lxr", |b| b.iter(|| lxr_spans::<Code>(black_box(&input))));
    group.bench_function("logos", |b| {
        b.iter(|| logos_spans::<Code>(black_box(&input)))
    });
    group.finish();
}

fn configured() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(3))
        .sample_size(50)
}

criterion_group! { name = benches; config = configured(); targets = realistic, synthetic, spans }
criterion_main!(benches);
