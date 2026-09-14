//! Shows regex building blocks that lxr supports.

use lxr::Lexer;

#[derive(Debug, PartialEq, Lexer)]
#[lxr(skip = r"\s+")]
enum Token {
    #[lxr(r"(true|false)")]
    Boolean,
    #[lxr(r"[A-Za-z_][A-Za-z0-9_]*")]
    Identifier,
    #[lxr(r"\d{1,3}(\.\d{1,3}){3}")]
    Address,
    #[lxr(r"\x{1F600}+")]
    Emoji,
    #[lxr(r"[^,\s]+")]
    Field,
    #[lxr(",")]
    Comma,
}

fn main() {
    let input = "true 127.0.0.1 😀😀 value,other";
    let scanned: Vec<_> = Token::scanner(input).collect();
    println!("{scanned:#?}");
}
