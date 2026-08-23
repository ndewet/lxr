//! Asserts that one step of a lexer reads the input at its offset, and nothing before it.
//!
//! [`Lexer::step`] takes the whole input and the offset at which the match starts. A node of the
//! graph that reads the wrong offset still gives a token, thus a scan of a short input hides the
//! fault. This test reads each offset of each input two times: one time at that offset of the
//! whole input, and one time at the offset 0 of the input after it. The two must agree, because a
//! lexer holds no context except its start condition.
//!
//! The comparison covers the token, the length of the match, the bytes that the step read, and the
//! start condition of the next step.

use lxr::{Lexer, Step};

/// The start conditions of the lexer of strings.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Context {
    Code,
    Text,
}

/// A lexer of names, numbers, keywords, and one literal of three characters.
#[derive(Debug, PartialEq, Eq, Lexer)]
#[lxr(skip = "[ \t\n]+")]
enum Code {
    #[lxr(token = "let")]
    Let,
    #[lxr(token = "fn")]
    Function,
    #[lxr(regex = "[a-z_][a-z0-9_]*")]
    Name,
    #[lxr(regex = "[0-9]+(\\.[0-9]+)?")]
    Number,
    #[lxr(regex = "[+*/-]")]
    Operator,
}

/// A lexer of two start conditions, which reads the text of a string under the second one.
#[derive(Debug, PartialEq, Eq, Lexer)]
#[lxr(condition = Context::Code)]
enum Text {
    #[lxr(regex = "[a-z]+")]
    Word,
    #[lxr(token = "\"", go = Context::Text)]
    Open,
    #[lxr(regex = "[^\"]+", in = [Context::Text])]
    Body,
    #[lxr(token = "\"", in = [Context::Text], go = Context::Code)]
    Close,
}

/// A lexer that reads characters above ASCII, which the graph lowers to bytes.
#[derive(Debug, PartialEq, Eq, Lexer)]
enum Wide {
    #[lxr(regex = "[\u{00e0}-\u{00ff}]+")]
    Letters,
    #[lxr(regex = "[\u{4e00}-\u{9fff}]")]
    Sign,
    #[lxr(regex = ".")]
    Other,
}

/// Asserts that a step of `T` at each offset of `input` agrees with a step of the input after that
/// offset, under `conditions` start conditions.
///
/// # Panics
///
/// This function panics if the two steps disagree.
fn agree<T: Lexer + PartialEq + std::fmt::Debug>(input: &str, conditions: u16) {
    for condition in 0..conditions {
        for at in 0..input.len() {
            if !input.is_char_boundary(at) {
                continue;
            }

            let mut whole = Step::new(condition);
            T::step(input, at, &mut whole);

            let mut rest = Step::new(condition);
            T::step(&input[at..], 0, &mut rest);

            assert_eq!(
                (whole.outcome, whole.length, whole.read, whole.condition),
                (rest.outcome, rest.length, rest.read, rest.condition),
                "the step disagrees at the offset {at} under the condition {condition} \
                 of {input:?}"
            );
            assert!(
                whole.length <= whole.read,
                "the step read {} bytes and matched {} of them",
                whole.read,
                whole.length
            );
            assert!(
                at + whole.read <= input.len(),
                "the step read past the end of the input"
            );
        }
    }
}

#[test]
fn each_step_of_a_lexer_of_names_reads_its_own_offset() {
    for input in [
        "let",
        "le",
        "lets",
        "let it be 42",
        "fn f() 1.5 + 2",
        "12.",
        "12.5.7",
        "____",
        "1 2 3 4 5",
        "!!!",
        "let\n\tname",
    ] {
        agree::<Code>(input, 1);
    }
}

#[test]
fn each_step_of_a_lexer_of_conditions_reads_its_own_offset() {
    for input in ["\"", "one\"two\"", "\"\"", "abc", "\"a b c\""] {
        agree::<Text>(input, 2);
    }
}

#[test]
fn each_step_of_a_lexer_of_wide_characters_reads_its_own_offset() {
    for input in ["à", "àéÿ", "中", "中文", "aà中", "\u{ffff}"] {
        agree::<Wide>(input, 1);
    }
}

#[test]
fn each_step_of_a_long_input_reads_its_own_offset() {
    let input = "let name = 1234 + other_name; fn f() ".repeat(20);

    agree::<Code>(&input, 1);
}

#[test]
fn a_step_that_no_rule_matches_gives_no_token_and_no_length() {
    let mut step = Step::new(0);

    Code::step("!", 0, &mut step);

    assert_eq!(step.outcome, lxr::Outcome::None);
    assert_eq!(step.length, 0);
}

#[test]
fn a_step_that_reads_past_its_match_reports_both_lengths() {
    let mut step = Step::new(0);

    Code::step("12.", 0, &mut step);

    assert_eq!(step.outcome, lxr::Outcome::Token(Code::Number));
    assert_eq!(step.length, 2);
    assert_eq!(step.read, 3);
}

#[test]
fn a_rule_that_changes_the_start_condition_writes_it_in_the_step() {
    let mut step = Step::new(0);

    Text::step("\"a\"", 0, &mut step);

    assert_eq!(step.outcome, lxr::Outcome::Token(Text::Open));
    assert_eq!(step.condition, 1);
}
