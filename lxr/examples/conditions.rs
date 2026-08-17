//! Reads a language that holds a comment and a string, with start conditions.
//!
//! A comment and a string hold bytes that mean something else in code. A start condition gives
//! each of them its own set of rules, thus the same bytes give a different token.
//!
//! The lexer holds three conditions:
//!
//! - `Code` reads a word, a number, and the start of a comment or of a string.
//! - `Comment` reads the body of a block comment, and it gives no token.
//! - `Text` reads the body of a string.
//!
//! A block comment does not nest. The conditions hold no stack, thus the first `*/` ends the
//! comment.
//!
//! Run it with `cargo run -p lxr --example conditions`.

use lxr::Lexer;

/// The start conditions of the lexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Context {
    Code,
    Comment,
    Text,
}

/// The tokens of a language that holds a line comment, a block comment, and a string.
///
/// Each rule that reads a comment skips its match, thus a comment gives no token. The rules of the
/// `Comment` condition read the body one piece at a time. `[^*]+` reads each byte that cannot end
/// the comment, and `\*` reads a star that `*/` does not follow. The longest match gives `*/` the
/// win over `\*`.
#[derive(Debug, PartialEq, Eq, Lexer)]
#[lxr(condition = Context::Code)]
#[lxr(skip = r"[ \t\n]+")]
#[lxr(skip = "//[^\n]*")]
#[lxr(skip = r"/\*", go = Context::Comment)]
#[lxr(skip = r"\*/", in = [Context::Comment], go = Context::Code)]
#[lxr(skip = "[^*]+", in = [Context::Comment])]
#[lxr(skip = r"\*", in = [Context::Comment])]
enum Token {
    #[lxr(regex = "[a-z][a-z0-9]*")]
    Word,
    #[lxr(regex = "[0-9]+")]
    Number,
    #[lxr(token = "=")]
    Assign,
    #[lxr(token = ";")]
    Semicolon,
    #[lxr(token = "\"", go = Context::Text)]
    Quote,
    #[lxr(regex = r#"([^"\\]|\\.)+"#, in = [Context::Text])]
    Text,
    #[lxr(token = "\"", in = [Context::Text], go = Context::Code)]
    End,
}

const SOURCE: &str = r#"// the name of the package
name = "lxr";

/* the version
   holds three parts, and a * inside the comment ends nothing */
version = 11;

greeting = "a // is not a comment, and a /* is not one either";
"#;

fn main() {
    println!("{SOURCE}");

    for found in Token::scan(SOURCE).located() {
        match found {
            Ok(found) => {
                let kind = format!("{:?}", found.token);

                println!(
                    "line {:<3} {kind:<10} {:?}",
                    found.line, &SOURCE[found.span]
                );
            }
            Err(error) => println!("error: {error}"),
        }
    }
}
