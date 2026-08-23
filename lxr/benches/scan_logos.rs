//! Scanning throughput of Logos in an executable that contains no lxr lexer.

mod common;
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use logos::Logos;
use std::hint::black_box;

#[derive(Clone, Copy, Debug, Eq, Logos, PartialEq)]
#[logos(skip r"[ \t\r\n]+")]
enum Json {
    #[token("{")]
    ObjectOpen,
    #[token("}")]
    ObjectClose,
    #[token("[")]
    ArrayOpen,
    #[token("]")]
    ArrayClose,
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
    #[regex(r#""([^"\\]|\\.)*""#)]
    String,
    #[regex(r"-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?")]
    Number,
}

#[derive(Clone, Copy, Debug, Eq, Logos, PartialEq)]
#[logos(skip r"[ \t\r\n]+")]
#[logos(skip(r"//[^\n]*", allow_greedy = true))]
enum Code {
    #[token("fn")]
    Function,
    #[token("let")]
    Let,
    #[token("mut")]
    Mut,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("while")]
    While,
    #[token("return")]
    Return,
    #[token("->")]
    Arrow,
    #[token("==")]
    Equal,
    #[token("=")]
    Assign,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("(")]
    ParenOpen,
    #[token(")")]
    ParenClose,
    #[token("{")]
    BraceOpen,
    #[token("}")]
    BraceClose,
    #[token("[")]
    BracketOpen,
    #[token("]")]
    BracketClose,
    #[token(":")]
    Colon,
    #[token(",")]
    Comma,
    #[token(";")]
    Semicolon,
    #[regex(r#""([^"\\]|\\.)*""#)]
    String,
    #[regex(r"[0-9]+(\.[0-9]+)?")]
    Number,
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,
}

#[derive(Clone, Copy, Debug, Eq, Logos, PartialEq)]
#[logos(skip r"[ \t\r\n]+")]
enum Synthetic {
    #[token("transformation")]
    LongKeyword,
    #[token("transform")]
    Keyword,
    #[token("trans")]
    ShortKeyword,
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,
    #[regex(r"[0-9]+")]
    Number,
    #[regex(r#""([^"\\]|\\.)*""#)]
    String,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("=")]
    Equal,
    #[token(";")]
    Semicolon,
    #[token("(")]
    Open,
    #[token(")")]
    Close,
}

fn tokens<'a, T: Logos<'a, Source = str>>(input: &'a str) -> (usize, usize)
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

fn measure<T>(criterion: &mut Criterion, name: &str, input: String)
where
    for<'a> T: Logos<'a, Source = str>,
    for<'a> <T as Logos<'a>>::Extras: Default,
{
    let mut group = criterion.benchmark_group(name);
    group.throughput(Throughput::Bytes(input.len() as u64));
    group.bench_function("logos", |b| b.iter(|| tokens::<T>(black_box(&input))));
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
    group.bench_function("logos", |b| {
        b.iter(|| {
            Code::lexer(black_box(&input))
                .spanned()
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
