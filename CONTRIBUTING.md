# Contributing to lxr

This file holds the error handling standard. Obey it in each change.

Two other files hold the remainder of the rules:

- `.github/writing-standard.md` holds the language rules.
- `CLAUDE.md` holds the code layout, the commits, and the verification.

## Error handling

The rules and the automaton are built inside a derive macro. A panic there
gives `custom derive panicked` and no span, thus the lexer author sees no
reason. A check on what the author wrote must give a value that the macro can
place.

### Rule 0: the boundary

Data that a lexer author writes is input. A pattern, a rule, and a token
attribute are input. Each check on input gives a `Result`.

Data that lxr computes is trusted. A `StateId`, an arena offset, and the length
of a byte sequence are trusted. A fault there is a defect in lxr, thus it
panics.

The boundary decides, and not the value:

```rust
CharSet::range('z', 'a')     // panics. The caller wrote both ends.
"[z-a]".parse::<Node>()      // gives ParseErrorKind::InvertedRange.
```

### Tier 1: give a `Result`

Give a `Result` for input, and for resource exhaustion. A large lexicon reaches
a limit, and it shows no defect.

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
outside that arena.

### Tier 3: use `debug_assert!`

Use `debug_assert!` for an invariant that only lxr can break, and whose check
costs time in a loop that runs one time for each symbol or for each state. The
epsilon closure checks each seed that way.

### Tier 4: never

- `unwrap` outside a test and a doctest.
- `todo!` or `unimplemented!` in a released crate.
- A panic in `Display`, `Debug`, `Drop`, or `Ord`.
- An `assert!` with no message.
- An `expect` on a value that an author supplied.

### The shape of an error type

Give one error type to one operation. The type is a struct that holds the
context that the caller needs to point at the fault. `ParseError` gives the
bytes of the pattern at fault, and `BuildError` gives the index of a rule. The
struct holds a kind enum if the operation has more than one cause.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct BuildError {
    pub rule: Option<usize>,
    pub kind: BuildErrorKind,
}
```

- Give `#[non_exhaustive]` to the struct and to the enum.
- Derive `Debug`, `Clone`, `PartialEq`, and `Eq`. Implement `Display` and
  `std::error::Error` by hand. See the rule on the dependencies below.
- Embed the fields of a cause. Do not box a cause, and do not hold the error
  type of another crate.
- Give the span of the input at fault as a byte range, and not as one offset.
  A caller slices the input with the range, thus it can mark the whole
  construction. A character offset does not slice a pattern that holds a
  character of more than one byte.
- Give the kind a `help` method. It tells the author how to correct the input.

### The text of a message

A `Display` of an error is a lowercase fragment with no full stop, because the
caller puts it in a larger message. A panic message is a full sentence.

A `help` is one or two full sentences, and it gives the correction as a
command. Name a construction that the author wrote, and not a part of lxr. The
author knows patterns and rules, and not nodes, arenas, or fragments.

```
error: invalid range 'z-a' at position 1
 --> [z-a]
      ^^^
 help: Write the low end first, for example `a-z`.
```

### The documentation duties

- Write an `# Errors` section for each fallible public function. Name each kind
  that the function returns.
- Write a `# Panics` section for each panic that a caller can reach. Start it
  with "This function panics if".
- Write no `# Panics` section for a panic that no caller can reach. Put the
  reason in the `expect` message.
- Give no reason that the code already shows. State a tier one time, at the
  module, and not again at each function.

### The dependencies

`lxr` keeps no dependency. A user crate compiles it, thus each dependency
there costs each user a compile.

`lxr-codegen` and the derive crate build for the host at compile time. They
may hold `proc-macro2`, `quote`, and `syn`. Add no other dependency without a
reason.

An error type holds no error type of another crate, whichever crate declares
it. Embed the fields of a cause.

### The lints

The root `Cargo.toml` holds the lints that check this standard, in
`[workspace.lints]`. Each crate inherits them with `[lints] workspace = true`.
Add an `#[allow]` only with a `reason`, and only at the item that needs it.

`clippy::panic_in_result_fn` stays off on purpose. A function that gives a
`Result` for a fault in the input still panics for a defect in lxr, thus the
lint disagrees with Tier 2.

### What the derive macro needs

1. Each check on a rule gives a `BuildError` that names the rule. Thus the
   macro places its `compile_error!` at the span of that rule.
2. `ParseError` gives a byte range of the pattern. Thus the macro marks the
   construction at fault inside the string of the attribute.
3. An error is `Clone` and holds no borrow. Thus the macro collects each error,
   then it reports them together.
