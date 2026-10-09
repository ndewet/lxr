//! Measures scanning throughput for a representative Rust-like grammar.

use lxr::{Lexer, Reader};
use std::hint::black_box;
use std::io::Cursor;
use std::time::Instant;

#[derive(Lexer)]
#[lexer(skip = r"[ \t\r\n]+")]
#[lexer(skip = r"//[^\n]*")]
enum Token {
    #[token("let")]
    Let,
    #[token("fn")]
    Function,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("return")]
    Return,
    #[token("[A-Za-z_][A-Za-z0-9_]*")]
    Identifier,
    #[token(r"[0-9]+\.[0-9]+")]
    Float,
    #[token("[0-9]+")]
    Integer,
    #[token(r#""([^"\\]|\\.)*""#)]
    String,
    #[token("==")]
    Equal,
    #[token("=")]
    Assign,
    #[token("->")]
    Arrow,
    #[token(r"\+")]
    Plus,
    #[token("-")]
    Minus,
    #[token(r"\*")]
    Star,
    #[token("/")]
    Slash,
    #[token(r"\(")]
    LeftParenthesis,
    #[token(r"\)")]
    RightParenthesis,
    #[token(r"\{")]
    LeftBrace,
    #[token(r"\}")]
    RightBrace,
    #[token(",")]
    Comma,
    #[token(";")]
    Semicolon,
}

fn input() -> String {
    concat!(
        "fn calculate(value_123) { ",
        "let result = value_123 * 42 + 3.5; ",
        "if result == 0 { return \"zero\"; } ",
        "else { return \"value\"; } } // one function\n",
    )
    .repeat(4096)
}

fn scan_slice(input: &str) -> usize {
    Token::scanner(black_box(input))
        .try_fold(0usize, |count, result| result.map(|_| count + 1))
        .expect("the benchmark input is valid")
}

fn scan_reader(input: &[u8]) -> usize {
    let source = Reader::new(Cursor::new(black_box(input)));
    Token::scanner_from(source)
        .try_fold(0usize, |count, result| result.map(|_| count + 1))
        .expect("the benchmark input is valid")
}

fn measure(name: &str, bytes: usize, mut scan: impl FnMut() -> usize) {
    const ITERATIONS: u32 = 50;

    let expected = black_box(scan());
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        assert_eq!(black_box(scan()), expected);
    }
    let elapsed = start.elapsed();
    let total_bytes = bytes as f64 * f64::from(ITERATIONS);
    let mib_per_second = total_bytes / elapsed.as_secs_f64() / (1024.0 * 1024.0);
    let per_iteration = elapsed / ITERATIONS;
    println!("{name}: {mib_per_second:.2} MiB/s, {per_iteration:?} per iteration");
}

fn main() {
    let input = input();
    measure("slice", input.len(), || scan_slice(input.as_str()));
    measure("reader", input.len(), || scan_reader(input.as_bytes()));
}
