# lxr

A lexer generator for Rust, written to learn the theory. A regular expression
becomes an automaton, and the derive macro emits a matcher for it.

## Core example

```rust
use lxr::{Lexer, Span, Spanned};

#[derive(Debug, PartialEq, Lexer)]
#[lxr(skip = r"[ \t\r\n]+")]
enum Token {
    #[lxr("[a-z]+")]
    Identifier(String),
    #[lxr("[0-9]+")]
    Integer(u64),
}

let scanned: Vec<_> = Token::scanner("name 42").collect();
assert_eq!(scanned[0], Ok(Spanned { token: Token::Identifier("name".into()), span: Span::new(0, 4) }));
assert_eq!(scanned[1], Ok(Spanned { token: Token::Integer(42), span: Span::new(5, 7) }));
```

Each variant has one `#[lxr("pattern")]` attribute. A variant may be unit or
contain one owned tuple payload. Payloads use their `FromStr` implementation,
so `String`, numeric types, and user types that implement `FromStr` work.

Add `with = path` to name a converter function. The converter receives the
matched lexeme. It returns the payload when a conversion cannot fail. It
returns a `Result` of the payload when a conversion can fail. An `Err`
gives `ScanError::InvalidPayload`, and the `Display` of the `Err` value
becomes the message of that error. Only a variant with a payload accepts a
converter.

```rust
#[lxr("![a-z]+", with = strip_bang)]
Shouted(String),

fn strip_bang(text: &str) -> String {
    text[1..].to_uppercase()
}
```

`Token::scanner(input)` returns `Result<Spanned<Token>, ScanError>` items.
`Spanned` records the matched token's UTF-8 byte range as a `Span`. Call
`span.text(input)` to get the lexeme, and `span.range()` for an index of
type `usize`. A `Span` holds two `u64` offsets, because a later streaming
source can be longer than `usize`. An unrecognized
character produces `ScanError` for that character and scanning continues.
`Token::scan(input)` is a convenience method that returns the first token and
the number of bytes consumed before it, including preceding trivia.

Rules use longest-match semantics. Declaration order resolves equal-length
matches.

## Input sources

The scanner consumes a `Source`, a small trait that fills a byte buffer and has
an associated error type. It does not require seeking, token-boundary-aware
chunks, or access to the source's complete contents. The scanner owns the
lookahead needed for longest-match selection and records absolute byte spans.

`Token::scanner(&str)` remains the convenient in-memory entry point. Use
`Slice` for a byte slice and `Reader` for files, network streams, and other
standard `Read` implementations:

```rust
use lxr::{Lexer, Reader};
use std::fs::File;

# #[derive(Lexer)]
# enum Token { #[lxr("x")] X }
let file = File::open("input.txt")?;
for item in Token::scanner_from(Reader::new(file)) {
    let token = item?;
    println!("{:?}", token.span);
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

Implement `Source` directly when input comes from another storage model. Its
error becomes the type parameter of `ScanError<E>`. Input is always interpreted
as UTF-8; arbitrary byte sources receive a recoverable `InvalidUtf8` error for
invalid sequences. `Scanner::into_source` returns unread lookahead together
with the original source when a parser stops before end of input.

## More examples

The runnable examples are in [`lxr/examples`](lxr/examples).

- [`basic.rs`](lxr/examples/basic.rs) shows unit tokens, skips, `scan`, and
  longest-match selection.
- [`payloads.rs`](lxr/examples/payloads.rs) shows `FromStr` payloads and a
  custom payload converter.
- [`errors.rs`](lxr/examples/errors.rs) shows token spans and recovery after
  unrecognized input or a payload error.
- [`modes.rs`](lxr/examples/modes.rs) shows `modes`, `begin`, `push`, and
  `pop`, including nested comments and an unterminated mode error.
- [`patterns.rs`](lxr/examples/patterns.rs) shows literals, groups,
  alternation, character classes, escapes, repetition, and Unicode.
- [`sources.rs`](lxr/examples/sources.rs) shows byte slices and standard
  readers as input sources.

Run an example from the workspace root:

```text
cargo run -p lxr --example modes
```

## Build

```
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

On Windows, put the target directory outside the project. A build in
`./target` fails with os error 4551.

```
export CARGO_TARGET_DIR="$TEMP/lxr-target"
```
