# Contributing to lxr

This file holds the error handling standard of the project. Obey it in each
change.

Two other files hold the remainder of the rules:

- `.github/writing-standard.md` holds the language rules. They apply to each
  message, to each doc comment, and to each commit message.
- `CLAUDE.md` holds the code layout, the shape of a commit, and the shape of a
  pull request.

## Error handling

### Why the standard exists

lxr is a lexer generator. The rules, the automaton, and the tables are built at
compile time, inside a derive macro. A panic in a proc macro gives
`custom derive panicked` and no span. The lexer author then sees no attribute,
no pattern, and no reason.

Thus a check on what the author wrote must give a value. The macro turns that
value into a `compile_error!` at the span of the rule.

A panic stays correct for one purpose. A panic reports a defect in lxr.

### Rule 0: the boundary

Data that a lexer author writes is input. A pattern, a rule, and a token
attribute are input. Each check on input gives a `Result`.

Data that lxr computes is trusted. A `StateId`, an arena offset, and the length
of a byte sequence are trusted. A fault there is a defect in lxr, thus it
panics.

The same value is input or contract, and the boundary decides which. Compare
these two:

```rust
// Contract. The caller writes both ends, thus an inverted range is a defect
// in the calling code.
CharSet::range('z', 'a')     // panics

// Input. The author writes the pattern, thus an inverted range is a fault in
// the input.
"[z-a]".parse::<Node>()      // gives ParseErrorKind::InvertedRange
```

### Tier 1: give a `Result`

Give a `Result` for input, and for resource exhaustion. A large lexicon is not
a defect. It asks for more than an automaton holds, thus `compile` reports it.

### Tier 2: panic

Use `panic!`, `assert!`, or `expect` for a documented precondition of an
internal API that only lxr calls. State what failed, and give the actual
values.

```rust
assert!(
    from.index() < self.accepts.len(),
    "cannot add a transition at {}: no such state",
    from.index()
);
```

A `Result` function still panics for a defect. `NfaBuilder::build` gives an
`Overflow` for a full state arena, and it panics for a transition that points
outside the state arena. The first is a limit. The second is a defect.

### Tier 3: use `debug_assert!`

Use `debug_assert!` for an invariant that only lxr can break, and whose check
costs time in a loop that runs one time for each symbol or for each state.

The epsilon closure checks each seed. `step` calls the closure one time for
each symbol, thus the check is a `debug_assert!`. A release build still panics,
because the index into the scratch space is checked. Only the message is worse.

### Tier 4: never

- `unwrap` outside a test and a doctest.
- `todo!` or `unimplemented!` in a released crate.
- A panic in `Display`, `Debug`, `Drop`, or `Ord`.
- An `assert!` with no message.
- An `expect` on a value that an author supplied.

### The shape of an error type

Give one error type to one operation. The type is a struct that holds the
context. The struct holds a kind enum if the operation has more than one cause.
Give `#[non_exhaustive]` to the struct and to the enum.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct BuildError {
    /// The index of the rule at fault. A fault of the whole lexicon gives
    /// `None`.
    pub rule: Option<usize>,
    /// The kind of the failure.
    pub kind: BuildErrorKind,
}
```

Rules:

- Derive `Debug`, `Clone`, `PartialEq`, and `Eq`. Implement `Display` and
  `std::error::Error`.
- Write each `impl` by hand. The crate has no dependencies, and it keeps none.
- Embed the fields of a cause. Do not box a cause, and do not hold the error
  type of another crate. Thus each error stays small, `Clone`, and free of a
  version lock. `BuildErrorKind::TooLarge` embeds the part and the maximum of
  an `Overflow`.
- Give the context that the caller needs to point at the fault. `ParseError`
  gives a position in the pattern. `BuildError` gives the index of a rule.

### The text of a message

- A `Display` of an error is a lowercase fragment, and it has no full stop. The
  caller puts it in a larger message.
- A panic message is a full sentence.
- Both obey `.github/writing-standard.md`.

```rust
// Display of an error.
"a rule needs at least one start condition"

// Panic message.
"start 3 points at 9, outside an arena of 4 states"
```

### The documentation duties

- Write an `# Errors` section for each fallible public function. Name each kind
  that the function returns.
- Write a `# Panics` section for each panic that a caller can reach. Start it
  with "This function panics if". Give the reason that makes the condition a
  defect.
- Write no `# Panics` section for a panic that no caller can reach. Put the
  reason in the `expect` message.

### The lints

`Cargo.toml` holds the lints that mechanise this standard. CI treats each
warning as a failure.

| Lint | Purpose |
| --- | --- |
| `missing_docs` | Each public item has a doc comment. |
| `unsafe_code` | The crate holds no `unsafe` block. |
| `clippy::missing_errors_doc` | Each fallible function has an `# Errors` section. |
| `clippy::missing_panics_doc` | Each reachable panic has a `# Panics` section. |
| `clippy::unwrap_used` | No `unwrap` outside a test. |
| `clippy::todo` | No `todo!` in the crate. |
| `clippy::unimplemented` | No `unimplemented!` in the crate. |

`clippy::panic_in_result_fn` stays off on purpose. A function that gives a
`Result` for a fault in the input still panics for a defect in lxr. The lint
rejects that pattern, thus it disagrees with Tier 2.

Add an `#[allow]` only with a `reason`, and only at the item that needs it.

### What the derive macro needs

Keep these properties as the macro lands:

1. Each check on a rule gives a `BuildError`, and the error names the rule.
   Thus the macro puts the `compile_error!` at the span of that rule.
2. `ParseError` gives a byte offset into the pattern. Thus the macro puts the
   `compile_error!` inside the string of the attribute.
3. An error is `Clone` and holds no borrow. Thus the macro collects each error
   of each rule, then it reports them together.
