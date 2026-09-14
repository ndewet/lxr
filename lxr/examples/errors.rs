//! Shows recoverable scan errors and UTF-8 byte spans.

use lxr::{Lexer, ScanError, Spanned};

#[derive(Debug, PartialEq, Lexer)]
#[lxr(skip = r"\s+")]
enum Token {
    #[lxr("[a-z]+")]
    Word,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = "one @ two";
    let scanned: Vec<_> = Token::scanner(input).collect();

    assert_eq!(
        scanned,
        vec![
            Ok(Spanned {
                token: Token::Word,
                span: 0..3,
            }),
            Err(ScanError::Unrecognized { span: 4..5 }),
            Ok(Spanned {
                token: Token::Word,
                span: 6..9,
            }),
        ],
    );

    for item in scanned {
        match item {
            Ok(spanned) => {
                let start = usize::try_from(spanned.span.start)?;
                let end = usize::try_from(spanned.span.end)?;
                println!("{:?}: {:?}", &input[start..end], spanned.token);
            }
            Err(error) => println!("scan error: {error:?}"),
        }
    }
    Ok(())
}
