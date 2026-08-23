//! Scanning throughput of lxr in an executable that contains no Logos lexer.

mod common;
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use lxr::Lexer;
use std::hint::black_box;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Lexer)]
#[lxr(skip = r"[ \t\r\n]+")]
enum Json {
    #[lxr(token = "{")]
    ObjectOpen,
    #[lxr(token = "}")]
    ObjectClose,
    #[lxr(token = "[")]
    ArrayOpen,
    #[lxr(token = "]")]
    ArrayClose,
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
    #[lxr(regex = r#""([^"\\]|\\.)*""#)]
    String,
    #[lxr(regex = r"-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?")]
    Number,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Lexer)]
#[lxr(skip = r"[ \t\r\n]+")]
#[lxr(skip = r"//[^\n]*")]
enum Code {
    #[lxr(token = "fn")]
    Function,
    #[lxr(token = "let")]
    Let,
    #[lxr(token = "mut")]
    Mut,
    #[lxr(token = "if")]
    If,
    #[lxr(token = "else")]
    Else,
    #[lxr(token = "while")]
    While,
    #[lxr(token = "return")]
    Return,
    #[lxr(token = "->")]
    Arrow,
    #[lxr(token = "==")]
    Equal,
    #[lxr(token = "=")]
    Assign,
    #[lxr(token = "+")]
    Plus,
    #[lxr(token = "-")]
    Minus,
    #[lxr(token = "*")]
    Star,
    #[lxr(token = "/")]
    Slash,
    #[lxr(token = "(")]
    ParenOpen,
    #[lxr(token = ")")]
    ParenClose,
    #[lxr(token = "{")]
    BraceOpen,
    #[lxr(token = "}")]
    BraceClose,
    #[lxr(token = "[")]
    BracketOpen,
    #[lxr(token = "]")]
    BracketClose,
    #[lxr(token = ":")]
    Colon,
    #[lxr(token = ",")]
    Comma,
    #[lxr(token = ";")]
    Semicolon,
    #[lxr(regex = r#""([^"\\]|\\.)*""#)]
    String,
    #[lxr(regex = r"[0-9]+(\.[0-9]+)?")]
    Number,
    #[lxr(regex = r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Lexer)]
#[lxr(skip = r"[ \t\r\n]+")]
enum Synthetic {
    #[lxr(token = "transformation")]
    LongKeyword,
    #[lxr(token = "transform")]
    Keyword,
    #[lxr(token = "trans")]
    ShortKeyword,
    #[lxr(regex = r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,
    #[lxr(regex = r"[0-9]+")]
    Number,
    #[lxr(regex = r#""([^"\\]|\\.)*""#)]
    String,
    #[lxr(token = "+")]
    Plus,
    #[lxr(token = "-")]
    Minus,
    #[lxr(token = "*")]
    Star,
    #[lxr(token = "/")]
    Slash,
    #[lxr(token = "=")]
    Equal,
    #[lxr(token = ";")]
    Semicolon,
    #[lxr(token = "(")]
    Open,
    #[lxr(token = ")")]
    Close,
}

fn tokens<T: Lexer>(input: &str) -> (usize, usize) {
    T::scan(input).fold((0, 0), |(tokens, errors), item| {
        black_box(&item);
        if item.is_ok() {
            (tokens + 1, errors)
        } else {
            (tokens, errors + 1)
        }
    })
}

fn measure<T: Lexer>(criterion: &mut Criterion, name: &str, input: String) {
    let mut group = criterion.benchmark_group(name);
    group.throughput(Throughput::Bytes(input.len() as u64));
    group.bench_function("lxr", |b| b.iter(|| tokens::<T>(black_box(&input))));
    group.finish();
}

fn suite(criterion: &mut Criterion) {
    measure::<Json>(criterion, "realistic/json_compact", common::compact_json());
    measure::<Json>(criterion, "realistic/json_pretty", common::pretty_json());
    measure::<Code>(criterion, "realistic/rust_source", common::rust_source());
    for (name, seed) in common::SYNTHETIC {
        measure::<Synthetic>(
            criterion,
            &format!("synthetic/{name}"),
            common::repeat(seed),
        );
    }
    let input = common::rust_source();
    let mut group = criterion.benchmark_group("api/spans");
    group.throughput(Throughput::Bytes(input.len() as u64));
    group.bench_function("lxr", |b| {
        b.iter(|| {
            Code::scan(black_box(&input))
                .located()
                .fold(0, |count, item| {
                    let _ = black_box(item);
                    count + 1
                })
        })
    });
    group.finish();
}

criterion_group! { name = benches; config = common::configured(); targets = suite }
criterion_main!(benches);
