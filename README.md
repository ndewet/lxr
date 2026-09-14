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
matched lexeme. It returns the payload, an `Option` of the payload, or a
`Result` of the payload. A `None` and an `Err` each give
`ScanError::InvalidPayload`. A converter on a unit variant, or on a `skip`
rule, returns `()`, `Option<()>`, or `Result<(), E>`.

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

## Caller state

An action can read and write state that lives for the whole scan. Name the
state type with `#[lxr(extras = Type)]`, and name a converter with
`with_extras = path`. Such a converter receives the lexeme and the state.
Use this for a symbol table, an indentation stack, or a count of errors.

```rust
#[derive(Default)]
struct Counts { words: usize }

#[derive(Lexer)]
#[lxr(extras = Counts)]
enum Token {
    #[lxr("[a-z]+", with_extras = count)]
    Word(usize),
}

fn count(_: &str, counts: &mut Counts) -> usize {
    counts.words += 1;
    counts.words
}

let mut scanner = Token::scanner("one two");
let scanned: Vec<_> = scanner.by_ref().collect();
assert_eq!(scanner.into_extras().words, 2);
```

The state starts from its `Default`. Call `scanner.with_extras(state)` for a
state that has no `Default`, or for a state that starts with content. Call
`scanner.extras()` and `scanner.extras_mut()` during a scan, and
`scanner.into_extras()` after one.

## More examples

The runnable examples are in [`lxr/examples`](lxr/examples).

- [`basic.rs`](lxr/examples/basic.rs) shows unit tokens, skips, `scan`, and
  longest-match selection.
- [`payloads.rs`](lxr/examples/payloads.rs) shows `FromStr` payloads and a
  custom payload converter.
- [`extras.rs`](lxr/examples/extras.rs) shows caller state, with a symbol
  table and a bracket depth.
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
