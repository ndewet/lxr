# runeweaver

[![CI](https://github.com/ndewet/runeweaver/actions/workflows/ci.yml/badge.svg)](https://github.com/ndewet/runeweaver/actions/workflows/ci.yml)

`runeweaver` is a compile-time lexer generator for Rust. Describe tokens with regular
expressions on an enum, derive `Lexer`, and receive a typed scanner backed by a
generated deterministic finite automaton.

- Longest-match token selection with declaration-order tie breaking
- UTF-8 input, Unicode literals and character classes, and byte-accurate spans
- In-memory and streaming input through one scanner API
- Owned token payloads through `FromStr` or custom converters
- Lexical modes for strings, interpolation, and nested comments
- Recoverable errors for unrecognized input, invalid UTF-8, and invalid payloads
- No runtime regular-expression engine

## Installation

Packages are available on crates.io:

- [`runeweaver`](https://crates.io/crates/runeweaver)
- [`runeweaver-derive`](https://crates.io/crates/runeweaver-derive)
- [`runeweaver-codegen`](https://crates.io/crates/runeweaver-codegen)

Add `runeweaver` to your manifest:

```toml
[dependencies]
runeweaver = "0.1"
```

`runeweaver` requires Rust 1.85 or newer.

## Quick start

```rust
use runeweaver::{Lexer, Span, Spanned};

#[derive(Debug, PartialEq, Lexer)]
#[lexer(skip = r"[ \t\r\n]+")]
enum Token {
    #[token("[A-Za-z_][A-Za-z0-9_]*")]
    Identifier(String),

    #[token("[0-9]+")]
    Integer(u64),

    #[token("==")]
    Equal,
}

let tokens = Token::scanner("answer == 42")
    .collect::<Result<Vec<_>, _>>()
    .expect("the input is valid");

assert_eq!(
    tokens,
    vec![
        Spanned {
            token: Token::Identifier("answer".into()),
            span: Span::new(0, 6),
        },
        Spanned {
            token: Token::Equal,
            span: Span::new(7, 9),
        },
        Spanned {
            token: Token::Integer(42),
            span: Span::new(10, 12),
        },
    ],
);
```

Each enum variant defines one token rule. Unit variants produce token kinds;
single-field tuple variants produce owned payloads. Struct variants, variants
with multiple fields, and borrowed payloads are rejected at compile time.

## Matching semantics

Rules are anchored at the scanner's current position. The rule that consumes
the most input wins. If several rules consume the same number of bytes, the
rule declared first wins.

This makes keywords and identifiers straightforward:

```rust
use runeweaver::Lexer;

#[derive(Lexer)]
enum Token {
    #[token("if")]
    If,

    #[token("[A-Za-z_][A-Za-z0-9_]*")]
    Identifier,
}
```

`if` becomes `Token::If`, while `if_else` becomes one `Token::Identifier`.
Patterns that can match an empty string are rejected because every lexer rule
must make progress. A rule that can never win because an earlier rule always
takes priority is also rejected.

## Pattern syntax

`runeweaver` implements the regular-language subset needed by lexers:

| Syntax | Meaning |
| --- | --- |
| `abc` | Literal text |
| `.` | Any Unicode scalar except newline |
| `[abc]` | One character from a class |
| `[a-z]` | One character from a range |
| `[^a-z]` | One character outside a class |
| <code>a&#124;b</code> | Alternation |
| `(ab)` | Grouping |
| `a?`, `a*`, `a+` | Optional, zero-or-more, and one-or-more |
| `a{3}`, `a{2,}`, `a{2,5}` | Counted repetition |
| `\n`, `\r`, `\t`, `\f`, `\v`, `\a` | Control-character escapes |
| `\x7f`, `\x{1F600}` | Hexadecimal scalar escapes |
| `\d`, `\w`, `\s` | ASCII digit, word, and whitespace classes |

Punctuation can be escaped to match it literally. Non-ASCII literals and
class ranges are supported directly. Shorthand classes are intentionally
ASCII-defined.

Anchors, look-around, backreferences, lazy quantifiers, capture modifiers,
POSIX character classes, and octal escapes are not supported. Lexer rules are
already anchored at the current position, and the generated matcher is a DFA,
so features that require capture or backtracking state are outside the pattern
model.

## Payloads and converters

By default, a tuple variant's field is parsed with `FromStr`:

```rust
use runeweaver::Lexer;

#[derive(Lexer)]
enum Token {
    #[token("[0-9]+")]
    Integer(u64),
}
```

Use `with` when a lexeme needs custom conversion. A converter receives the
matched text and may return the payload directly or return a `Result`:

```rust
use runeweaver::Lexer;

#[derive(Lexer)]
enum Token {
    #[token("#[0-9a-fA-F]{6}", with = parse_color)]
    Color(u32),
}

fn parse_color(text: &str) -> Result<u32, std::num::ParseIntError> {
    u32::from_str_radix(&text[1..], 16)
}
```

A failed `FromStr` or converter becomes `ScanError::InvalidPayload`. The
scanner consumes that lexeme, reports its span, and continues with subsequent
input.

## Spans and errors

`Token::scanner(input)` yields `Result<Spanned<Token>, ScanError>`. A `Span` is
a half-open range of absolute UTF-8 byte offsets:

```rust
use runeweaver::Span;

let span = Span::new(5, 7);
assert_eq!(span.text("name 42"), Some("42"));
assert_eq!(span.range(), Some(5..7));
```

Scanning distinguishes five input failures:

- `Unrecognized` for a valid UTF-8 character accepted by no rule
- `InvalidUtf8` for malformed byte input
- `InvalidPayload` when token conversion fails
- `UnterminatedMode` when input ends inside a pushed lexical mode
- `Source` when a streaming source cannot provide more input

Unrecognized input, invalid UTF-8, and payload failures are recoverable: the
scanner consumes the offending range and continues. A source failure ends the
iterator because the scanner cannot determine whether more bytes would extend
the current match.

`Token::scan(input)` is a convenience for requesting only the first token. It
returns the token and the total byte count consumed before it, including
leading skipped input.

## Streaming sources

The scanner accepts any `Source`. Use `&str` for in-memory text, `Slice` for an
arbitrary byte slice, and `Reader` for standard `Read` implementations:

```rust,no_run
use runeweaver::{Lexer, Reader};
use std::fs::File;

#[derive(Lexer)]
enum Token {
    #[token("[A-Za-z_][A-Za-z0-9_]*")]
    Identifier,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("input.txt")?;

    for item in Token::scanner_from(Reader::new(file)) {
        let token = item?;
        println!("{:?}", token.span);
    }

    Ok(())
}
```

A source does not need to seek or preserve token boundaries. The scanner owns
the lookahead needed for longest-match selection. If a parser stops early,
`Scanner::into_source` returns the unread lookahead followed by the original
source.

Streaming bounds the amount read from the underlying source at one time, but
the scanner retains the current candidate lexeme and its lookahead. Memory use
therefore scales with the longest candidate token, not only with the fixed read
chunk. Applications processing untrusted input should design token rules with
appropriate length bounds.

## Lexical modes

Modes enable different rules in different contexts. Rules can replace the
current mode with `begin`, push a nested mode with `push`, or return to the
previous mode with `pop`:

```rust
use runeweaver::Lexer;

#[derive(Lexer)]
#[lexer(mode = Comment)]
#[lexer(skip = r"/\*", push = Comment)]
#[lexer(skip = r"/\*", modes = Comment, push = Comment)]
#[lexer(skip = r"\*/", modes = Comment, pop)]
#[lexer(skip = r"[^*/]+|[*/]", modes = Comment)]
enum Token {
    #[token("[A-Za-z_][A-Za-z0-9_]*")]
    Identifier,
}
```

This lexer skips arbitrarily nested block comments. Reaching end of input with
a pushed mode still active produces `ScanError::UnterminatedMode` with the
mode's name and the range from its opening rule through end of input.

## Examples

The [`runeweaver/examples`](runeweaver/examples) directory contains runnable programs for:

- basic matching, skipped input, and rule priority;
- automatic and custom payload conversion;
- error reporting and recovery;
- lexical modes and nested comments;
- supported pattern constructs; and
- slices, readers, and custom input sources.

Run an example from the workspace root:

```console
cargo run -p runeweaver --example modes
```

## Stability and support

`runeweaver` is currently pre-1.0. It follows Cargo's compatibility conventions for
0.x releases: patch releases within the 0.1 series preserve the documented
public API, while a new minor release may include breaking changes. Deprecated
APIs will be called out in release notes when a practical migration path
exists.

The supported public surface is the `runeweaver` runtime crate and its re-exported
derive macro. `runeweaver-codegen` is an implementation-facing crate and does not
carry the same compatibility guarantee.

Rust 1.85 is the minimum supported Rust version. Raising the MSRV requires at
least a minor release. The latest release line receives bug fixes; there are no
separate long-term-support branches. Report defects and support requests
through the repository's GitHub issue tracker.

## Development

Run the same checks used by CI:

```console
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --workspace --release
```

Performance benchmarks are included for lexer generation and scanning:

```console
cargo bench --workspace
```

On Windows, put the target directory outside the project. A build in
`./target` fails with os error 4551.

```console
set CARGO_TARGET_DIR=%TEMP%\runeweaver-target
```

## Releasing

The dispatch-only `Release` GitHub Actions workflow publishes the three crates
in dependency order. It needs a crates.io API token in the
`CARGO_REGISTRY_TOKEN` repository secret.

After the initial `v0.1.0` release, the workflow chooses the next version from
conventional commits since the latest release tag. A breaking change (`!` or a
`BREAKING CHANGE:` footer) bumps the major version, `feat` bumps the minor
version, and `fix` bumps the patch version. Only the largest change is applied.

## Acknowledgements

Runeweaver's derive-based lexer declaration syntax was inspired by
[Logos](https://github.com/maciejhirsz/logos). Runeweaver is an independent
implementation with its own regex parser and automata pipeline, designed around
streaming input, lexical modes, owned payloads, and structured error recovery.
Runeweaver does not incorporate Logos source code.
