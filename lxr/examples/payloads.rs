//! Shows automatic and custom token payload conversion.

use lxr::{Lexer, ScanError};
use std::convert::Infallible;

#[derive(Debug, PartialEq, Lexer)]
#[lxr(skip = r"\s+")]
enum Token {
    #[lxr("[0-9]+")]
    Integer(u64),
    #[lxr("![a-z]+", strip_bang)]
    Shouted(String),
}

fn strip_bang(text: &str) -> Result<String, Infallible> {
    Ok(text[1..].to_uppercase())
}

fn main() {
    let scanned: Vec<_> = Token::scanner("42 !hello").collect();
    println!("{scanned:#?}");

    let failed: Vec<_> = Token::scanner("18446744073709551616 7").collect();
    assert!(matches!(
        failed.first(),
        Some(Err(ScanError::InvalidPayload { span, .. })) if span == &(0..20)
    ));
    println!("{failed:#?}");
}
