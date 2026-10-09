//! Shows basic token rules, skips, and rule selection.

use runeweaver::Lexer;

#[derive(Debug, PartialEq, Lexer)]
#[lexer(skip = r"[ \t\r\n]+")]
enum Token {
    #[token("let")]
    Let,
    #[token("[A-Za-z_][A-Za-z0-9_]*")]
    Identifier,
    #[token("[0-9]+")]
    Integer,
    #[token("=")]
    Equal,
}

fn main() {
    assert_eq!(Token::scan("let"), Some((Token::Let, 3)));
    assert_eq!(
        Token::scan("letter"),
        Some((Token::Identifier, 6)),
        "the longer identifier rule wins",
    );

    let tokens: Vec<_> = Token::scanner("let answer = 42").collect();
    println!("{tokens:#?}");
}
