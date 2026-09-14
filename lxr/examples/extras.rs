//! Shows caller state that each lexer action reads and writes.

use lxr::{Lexer, ScanError};
use std::collections::HashMap;

/// The state that the lexer keeps for the whole scan.
#[derive(Default)]
struct Symbols {
    names: HashMap<String, usize>,
    depth: usize,
    deepest: usize,
}

impl Symbols {
    fn intern(&mut self, name: &str) -> usize {
        let next = self.names.len();
        *self.names.entry(name.to_owned()).or_insert(next)
    }
}

#[derive(Debug, PartialEq, Lexer)]
#[lxr(extras = Symbols)]
#[lxr(skip = r"[ \n]+")]
enum Token {
    #[lxr("[a-z]+", with_extras = intern)]
    Identifier(usize),
    #[lxr(r"\(", with_extras = open)]
    Open,
    #[lxr(r"\)", with_extras = close)]
    Close,
}

/// A converter for a payload variant returns the payload.
fn intern(text: &str, symbols: &mut Symbols) -> usize {
    symbols.intern(text)
}

/// A converter for a unit variant returns `()`, thus it only changes state.
fn open(_: &str, symbols: &mut Symbols) {
    symbols.depth += 1;
    symbols.deepest = symbols.deepest.max(symbols.depth);
}

/// A converter returns a `Result` to reject the lexeme.
fn close(_: &str, symbols: &mut Symbols) -> Result<(), &'static str> {
    symbols.depth = symbols.depth.checked_sub(1).ok_or("unbalanced bracket")?;
    Ok(())
}

fn main() {
    let mut scanner = Token::scanner("one (two (one)) three");
    let scanned: Vec<_> = scanner.by_ref().collect();
    println!("{scanned:#?}");

    let symbols = scanner.into_extras();
    assert_eq!(symbols.names.len(), 3, "one, two, and three");
    assert_eq!(symbols.deepest, 2);
    assert_eq!(symbols.depth, 0);
    println!("interned {} names", symbols.names.len());
    println!("deepest bracket {}", symbols.deepest);

    let unbalanced: Vec<_> = Token::scanner(")").collect();
    assert!(matches!(
        unbalanced.first(),
        Some(Err(ScanError::InvalidPayload { .. }))
    ));
    println!("{unbalanced:#?}");
}
