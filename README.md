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
`Spanned` records the matched token's UTF-8 byte range. An unrecognized
character produces `ScanError` for that character and scanning continues.
`Token::scan(input)` is a convenience method that returns the first token and
the number of bytes consumed before it, including preceding trivia.

Rules use longest-match semantics. Declaration order resolves equal-length
matches.

## Streaming input

Use `Token::from_reader(reader)` for a blocking `Read` source, including a
file. Use `Token::from_bufread(source)` for an existing `BufRead` source.
The scanner retains lexemes and lookahead across buffer boundaries.
The input need not support seeking.

```rust
use std::fs::File;

let file = File::open("input.txt")?;
let scanner = Token::from_reader(file);
for item in scanner {
    let token = item?;
    // Use the token.
}
```

`Span` holds two `u64` byte offsets relative to the initial position of
the scanner. Call `span.text(input)` to get the lexeme from an in-memory
string, and `span.range()` for an index of type `usize`.
Token payloads remain owned. The scanner validates UTF-8 across chunks.
It does not normalize line endings.

Lexical errors remain recoverable. I/O errors, invalid UTF-8, position
overflow, and resource limits terminate iteration after one error.
A terminal error discards a pending token. The scanner reports the
terminal error, and not the rule that it accepted before the failure.
This applies to each terminal error, and not only to an I/O error.
The scanner retries `Interrupted`; `WouldBlock` is terminal. This API
does not support async input or files that the caller wants to follow as
they grow.

`Limits` defaults to 8 MiB of retained input and 1024 mode frames.
Use `scanner.with_limits(limits)?` before scanning to change these bounds.
Retained input includes the lexeme, speculative input, and lookahead.
A limit error never changes longest-match selection.
The bounds exclude source buffers, token payloads, and line indexes.
Some rule sets require repeated scanning after rollback.
The API does not guarantee linear total execution time.

Use `scanner.into_remainder()` to recover unread input when scanning stops
early. The returned reader supplies retained lookahead before the source.

## Replay and diagnostic locations

Use `Replay` to enable lazy location lookup for a stable source that
implements `BufRead + Seek`. The source content must remain unchanged
while the adapter exists.

```rust
use lxr::{Locate, Replay, ScanError};
use std::{fs::File, io::BufReader};

let source = Replay::new(BufReader::new(File::open("input.txt")?))?;
let mut scanner = Token::from_bufread(source);

while let Some(item) = scanner.next() {
    if let Err(ScanError::Unrecognized { span }) = item {
        let location = scanner.locate_span(span)?.start;
        eprintln!("unrecognized input at {}:{}", location.line, location.column);
    }
}
```

Call `next` in a `while let` loop, and not in a `for` loop. A `for` loop
moves the scanner, thus `locate_span` is then unavailable.

Location lookup does not execute lexer actions.

`Token::scanner(&str)` supports lazy locations directly.
Normal scanning of a replayable source does no line tracking.
The first location request indexes line starts through the requested
offset. Later requests reuse that index and extend it when needed.
Lookup preserves the source position, including after an indexing error.
If position restoration fails, the replay adapter rejects later reads.

For a forward-only source, explicitly enable tracking:

```rust
use lxr::{Locate, Tracking};

let stdin = std::io::stdin();
let mut scanner = Token::from_bufread(Tracking::new(stdin.lock()));
while let Some(item) = scanner.next() {
    // Call scanner.locate_span(span) when a diagnostic needs it.
}
```

`Tracking` retains line starts as buffers become available. It can resolve
consumed positions after the corresponding text is discarded.
Both adapters retain one `u64` per indexed line, plus vector capacity.
They do not retain source text for diagnostic excerpts.

`Location` uses one-based line numbers and byte columns. LF starts a new
line. CR and tabs each occupy one byte column, so CRLF advances one line.
UTF-8 continuation bytes also have byte positions. EOF is a valid position.
`locate_span` resolves both endpoints of a half-open span.
Reversed spans and unavailable offsets produce errors.
Offsets carry no source identity; use the scanner that produced the span.
If an adapter is consumed before scanner construction, lookup preserves
that adapter's original line coordinates.

Run `cargo run -p lxr --example streaming` for a complete example.

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
