//! Reports each character that no rule matches, in the manner of a compiler.
//!
//! A scan does not stop at the first fault. It gives one [`ScanError`] for each character that it
//! cannot read, then it reads the input after that character. Thus one pass finds each fault.
//!
//! Run it with `cargo run -p lxr --example errors`.

use lxr::{Lexer, ScanError};

/// The tokens of a language of an assignment. It holds no rule for `?` and none for `¤`.
#[derive(Debug, PartialEq, Eq, Lexer)]
#[lxr(skip = r"[ \t\r\n]+")]
enum Token {
    #[lxr(token = "let")]
    Let,
    #[lxr(token = "=")]
    Assign,
    #[lxr(token = ";")]
    Semicolon,
    #[lxr(regex = "[0-9]+")]
    Number,
    #[lxr(regex = "[a-z]+")]
    Name,
}

const SOURCE: &str = "let width = 80;
let ¤ = 12;
let height = 4?0;
";

fn main() {
    let mut faults = 0;

    for found in Token::scan(SOURCE) {
        if let Err(error) = found {
            faults += 1;
            report(SOURCE, &error);
        }
    }

    println!("{faults} faults");
}

/// Writes `error` with the line that holds it, and a caret under the character at fault.
///
/// The column counts characters and not bytes, thus the caret lands under a character above ASCII.
/// A terminal moves a tab to the next tab stop, thus the indent keeps each tab of the line and it
/// writes one space for each other character.
fn report(source: &str, error: &ScanError) {
    let line = source
        .lines()
        .nth(error.line as usize - 1)
        .unwrap_or_default();
    let indent: String = line
        .chars()
        .take(error.column as usize - 1)
        .map(|character| if character == '\t' { '\t' } else { ' ' })
        .collect();

    println!("error: {error}");
    println!("   |");
    println!("   | {line}");
    println!(
        "   | {indent}^ the bytes {}..{}",
        error.span.start, error.span.end
    );
    println!("   |");
    println!("   = help: {}\n", error.help());
}
