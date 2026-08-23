//! Scans an input with a rule graph that this test writes by hand.
//!
//! The derive macro writes the graph of a lexer as code. This test writes that code directly, in
//! the shape that the macro gives. Thus it specifies the runtime and it needs no macro.

use lxr::{Lexer, Located, Outcome, Step};

/// A lexer of one condition. It reads a word of the letter `a`, and it skips a space and a newline.
///
/// | Node | Meaning |
/// | --- | --- |
/// | `root` | The start. |
/// | `word` | A run of the letter `a`. It gives rule 0. |
/// | `space` | A run of a space and a newline. It gives rule 1, which skips. |
/// | `fault` | No rule matched. |
mod words {
    use super::{Lexer, Outcome, Step};

    #[derive(Debug, PartialEq, Eq)]
    pub enum Token {
        Word,
    }

    impl Lexer for Token {
        type Condition = ();

        fn step(input: &str, at: usize, step: &mut Step<Self>) {
            fn root(input: &str, at: usize, index: usize, step: &mut Step<Token>) {
                let bytes = input.as_bytes();
                let Some(&byte) = bytes.get(index) else {
                    fault(at, index, step);
                    return;
                };
                match byte {
                    b'a' => word(input, at, index + 1, step),
                    b' ' | b'\n' => space(input, at, index + 1, step),
                    _ => fault(at, index, step),
                }
            }

            fn word(input: &str, at: usize, index: usize, step: &mut Step<Token>) {
                let bytes = input.as_bytes();
                let mut index = index;
                while let Some(&byte) = bytes.get(index) {
                    if byte == b'a' {
                        index += 1;
                    } else {
                        break;
                    }
                }
                step.outcome = Outcome::Token(Token::Word);
                step.length = index - at;
                step.read = index - at;
            }

            fn space(input: &str, at: usize, index: usize, step: &mut Step<Token>) {
                let bytes = input.as_bytes();
                let mut index = index;
                while let Some(&byte) = bytes.get(index) {
                    if (byte == b' ') | (byte == b'\n') {
                        index += 1;
                    } else {
                        break;
                    }
                }
                step.outcome = Outcome::Skip;
                step.length = index - at;
                step.read = index - at;
            }

            fn fault(at: usize, index: usize, step: &mut Step<Token>) {
                step.outcome = Outcome::None;
                step.length = 0;
                step.read = index - at;
            }

            root(input, at, at, step);
        }

        fn condition(_index: u16) {}
    }
}

/// A lexer of two conditions. A quote changes the condition, thus the same letters give a different
/// token inside a string.
///
/// | Node | Meaning |
/// | --- | --- |
/// | `code` | The start of the code condition. |
/// | `text` | The start of the text condition. |
/// | `word` | A word in code. It gives rule 3. |
/// | `body` | The text of a string. It gives rule 2. |
/// | `open` | A quote in code. It gives rule 0, which goes to text. |
/// | `close` | A quote in text. It gives rule 1, which goes to code. |
mod strings {
    use super::{Lexer, Outcome, Step};

    #[derive(Debug, PartialEq, Eq)]
    pub enum Token {
        Quote,
        Text,
        Word,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Context {
        Code,
        Text,
    }

    impl Lexer for Token {
        type Condition = Context;

        fn step(input: &str, at: usize, step: &mut Step<Self>) {
            fn code(input: &str, at: usize, index: usize, step: &mut Step<Token>) {
                let bytes = input.as_bytes();
                let Some(&byte) = bytes.get(index) else {
                    fault(at, index, step);
                    return;
                };
                match byte {
                    b'"' => open(at, index + 1, step),
                    b'a'..=b'z' => word(input, at, index + 1, step),
                    _ => fault(at, index, step),
                }
            }

            fn text(input: &str, at: usize, index: usize, step: &mut Step<Token>) {
                let bytes = input.as_bytes();
                let Some(&byte) = bytes.get(index) else {
                    fault(at, index, step);
                    return;
                };
                match byte {
                    b'"' => close(at, index + 1, step),
                    b'a'..=b'z' => body(input, at, index + 1, step),
                    _ => fault(at, index, step),
                }
            }

            fn word(input: &str, at: usize, index: usize, step: &mut Step<Token>) {
                let index = letters(input, index);
                step.outcome = Outcome::Token(Token::Word);
                step.length = index - at;
                step.read = index - at;
            }

            fn body(input: &str, at: usize, index: usize, step: &mut Step<Token>) {
                let index = letters(input, index);
                step.outcome = Outcome::Token(Token::Text);
                step.length = index - at;
                step.read = index - at;
            }

            fn letters(input: &str, index: usize) -> usize {
                let bytes = input.as_bytes();
                let mut index = index;
                while let Some(&byte) = bytes.get(index) {
                    if byte.wrapping_sub(b'a') <= 25 {
                        index += 1;
                    } else {
                        break;
                    }
                }
                index
            }

            fn open(at: usize, index: usize, step: &mut Step<Token>) {
                step.outcome = Outcome::Token(Token::Quote);
                step.length = index - at;
                step.read = index - at;
                step.condition = 1;
            }

            fn close(at: usize, index: usize, step: &mut Step<Token>) {
                step.outcome = Outcome::Token(Token::Quote);
                step.length = index - at;
                step.read = index - at;
                step.condition = 0;
            }

            fn fault(at: usize, index: usize, step: &mut Step<Token>) {
                step.outcome = Outcome::None;
                step.length = 0;
                step.read = index - at;
            }

            let condition = step.condition;
            match condition {
                0 => code(input, at, at, step),
                1 => text(input, at, at, step),
                condition => panic!("condition {condition} is not a condition of the lexer"),
            }
        }

        fn condition(index: u16) -> Context {
            match index {
                0 => Context::Code,
                1 => Context::Text,
                other => panic!("condition {other} is not a condition of the lexer"),
            }
        }
    }
}

/// Returns the tokens of `input`, and stops at the first fault.
fn tokens<T: Lexer>(input: &str) -> Vec<T> {
    T::scan(input)
        .map(|found| found.expect("each character of the input belongs to a token"))
        .collect()
}

#[test]
fn a_scan_gives_each_token_of_the_input() {
    use words::Token;

    assert_eq!(tokens::<Token>("aa a"), vec![Token::Word, Token::Word]);
    assert_eq!(tokens::<Token>("a"), vec![Token::Word]);
}

#[test]
fn a_scan_of_no_input_gives_no_token() {
    assert_eq!(tokens::<words::Token>(""), vec![]);
}

#[test]
fn a_rule_that_skips_gives_no_token_and_reads_its_match() {
    use words::Token;

    assert_eq!(tokens::<Token>("   "), vec![]);
    assert_eq!(tokens::<Token>(" a "), vec![Token::Word]);
}

#[test]
fn the_longest_match_wins() {
    let mut scan = words::Token::scan("aaa");

    assert_eq!(scan.next(), Some(Ok(words::Token::Word)));
    assert_eq!(scan.span(), 0..3);
    assert_eq!(scan.next(), None);
}

#[test]
fn a_scan_gives_the_span_and_the_text_of_each_token() {
    let mut scan = words::Token::scan("aa a");

    assert_eq!(scan.span(), 0..0);
    assert_eq!(scan.slice(), "");

    assert_eq!(scan.next(), Some(Ok(words::Token::Word)));
    assert_eq!(scan.span(), 0..2);
    assert_eq!(scan.slice(), "aa");

    assert_eq!(scan.next(), Some(Ok(words::Token::Word)));
    assert_eq!(scan.span(), 3..4);
    assert_eq!(scan.slice(), "a");
}

#[test]
fn a_scan_counts_the_line_and_the_column_of_each_token() {
    let mut scan = words::Token::scan("aa\na\n\na");

    assert_eq!(scan.next(), Some(Ok(words::Token::Word)));
    assert_eq!((scan.line(), scan.column()), (1, 1));

    assert_eq!(scan.next(), Some(Ok(words::Token::Word)));
    assert_eq!((scan.line(), scan.column()), (2, 1));

    assert_eq!(scan.next(), Some(Ok(words::Token::Word)));
    assert_eq!((scan.line(), scan.column()), (4, 1));
}

#[test]
fn a_column_counts_a_character_and_not_a_byte() {
    let mut scan = words::Token::scan("éa");

    let error = scan
        .next()
        .expect("the scan gives one result")
        .expect_err("no rule matches é");
    assert_eq!(error.span(), 0..2);
    assert_eq!((error.line(), error.column()), (1, 1));

    assert_eq!(scan.next(), Some(Ok(words::Token::Word)));
    assert_eq!((scan.line(), scan.column()), (1, 2));
    assert_eq!(scan.span(), 2..3);
}

#[test]
fn a_character_that_no_rule_matches_gives_one_error_and_the_scan_reads_on() {
    let found: Vec<_> = words::Token::scan("aZa").collect();

    assert_eq!(found.len(), 3);
    assert_eq!(found[0], Ok(words::Token::Word));
    assert!(found[1].is_err());
    assert_eq!(found[2], Ok(words::Token::Word));
}

#[test]
fn an_error_names_the_bytes_of_the_character_at_fault() {
    let error = words::Token::scan("Z")
        .next()
        .expect("the scan gives one result")
        .expect_err("no rule matches Z");

    assert_eq!(error.span(), 0..1);
    assert_eq!(error.line(), 1);
    assert_eq!(error.column(), 1);
    assert_eq!(
        error.to_string(),
        "no rule matches the input at line 1, column 1"
    );
}

#[test]
fn a_scan_of_only_faults_gives_one_error_for_each_character() {
    let found: Vec<_> = words::Token::scan("ZZ").collect();

    assert_eq!(found.len(), 2);
    assert!(found.iter().all(Result::is_err));
}

#[test]
fn a_rule_changes_the_start_condition_after_it_matches() {
    use strings::Token;

    assert_eq!(
        tokens::<Token>("ab\"cd\"ef"),
        vec![
            Token::Word,
            Token::Quote,
            Token::Text,
            Token::Quote,
            Token::Word,
        ]
    );
}

#[test]
fn only_the_rules_of_the_start_condition_match() {
    use strings::{Context, Token};

    let mut scan = Token::scan("\"ab");

    assert_eq!(scan.condition(), Context::Code);
    assert_eq!(scan.next(), Some(Ok(Token::Quote)));
    assert_eq!(scan.condition(), Context::Text);
    assert_eq!(scan.next(), Some(Ok(Token::Text)));
    assert_eq!(scan.slice(), "ab");
}

#[test]
fn a_rule_that_skips_leaves_the_place_of_the_last_token() {
    let mut scan = words::Token::scan("a  ");

    assert_eq!(scan.next(), Some(Ok(words::Token::Word)));
    assert_eq!(scan.span(), 0..1);

    assert_eq!(scan.next(), None);
    assert_eq!(scan.span(), 0..1);
    assert_eq!(scan.slice(), "a");
    assert_eq!((scan.line(), scan.column()), (1, 1));
}

#[test]
fn a_rule_that_skips_still_counts_the_lines_of_its_match() {
    let mut scan = words::Token::scan("\n\na");

    assert_eq!(scan.next(), Some(Ok(words::Token::Word)));
    assert_eq!((scan.line(), scan.column()), (3, 1));
}

#[test]
fn a_scan_reports_where_it_stopped() {
    let mut scan = words::Token::scan("aa a");

    assert_eq!(scan.offset(), 0);
    assert_eq!(scan.remainder(), "aa a");

    assert_eq!(scan.next(), Some(Ok(words::Token::Word)));
    assert_eq!(scan.offset(), 2);
    assert_eq!(scan.remainder(), " a");
}

#[test]
fn a_located_scan_gives_the_place_of_each_token_with_the_token() {
    let found: Vec<_> = words::Token::scan("aa\n a")
        .located()
        .map(|found| found.expect("each character of the input belongs to a token"))
        .collect();

    assert_eq!(
        found,
        vec![
            Located {
                token: words::Token::Word,
                span: 0..2,
                line: 1,
                column: 1,
            },
            Located {
                token: words::Token::Word,
                span: 4..5,
                line: 2,
                column: 2,
            },
        ]
    );
}

#[test]
fn a_located_scan_reports_each_character_that_no_rule_matches() {
    let found: Vec<_> = words::Token::scan("aZa").located().collect();

    assert_eq!(found.len(), 3);
    assert_eq!(found[0].as_ref().map(|found| found.span.clone()), Ok(0..1));
    assert_eq!(found[1].as_ref().map_err(|error| error.span()), Err(1..2));
    assert_eq!(found[2].as_ref().map(|found| found.span.clone()), Ok(2..3));
}

#[test]
fn a_located_scan_reads_the_start_condition_that_it_is_under() {
    use strings::{Context, Token};

    let mut scan = Token::scan("\"ab").located();

    assert_eq!(scan.condition(), Context::Code);
    assert_eq!(
        scan.next().map(|found| found.map(|found| found.token)),
        Some(Ok(Token::Quote))
    );
    assert_eq!(scan.condition(), Context::Text);
}

#[test]
fn a_located_scan_reports_where_it_stopped() {
    let mut scan = words::Token::scan("aa a").located();

    assert_eq!(scan.offset(), 0);
    assert_eq!(scan.remainder(), "aa a");

    assert!(scan.next().is_some());
    assert_eq!(scan.offset(), 2);
    assert_eq!(scan.remainder(), " a");
}

#[test]
fn a_scan_gives_no_token_after_the_end_of_the_input() {
    let mut scan = words::Token::scan("a");

    assert_eq!(scan.next(), Some(Ok(words::Token::Word)));
    assert_eq!(scan.next(), None);
    assert_eq!(scan.next(), None);
}
