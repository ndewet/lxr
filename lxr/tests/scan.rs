//! Scans an input with tables that this test builds by hand.
//!
//! The derive macro builds a table from an automaton. This test builds one directly, thus it
//! specifies the runtime and it needs no macro.

use lxr::{Action, Lexer, Located, Tables};

/// A lexer of one condition. It reads a word of the letter `a`, and it skips a space and a newline.
///
/// | State | Meaning |
/// | --- | --- |
/// | 0 | The dead state. |
/// | 1 | The start. |
/// | 2 | Inside a word. It accepts rule 0. |
/// | 3 | A space or a newline. It accepts rule 1, which skips. |
mod words {
    use super::{Action, Lexer, Tables};

    #[derive(Debug, PartialEq, Eq)]
    pub enum Token {
        Word,
    }

    /// Class 1 is the letter `a`. Class 2 is a space or a newline.
    static CLASSES: [u16; 256] = {
        let mut classes = [0; 256];
        classes[b'a' as usize] = 1;
        classes[b' ' as usize] = 2;
        classes[b'\n' as usize] = 2;
        classes
    };

    #[rustfmt::skip]
    static NEXT: [u16; 12] = [
        0, 0, 0,
        0, 2, 3,
        0, 2, 0,
        0, 0, 0,
    ];

    static ACCEPT: [u16; 4] = [0, 0, 1, 2];
    static START: [u16; 1] = [1];
    static ACTIONS: [Action; 2] = [Action::token(), Action::skip()];

    impl Lexer for Token {
        type Condition = ();

        const TABLES: Tables<'static> = Tables {
            classes: &CLASSES,
            next: &NEXT,
            width: 3,
            accept: &ACCEPT,
            start: &START,
            actions: &ACTIONS,
        };

        fn token(rule: u16, _text: &str) -> Option<Self> {
            match rule {
                0 => Some(Token::Word),
                other => panic!("rule {other} gives no token"),
            }
        }

        fn condition(_index: u16) {}
    }
}

/// A lexer of two conditions. A quote changes the condition, thus the same letters give a different
/// token inside a string.
///
/// | State | Meaning |
/// | --- | --- |
/// | 1 | The start of the code condition. |
/// | 2 | The start of the text condition. |
/// | 3 | A quote in code. It accepts rule 0, which goes to text. |
/// | 4 | A word in code. It accepts rule 3. |
/// | 5 | A quote in text. It accepts rule 1, which goes to code. |
/// | 6 | Text in a string. It accepts rule 2. |
mod strings {
    use super::{Action, Lexer, Tables};

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

    /// Class 1 is a quote. Class 2 is a letter from `a` to `z`.
    static CLASSES: [u16; 256] = {
        let mut classes = [0; 256];
        classes[b'"' as usize] = 1;
        let mut byte = b'a';
        while byte <= b'z' {
            classes[byte as usize] = 2;
            byte += 1;
        }
        classes
    };

    #[rustfmt::skip]
    static NEXT: [u16; 21] = [
        0, 0, 0,
        0, 3, 4,
        0, 5, 6,
        0, 0, 0,
        0, 0, 4,
        0, 0, 0,
        0, 0, 6,
    ];

    static ACCEPT: [u16; 7] = [0, 0, 0, 1, 4, 2, 3];
    static START: [u16; 2] = [1, 2];
    static ACTIONS: [Action; 4] = [
        Action::token().going(1),
        Action::token().going(0),
        Action::token(),
        Action::token(),
    ];

    impl Lexer for Token {
        type Condition = Context;

        const TABLES: Tables<'static> = Tables {
            classes: &CLASSES,
            next: &NEXT,
            width: 3,
            accept: &ACCEPT,
            start: &START,
            actions: &ACTIONS,
        };

        fn token(rule: u16, _text: &str) -> Option<Self> {
            match rule {
                0 | 1 => Some(Token::Quote),
                2 => Some(Token::Text),
                3 => Some(Token::Word),
                other => panic!("rule {other} gives no token"),
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
    assert_eq!(error.span, 0..2);
    assert_eq!((error.line, error.column), (1, 1));

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

    assert_eq!(error.span, 0..1);
    assert_eq!(error.line, 1);
    assert_eq!(error.column, 1);
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
    assert_eq!(
        found[1].as_ref().map_err(|error| error.span.clone()),
        Err(1..2)
    );
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
fn a_scan_gives_no_token_after_the_end_of_the_input() {
    let mut scan = words::Token::scan("a");

    assert_eq!(scan.next(), Some(Ok(words::Token::Word)));
    assert_eq!(scan.next(), None);
    assert_eq!(scan.next(), None);
}
