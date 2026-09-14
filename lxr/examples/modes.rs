//! Shows start conditions and nested block comments.

use lxr::{Lexer, ScanError};

#[derive(Debug, PartialEq, Lexer)]
#[lxr(mode = Directive)]
#[lxr(mode = Comment)]
#[lxr(skip = r"\s+", modes = [INITIAL, Directive])]
#[lxr(skip = r"/\*", modes = [INITIAL, Directive], push = Comment)]
#[lxr(skip = r"/\*", modes = Comment, push = Comment)]
#[lxr(skip = r"\*/", modes = Comment, pop)]
#[lxr(skip = r"[^*/]+|[*/]", modes = Comment)]
enum Token {
    #[lxr("@", begin = Directive)]
    DirectiveStart,
    #[lxr(";", modes = Directive, begin = INITIAL)]
    DirectiveEnd,
    #[lxr("[a-z]+")]
    Word,
    #[lxr("[A-Z]+", modes = Directive)]
    Command,
}

fn main() {
    let input = "word @ RUN /* outer /* inner */ outer */ ; tail";
    let scanned: Vec<_> = Token::scanner(input).collect();
    println!("{scanned:#?}");

    let unterminated: Vec<_> = Token::scanner("word /* open").collect();
    assert!(matches!(
        unterminated.last(),
        Some(Err(ScanError::UnterminatedMode {
            mode: "Comment",
            ..
        }))
    ));
    println!("{unterminated:#?}");
}
