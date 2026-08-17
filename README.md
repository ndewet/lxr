# lxr

A lexer generator for Rust. Write the tokens of a language as an enum, and the
derive macro builds the automaton that reads them.

The macro does the work at compile time. It parses each pattern, it builds one
deterministic automaton of each rule together, and it emits that automaton as
tables in the read only data of the program. Thus a scan makes no allocation, it
reads each byte one time, and the crate of the author compiles no regex engine.

## Install

lxr is not on crates.io yet. Take it from the repository:

```toml
[dependencies]
lxr = { git = "https://github.com/ndewet/lxr" }
```

## Use

```rust
use lxr::Lexer;

#[derive(Debug, PartialEq, Lexer)]
#[lxr(skip = r"[ \t\n]+")]
enum Token {
    #[lxr(token = "let")]
    Let,
    #[lxr(token = "=")]
    Assign,
    #[lxr(regex = "[0-9]+")]
    Number(u64),
    #[lxr(regex = "[a-z][a-z0-9]*")]
    Name(String),
}

fn main() {
    for found in Token::scan("let width = 80").located() {
        let found = found.expect("each character of the input belongs to a token");

        println!("{:?} at line {}, column {}", found.token, found.line, found.column);
    }
}
```

```text
Let at line 1, column 1
Name("width") at line 1, column 5
Assign at line 1, column 11
Number(80) at line 1, column 13
```

## What it gives

- **A token that carries its value.** A variant of one field takes that field
  from the text of the match, thus `Number(u64)` holds the number.
- **The place of each token.** A `Located` holds the token, the span, the line,
  and the column. The span counts bytes, and the column counts characters.
- **A start condition.** A string and a comment hold bytes that mean something
  else in code. A condition gives each one its own set of rules, and a rule
  moves the scan between them.
- **A scan that does not stop at the first fault.** The scan reports each
  character that no rule matches, then it reads the input after it. Thus one
  pass finds each fault.
- **A report at compile time.** A pattern that the parser cannot read, a rule
  that matches the empty string, and a rule that can never win a match each
  mark the attribute that the author wrote.

## The rules

| Option | Where | What it does |
| --- | --- | --- |
| `token = "fn"` | A variant | Matches the literal. A regex character needs no escape. |
| `regex = "[a-z]+"` | A variant | Matches the regular expression. |
| `skip = "[ \t]+"` | The enum | Reads the match, and gives no token. |
| `in = [Context::Text]` | A rule | Names the start conditions of the rule. |
| `go = Context::Code` | A rule | Changes the start condition after the match. |
| `condition = Context::Code` | The enum | Names the condition at which the scan begins. |

The longest match wins, and the earliest rule wins a tie. Thus a keyword goes
before the rule of a name.

The module `lxr::syntax` holds the reference: each attribute, the pattern
language, the constructions that lxr rejects, and each limit.

## The examples

`lxr/examples/` holds one example for each part of the macro. Run one with
`cargo run -p lxr --example <name>`.

| Example | What it shows |
| --- | --- |
| `tokens` | The parts of a lexer, and a token that carries its value. |
| `json` | One regular expression that reads a whole string of JSON. |
| `conditions` | A comment and a string, with start conditions. |
| `errors` | A report of each fault, in the manner of a compiler. |

## The crates

| Crate | What it holds |
| --- | --- |
| `lxr` | The runtime, and the re-export of the macro. A user crate holds this one. |
| `lxr-derive` | The derive macro. It reads the attributes with `syn`. |
| `lxr-codegen` | The regex parser, the automata, the tables, and the emitter. It builds for the host. |

`lxr` holds no dependency of its own.

## Contributing

`CONTRIBUTING.md` holds the error handling standard, and
`.github/writing-standard.md` holds the language rules. `CLAUDE.md` holds the
code layout, the shape of a commit, and each command that CI runs.
