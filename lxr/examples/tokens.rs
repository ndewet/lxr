//! Reads a small language into tokens that carry their values.
//!
//! This example shows the parts that each lexer needs: a literal, a regular expression, a rule
//! that skips, and the place of each token.
//!
//! It also shows a token that carries a value. A variant that holds one field takes that field
//! from the text of the match. `Number(u64)` holds the number, and `Name(String)` holds the text.
//! A variant that holds no field carries nothing.
//!
//! Run it with `cargo run -p lxr --example tokens`.

use lxr::{Lexer, Located};

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
    Number(u64),
    #[lxr(regex = "[a-zA-Z_][a-zA-Z0-9_]*")]
    Name(String),
}

fn main() {
    let source = "let total = (price + 3) * 42";

    println!("{source}\n");
    println!("SPAN     TOKEN            PLACE  TEXT");

    for found in Token::scan(source).located() {
        match found {
            Ok(Located {
                token,
                span,
                line,
                column,
            }) => {
                let range = format!("{}..{}", span.start, span.end);
                let kind = format!("{token:?}");
                let place = format!("{line}:{column}");

                println!("{range:<8} {kind:<16} {place:<6} {:?}", &source[span]);
            }
            Err(error) => println!("error: {error}"),
        }
    }
}
