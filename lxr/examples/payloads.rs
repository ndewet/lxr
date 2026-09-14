//! Shows automatic and custom token payload conversion.

use lxr::{Lexer, ScanError, Span};

#[derive(Debug, PartialEq, Lexer)]
#[lxr(skip = r"\s+")]
enum Token {
    #[lxr("[0-9]+")]
    Integer(u64),
    #[lxr("![a-z]+", with = strip_bang)]
    Shouted(String),
    #[lxr("[a-z]+", with = short_word)]
    Short(String),
    #[lxr(r"\?[a-z]+", with = reject_question)]
    Checked(String),
}

/// A converter that cannot fail returns the payload.
fn strip_bang(text: &str) -> String {
    text[1..].to_uppercase()
}

/// A converter returns `None` to reject the lexeme.
fn short_word(text: &str) -> Option<String> {
    (text.len() <= 3).then(|| text.to_owned())
}

/// A converter returns `Err` to explain a rejection.
fn reject_question(text: &str) -> Result<String, String> {
    Err(format!("a question is not a value: {text}"))
}

fn main() {
    let scanned: Vec<_> = Token::scanner("42 !hello odd").collect();
    println!("{scanned:#?}");

    let failed: Vec<_> = Token::scanner("18446744073709551616 7").collect();
    assert!(matches!(
        failed.first(),
        Some(Err(ScanError::InvalidPayload { span, .. })) if span == &Span::new(0, 20)
    ));

    let rejected: Vec<_> = Token::scanner("toolongword ?why").collect();
    assert!(matches!(
        rejected.first(),
        Some(Err(ScanError::InvalidPayload { .. }))
    ));
    println!("{failed:#?}");
    println!("{rejected:#?}");
}
