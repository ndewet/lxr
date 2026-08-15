# Writing standard

All text in this repository obeys ASD-STE100 (Simplified Technical English)
and IEC/IEEE 82079-1:2019.

## Scope

The standard applies to:

- Doc comments (`///` and `//!`).
- Non-documentation comments (`//`).
- The text of a message, for example a `panic!`, an `assert!`, or an error
  `Display`.
- Commit messages, pull request titles, and pull request bodies.
- All other documentation.

The standard does not apply to code identifiers, to test names, or to the
existing code.

## ASD-STE100 rules

- Use approved words in their approved part of speech. One word has one
  meaning. One meaning has one word.
- Use the active voice. Give an instruction as a command.
- Use the present tense. Do not use a complex verb form.
- Write a maximum of 20 words in a procedural sentence.
- Write a maximum of 25 words in a descriptive sentence.
- Write one instruction in one sentence. Put one topic in one paragraph.
- Do not write a noun cluster of more than three words.
- Keep the articles and the other structure words. Do not remove them to make
  the text shorter.
- Do not use slang, jargon, humour, or figurative language.
- Do not use an em dash or an en dash. Write two sentences.

Technical names and technical verbs for this project stay permitted. Examples:
NFA, DFA, UTF-8, codepoint, regex, cargo, rustdoc, epsilon closure,
determinization, minimization, Thompson construction.

## IEC/IEEE 82079-1:2019 rules

- Give the purpose first. Then give the conditions for use. Then give the
  result.
- Put a warning or a caution before the step that it applies to.
- Make the information complete for the intended user and task.
- Use a list or numbered steps for a procedure. Put the steps in the sequence
  of the task.

## Doc comments

- Write a doc comment for each public item.
- Start with one summary sentence. Give the purpose, and not the
  implementation.
- Use the rustdoc sections in this sequence: `# Errors`, `# Panics`,
  `# Safety`, `# Examples`. A `# Panics` section and a `# Safety` section are
  warnings, thus they come before `# Examples`.
- Start each `# Panics` section with "This function panics if".
- Give an example for each public function. CI runs the examples as doctests.
- Keep the wrap width of the file. The files in `src/automata/` wrap near 100
  columns. The files in `src/regex/` and `src/compiler/` wrap near 80 columns.

## Non-documentation comments

Keep `//` comments to an absolute minimum.

- Prefer a clear name or a small function.
- Write a `//` comment only for a reason that the code cannot show: a
  non-obvious invariant, a reference to a specification, or a workaround for a
  known defect.
- Do not restate the code.
- Do not leave commented-out code, a history of the work, or a note to
  yourself.

## Terminology

Use one term for one concept:

| Use | Do not use |
| --- | --- |
| state arena | arena of states, the arena |
| start state (an automaton) | entry point, start |
| start condition (a lexer) | mode |
| label | edge condition, symbol set |
| execution (one scan in progress) | run, simulation, simulator |
| accept | acceptance, accepting state |
| character set | charset, except as the type name `CharSet` |
| byte sequence | byte string |
| lower (verb) | rewrite, translate, turn into |
| scan (verb) | run, walk, drive |
| identifier | id, ident |

## Commits and pull requests

- Keep the conventional prefix `type(scope):`. The prefix is a machine token,
  thus ASD-STE100 does not change it.
- Write the subject after the prefix as one command in the present tense.
  Example: `feat(regex): parse strings into regex nodes`.
- Write a maximum of 20 words in the subject. Put one topic in one commit.
- Write the body as short sentences or as a list.
- Give the reason for the change and the effect on the user. Do not give a
  history of the work.
- Merges to `main` are squash merges, thus a pull request title becomes the
  commit subject. Apply the same rules to a pull request title.
