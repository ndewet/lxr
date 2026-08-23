//! Measures the scan of a lexer that lxr derives.
//!
//! The benchmark reads a document of JSON and a document of source code. It scans each one with a
//! lexer of lxr and with a lexer of [`logos`], thus a reader sees the cost of one byte and the two
//! lexers together. The measure is the time of the whole scan, and criterion divides it by the
//! number of the bytes.
//!
//! Each document holds bytes that each lexer reads without a fault. Thus the measure holds the
//! cost of a match alone, and not the cost of a report.
//!
//! The scan of lxr counts the line and the column of each token, and the lexer of logos counts
//! neither one. Thus the measure of lxr holds that cost as well.
//!
//! Run it with `cargo bench -p lxr`.

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use logos::Logos;
use lxr::Lexer;

/// The tokens of JSON, as the example `json` gives them.
#[derive(Debug, PartialEq, Eq, Lexer)]
#[lxr(skip = r"[ \t\r\n]+")]
enum Json {
    #[lxr(token = "{")]
    OpenObject,
    #[lxr(token = "}")]
    CloseObject,
    #[lxr(token = "[")]
    OpenArray,
    #[lxr(token = "]")]
    CloseArray,
    #[lxr(token = ":")]
    Colon,
    #[lxr(token = ",")]
    Comma,
    #[lxr(token = "true")]
    True,
    #[lxr(token = "false")]
    False,
    #[lxr(token = "null")]
    Null,
    #[lxr(regex = r#""([^\x00-\x1F"\\]|\\["\\/bfnrt]|\\u[0-9a-fA-F]{4})*""#)]
    Text,
    #[lxr(regex = r"-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?")]
    Number,
}

/// The tokens of JSON, with the same patterns, for [`logos`].
#[derive(Debug, PartialEq, Eq, Logos)]
#[logos(skip r"[ \t\r\n]+")]
enum LogosJson {
    #[token("{")]
    OpenObject,
    #[token("}")]
    CloseObject,
    #[token("[")]
    OpenArray,
    #[token("]")]
    CloseArray,
    #[token(":")]
    Colon,
    #[token(",")]
    Comma,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("null")]
    Null,
    #[regex(r#""([^\x00-\x1F"\\]|\\["\\/bfnrt]|\\u[0-9a-fA-F]{4})*""#)]
    Text,
    #[regex(r"-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?")]
    Number,
}

/// The tokens of a language of keywords, names, numbers, and strings.
#[derive(Debug, PartialEq, Eq, Lexer)]
#[lxr(skip = r"[ \t\r\n]+")]
enum Code {
    #[lxr(token = "let")]
    Let,
    #[lxr(token = "fn")]
    Function,
    #[lxr(token = "if")]
    If,
    #[lxr(token = "else")]
    Else,
    #[lxr(token = "while")]
    While,
    #[lxr(token = "return")]
    Return,
    #[lxr(token = "=")]
    Assign,
    #[lxr(token = "+")]
    Plus,
    #[lxr(token = ";")]
    Semicolon,
    #[lxr(token = "(")]
    Open,
    #[lxr(token = ")")]
    Close,
    #[lxr(token = "{")]
    OpenBlock,
    #[lxr(token = "}")]
    CloseBlock,
    #[lxr(regex = "[a-zA-Z_][a-zA-Z0-9_]*")]
    Name,
    #[lxr(regex = "[0-9]+")]
    Number,
    #[lxr(regex = r#""([^"\\]|\\.)*""#)]
    Text,
}

/// The tokens of that language, with the same patterns, for [`logos`].
#[derive(Debug, PartialEq, Eq, Logos)]
#[logos(skip r"[ \t\r\n]+")]
enum LogosCode {
    #[token("let")]
    Let,
    #[token("fn")]
    Function,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("while")]
    While,
    #[token("return")]
    Return,
    #[token("=")]
    Assign,
    #[token("+")]
    Plus,
    #[token(";")]
    Semicolon,
    #[token("(")]
    Open,
    #[token(")")]
    Close,
    #[token("{")]
    OpenBlock,
    #[token("}")]
    CloseBlock,
    #[regex("[a-zA-Z_][a-zA-Z0-9_]*")]
    Name,
    #[regex("[0-9]+")]
    Number,
    #[regex(r#""([^"\\]|\\.)*""#)]
    Text,
}

/// Returns a document of JSON of `count` objects.
fn json(count: usize) -> String {
    let mut document = String::from("[\n");
    for index in 0..count {
        if index > 0 {
            document.push_str(",\n");
        }
        document.push_str(
            "  { \"name\": \"item\", \"index\": 0, \"ratio\": -1.5e-3, \"tags\": [1, 2, 3],\n    \
             \"open\": true, \"parent\": null, \"note\": \"a quote \\\" stays inside\" }",
        );
    }
    document.push_str("\n]\n");
    document
}

/// Returns a document of source code of `count` functions.
fn code(count: usize) -> String {
    let mut document = String::new();
    for _ in 0..count {
        document.push_str(
            "fn measure(width) {\n    let total = 0;\n    while total {\n        \
             total = total + width;\n    }\n    if total { return \"wide\"; } \
             else { return \"narrow\"; }\n}\n\n",
        );
    }
    document
}

/// Returns the number of the tokens that the lexer of lxr reads in `input`.
fn scanned<T: Lexer>(input: &str) -> usize {
    T::scan(input).filter(Result::is_ok).count()
}

/// Returns the number of the tokens that the lexer of [`logos`] reads in `input`.
fn lexed<'a, T: Logos<'a, Source = str>>(input: &'a str) -> usize
where
    T::Extras: Default,
{
    T::lexer(input).filter(Result::is_ok).count()
}

/// Asserts that the two lexers read the same number of tokens in `input`.
///
/// # Panics
///
/// This function panics if the two lexers disagree. The measure then holds two amounts of work,
/// and the comparison says nothing.
fn same_tokens(lxr: usize, logos: usize, name: &str) {
    assert_eq!(
        lxr, logos,
        "the two lexers of {name} must read the same tokens"
    );
}

/// Measures the scan of each document with each lexer.
fn scan(criterion: &mut Criterion) {
    let json = json(200);
    let code = code(200);

    same_tokens(scanned::<Json>(&json), lexed::<LogosJson>(&json), "JSON");
    same_tokens(scanned::<Code>(&code), lexed::<LogosCode>(&code), "code");

    let mut group = criterion.benchmark_group("json");
    group.throughput(Throughput::Bytes(json.len() as u64));
    group.bench_function("lxr", |bencher| {
        bencher.iter(|| scanned::<Json>(black_box(&json)));
    });
    group.bench_function("logos", |bencher| {
        bencher.iter(|| lexed::<LogosJson>(black_box(&json)));
    });
    group.finish();

    let mut group = criterion.benchmark_group("code");
    group.throughput(Throughput::Bytes(code.len() as u64));
    group.bench_function("lxr", |bencher| {
        bencher.iter(|| scanned::<Code>(black_box(&code)));
    });
    group.bench_function("logos", |bencher| {
        bencher.iter(|| lexed::<LogosCode>(black_box(&code)));
    });
    group.finish();
}

criterion_group!(benches, scan);
criterion_main!(benches);
