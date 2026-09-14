# lxr

A lexer generator for Rust, written to learn the theory. A regular expression
becomes an automaton, and the derive macro emits a matcher for it.

## Use

```rust
use lxr::{Lexer, Spanned};

#[derive(Debug, PartialEq, Lexer)]
#[lxr(skip = r"[ \t\n]+")]
#[lxr(skip = r"//[^\n]*")]
enum Token {
    #[lxr("[a-z]+")]
    Identifier(String),
    #[lxr("[0-9]+")]
    Integer(u64),
}

let scanned: Vec<_> = Token::scanner("name // note\n42").collect();
assert_eq!(scanned[0], Ok(Spanned { token: Token::Identifier("name".into()), span: 0..4 }));
assert_eq!(scanned[1], Ok(Spanned { token: Token::Integer(42), span: 13..15 }));
```

Each variant has one `#[lxr("pattern")]` attribute. A variant may be unit or
contain one owned tuple payload. Payloads use their `FromStr` implementation,
so `String`, numeric types, and user types that implement `FromStr` work with
no converter. For custom conversion, add a second argument that returns
`Result<payload, error>`: `#[lxr("![a-z]+", strip_bang)]`.

Enum-level
`#[lxr(skip = "pattern")]` attributes consume whitespace, comments, or other
trivia before the next token. Rules use longest-match semantics; declaration
order resolves equal-length matches.

`Token::scanner(input)` returns `Result<Spanned<Token>, ScanError>` items.
`Spanned` records the matched token's UTF-8 byte range. An unrecognized
character produces `ScanError` for that character and scanning continues.
`Token::scan(input)` is a convenience method that returns the first token and
the number of bytes consumed before it, including preceding trivia.

Payload conversion failures are reported as `ScanError::InvalidPayload` with
the matched span, then scanning continues. `Spanned` remains useful when a
consumer also needs the original matched text.

## What is here

`lxr-codegen` holds each part that exists.

- `regex` parses a pattern into an `Expression`. A `Quantifier` controls a
  repetition, and a `CharSet` specifies a leaf.
- `automata` holds finite automata and their shared vocabulary. A transition
  label is generic, thus an automaton knows no lexer concept.
- `automata::nfa` holds the NFA and its builder; test-only execution helpers
  validate its matching semantics.
- `automata::nfa::thompson` is Thompson construction. It walks a syntax tree,
  and it gives one fragment for each operator.
- `automata::encoding` maps character sets to UTF-8 byte-range sequences.
  Thompson construction maps those sequences to NFA paths.
- `automata::dfa` holds deterministic automata. Its subset construction turns
  a reachable NFA state set into each DFA state, then minimization merges
  equivalent states.
- `lexer` models rules and start conditions. `emitter` renders a minimized
  byte-oriented DFA as the matcher method used by the derive macro.

`lxr` supplies the public `Lexer` trait and re-exports its derive macro.
`lxr-derive` parses token variants and wires the full code-generation pipeline.

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
