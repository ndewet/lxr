# lxr

A lexer generator for Rust, written to learn the theory. A regular expression
becomes an automaton, and the derive macro emits a matcher for it.

## Use

```rust
use lxr::Lexer;

#[derive(Debug, PartialEq, Lexer)]
enum Token {
    #[lxr("[a-z]+")]
    Identifier,
    #[lxr("[0-9]+")]
    Integer,
}

assert_eq!(Token::scan("name42"), Some((Token::Identifier, 4)));
```

Each unit variant has one `#[lxr("pattern")]` attribute. `scan` returns the
longest matching prefix and its UTF-8 byte length; declaration order resolves
equal-length matches.

## What is here

`lxr-codegen` holds each part that exists.

- `regex` parses a pattern into an `Expression`. A `Quantifier` controls a
  repetition, and a `CharSet` specifies a leaf.
- `automata` holds finite automata and their shared vocabulary. A transition
  label is generic, thus an automaton knows no lexer concept.
- `automata::nfa` holds `Nfa`, its `Builder`, `Execution`, and `Matcher`.
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
