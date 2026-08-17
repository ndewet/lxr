//! Reads a small language into tokens.
//!
//! This example shows the parts that each lexer needs: a literal, a regular expression, a rule
//! that skips, and the place of each token.
//!
//! Run it with `cargo run -p lxr --example tokens`.

use lxr::Lexer;

/// The tokens of a language of an assignment and an arithmetic expression.
///
/// The rules are in the sequence of precedence. `Let` comes before `Name`, thus `let` gives the
/// keyword. The longest match still wins, thus `letter` gives a name and not a keyword.
#[derive(Debug, PartialEq, Eq, Lexer)]
#[lxr(skip = r"[ \t\n]+")]
enum Token {
    #[lxr(token = "let")]
    Let,
    #[lxr(token = "=")]
    Assign,
    #[lxr(token = "+")]
    Plus,
    #[lxr(token = "*")]
    Star,
    #[lxr(token = "(")]
    Open,
    #[lxr(token = ")")]
    Close,
    #[lxr(regex = "[0-9]+")]
    Number,
    #[lxr(regex = "[a-zA-Z_][a-zA-Z0-9_]*")]
    Name,
}

fn main() {
    let source = "let total = (price + 3) * 42";

    println!("{source}\n");
    println!("SPAN     TOKEN      PLACE  TEXT");

    let mut scan = Token::scan(source);
    while let Some(found) = scan.next() {
        let span = scan.span();
        let range = format!("{}..{}", span.start, span.end);
        let place = format!("{}:{}", scan.line(), scan.column());
        let (kind, text) = match found {
            Ok(token) => (format!("{token:?}"), format!("{:?}", scan.slice())),
            Err(error) => ("error".to_owned(), error.to_string()),
        };

        println!("{range:<8} {kind:<10} {place:<6} {text}");
    }
}
