//! The rules of a lexer, and the pattern language that a rule holds.
//!
//! This page holds the reference. [`Lexer`](crate::Lexer) holds the example that shows the shape of
//! a lexer.
//!
//! # The attributes
//!
//! Write one `#[lxr(...)]` attribute for each rule. A rule of a variant goes on that variant, and a
//! rule that skips goes on the enum.
//!
//! | Option | Where | What it does |
//! | --- | --- | --- |
//! | `token = "fn"` | A variant | Matches the literal. A regex character in it needs no escape. |
//! | `regex = "[a-z]+"` | A variant | Matches the regular expression. |
//! | `skip = "[ \t]+"` | The enum | Reads the match, and gives no token. |
//! | `in = [Context::Text]` | A rule | Names the start conditions of the rule. |
//! | `go = Context::Code` | A rule | Changes the start condition after the match. |
//! | `condition = Context::Code` | The enum | Names the condition at which the scan begins. |
//!
//! One start condition needs no list, thus `in = Context::Text` gives the same rule as
//! `in = [Context::Text]`.
//!
//! Each attribute names one option or more, and it names each option one time. Each variant of the
//! enum holds a rule, and the enum holds no generic parameter.
//!
//! # The value of a token
//!
//! A variant that holds one unnamed field carries a value. The field takes the text of the match
//! through [`FromStr`](std::str::FromStr), thus `Name(String)` holds the text and `Int(u64)` holds
//! the number. A text that the field does not hold gives a
//! [`ScanError`](crate::ScanErrorKind::Value), and the scan reads on.
//!
//! A variant that holds no field carries nothing.
//!
//! # The sequence of the rules
//!
//! The longest match wins. The earliest rule wins a tie of the same length, and a rule of a variant
//! comes before a rule that skips.
//!
//! Thus a keyword goes before the rule of a name:
//!
//! | Rules | Input | Token |
//! | --- | --- | --- |
//! | `token = "let"`, then `regex = "[a-z]+"` | `let` | The keyword. The earliest rule wins. |
//! | `token = "let"`, then `regex = "[a-z]+"` | `letter` | The name. The longest match wins. |
//! | `regex = "[a-z]+"`, then `token = "let"` | `let` | The name. The keyword can never win. |
//!
//! A rule that can never win a match is a fault, and the macro reports it. Thus the third row of the
//! table does not build.
//!
//! # The start conditions
//!
//! A start condition gives one set of rules its own state. A string and a comment hold bytes that
//! mean something else in code, thus each one needs its own condition.
//!
//! **Caution:** a rule with no `in` is applicable under the first condition alone. Thus a rule that
//! skips a space does not skip a space inside a string. Name each condition that the rule belongs
//! to:
//!
//! ```text
//! #[lxr(skip = "[ \t]+", in = [Context::Code, Context::Text])]
//! ```
//!
//! Only a rule changes the condition. The scan begins at the condition of `condition`, and `go`
//! moves it after a match. The end of the input closes nothing, thus a string that no rule leaves
//! reads to the end and gives no fault. Read [`Scan::condition`](crate::Scan::condition) after the
//! scan to find such an input.
//!
//! Each of `in`, `go`, and `condition` names the type of the conditions before the name of the
//! condition. Write `Context::Text`, and not `Text` after a `use`. Two spellings of one condition
//! read as two conditions, thus the macro rejects the second form.
//!
//! # The syntax of a regular expression
//!
//! | Pattern | What it matches |
//! | --- | --- |
//! | `a` | The character. Each character that no row below names is a literal. |
//! | `.` | Each character except a newline. |
//! | `[abc]` | One character of the set. |
//! | `[a-z]` | One character of the range. |
//! | `[^a-z]` | One character that the set does not hold, a newline included. |
//! | `x\|y` | `x` or `y`. |
//! | `(x)` | `x`. The group holds no capture. |
//! | `x*` | `x` zero times or more. |
//! | `x+` | `x` one time or more. |
//! | `x?` | `x` zero times or one time. |
//! | `x{3}` | `x` three times. |
//! | `x{3,}` | `x` three times or more. |
//! | `x{3,5}` | `x` from three to five times. |
//!
//! ## The escapes
//!
//! | Escape | What it matches |
//! | --- | --- |
//! | `\d`, `\D` | `[0-9]`, and each character outside it. |
//! | `\w`, `\W` | `[0-9A-Za-z_]`, and each character outside it. |
//! | `\s`, `\S` | `[ \t\n\v\f\r]`, and each character outside it. |
//! | `\n`, `\t`, `\r`, `\f`, `\v`, `\a` | The control character. |
//! | `\x41`, `\x{1F600}` | The codepoint. |
//! | `\.`, `\*`, `\\` | The character itself. A backslash before a mark gives that mark. |
//!
//! `.` does not match a newline, and a negated class does. Write `[\s\S]` for each character.
//!
//! ## Inside a class
//!
//! A mark inside a class is a literal, thus `[.*+]` holds three marks. Three places need care:
//!
//! - `]` is a literal at the start of the class. Write `[]]` or `[\]]`.
//! - `-` is a literal at the start and at the end. Write `[-a]` or `[a-]`.
//! - A class escape is not an end of a range. Write `[\d-z]` for the digits, `-`, and `z`.
//!
//! # What lxr does not hold
//!
//! Each pattern below gives an error, and the error names the correction. lxr does not read one of
//! them as a literal.
//!
//! | Pattern | The reason |
//! | --- | --- |
//! | `^a`, `a$` | An anchor. A rule matches at the point of the scan. Write `\^` for the mark. |
//! | `(?:a)`, `(?i)a` | A group modifier. A group of lxr holds no capture already. |
//! | `[[:alpha:]]` | A POSIX class. Write the class, or `\w`. |
//! | `\012` | An octal escape. Write `\x0A`. |
//! | `(a)\1` | A backreference. One automaton cannot hold one. |
//! | `a*?` | A lazy quantifier. The longest match wins. |
//! | `a**`, `a*{3}` | Two quantifiers on one atom. Write `a*`. |
//! | `a*` as a whole rule | A rule that matches the empty string. The scan makes no progress. |
//!
//! # The limits
//!
//! | The limit | The maximum |
//! | --- | --- |
//! | A repetition count | 65535 |
//! | The depth of the groups | 250 |
//! | The nodes of one pattern | 100000 |
//! | The rules of a lexer | 65535 |
//! | The states of the automaton | 65535 |
//!
//! A repetition makes one copy of the expression for each count. Thus a large count of a large group
//! reaches the limit of the nodes, and the macro reports it.
