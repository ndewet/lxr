//! Shows replay and diagnostic locations over buffered input.

use std::io::{BufReader, Cursor};

use lxr::{Lexer, Locate, Replay, ScanError, Tracking};

#[derive(Debug, PartialEq, Lexer)]
#[lxr(skip = r"[ \r\n]+")]
enum Token {
    #[lxr("[a-z]+")]
    Word(String),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = b"one\n@ two";
    let source = Replay::new(BufReader::with_capacity(2, Cursor::new(input)))?;
    let mut scanner = Token::from_bufread(source);

    // `next` borrows the scanner, thus `locate_span` stays available.
    let mut scanned = Vec::new();
    while let Some(item) = scanner.next() {
        match item {
            Ok(token) => {
                println!("{:?} at {:?}", token.token, token.span);
                scanned.push(Ok(token));
            }
            Err(ScanError::Unrecognized { span }) => {
                let location = scanner.locate_span(span)?.start;
                assert_eq!((location.line, location.column), (2, 1));
                println!(
                    "unrecognized input at {}:{}",
                    location.line, location.column
                );
                scanned.push(Err(ScanError::Unrecognized { span }));
            }
            Err(error) => return Err(error.into()),
        }
    }

    let source = Tracking::new(BufReader::with_capacity(1, &input[..]));
    let mut tracked = Token::from_bufread(source);
    assert_eq!(tracked.by_ref().collect::<Vec<_>>(), scanned);
    assert_eq!(tracked.locate(4)?.line, 2);
    Ok(())
}
