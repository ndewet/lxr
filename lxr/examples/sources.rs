//! Shows in-memory bytes and standard readers as lexer input.

use lxr::{Lexer, Reader, Slice};
use std::io::Cursor;

#[derive(Debug, PartialEq, Lexer)]
#[lxr(skip = r"\s+")]
enum Token {
    #[lxr("[a-z]+")]
    Word,
}

fn main() {
    let bytes = Slice::new(b"from bytes");
    let from_bytes: Vec<_> = Token::scanner_from(bytes).collect();
    println!("byte slice: {from_bytes:#?}");

    let stream = Reader::new(Cursor::new(b"from reader"));
    let from_reader: Vec<_> = Token::scanner_from(stream).collect();
    println!("reader: {from_reader:#?}");
}
