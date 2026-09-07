# lxr

A lexer generator for Rust, written to learn the theory. The repository holds
the front of that generator: a regular expression becomes a syntax tree, and
the syntax tree becomes an automaton.

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

`lxr` and `lxr-derive` hold no code. `lxr` is the runtime of a generated lexer,
and `lxr-derive` is the derive macro that emits one.

## What comes next

1. The subset construction. A nondeterministic automaton becomes a
   deterministic automaton.
2. The minimization. The states that read the same input become one state.
3. The emitter. A deterministic automaton becomes the source of a matcher, and
   the two empty crates get their code.

## Build

With Nix, enter the pinned development environment first:

```
nix develop
```

The shell provides stable Rust, Cargo, rustfmt, Clippy, rust-analyzer, and the
Rust standard-library sources.

```
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

On Windows, put the target directory outside the project. A build in
`./target` fails with os error 4551.

```
export CARGO_TARGET_DIR="$TEMP/lxr-target"
```
