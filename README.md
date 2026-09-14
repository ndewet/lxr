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
