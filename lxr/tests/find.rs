//! Asserts that the scan that the macro emits agrees with the scan of the tables.
//!
//! The derive macro emits one scan for each lexer, and the runtime keeps the scan of the tables
//! for a region that no rule ends. Thus one lexer holds two scans, and each one must give the same
//! match. This test reads each offset of each input with both, and it compares the rule, the
//! length, and the bytes that each one read.

use lxr::{Lexer, Matched};

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

/// A lexer that reads characters above ASCII, which the automaton lowers to bytes.
#[derive(Debug, PartialEq, Eq, Lexer)]
enum Wide {
    #[lxr(regex = "[\u{00e0}-\u{00ff}]+")]
    Letters,
    #[lxr(regex = "[\u{4e00}-\u{9fff}]")]
    Sign,
    #[lxr(regex = ".")]
    Other,
}

/// Asserts that the two scans of `T` agree at each offset of `input`, under each condition.
///
/// # Panics
///
/// This function panics if the two scans disagree.
fn agree<T: Lexer>(input: &str) {
    let bytes = input.as_bytes();
    let conditions = T::TABLES.start.len();

    for condition in 0..conditions {
        let condition = u16::try_from(condition).expect("a test holds few conditions");
        for at in 0..=bytes.len() {
            let code: Matched = T::find(bytes, at, condition);
            let tables: Matched = T::TABLES.find(bytes, at, condition);

            assert_eq!(
                code, tables,
                "the two scans disagree at the offset {at} under the condition {condition} \
                 of {input:?}"
            );
        }
    }
}

#[test]
fn the_two_scans_of_a_lexer_of_names_agree() {
    for input in [
        "",
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
        agree::<Code>(input);
    }
}

#[test]
fn the_two_scans_of_a_lexer_of_conditions_agree() {
    for input in ["", "\"", "one\"two\"", "\"\"", "abc", "\"a b c\""] {
        agree::<Text>(input);
    }
}

#[test]
fn the_two_scans_of_a_lexer_of_wide_characters_agree() {
    for input in ["", "à", "àéÿ", "中", "中文", "aà中", "\u{ffff}"] {
        agree::<Wide>(input);
    }
}

#[test]
fn the_two_scans_of_a_lexer_agree_at_each_byte_of_a_long_input() {
    let input = "let name = 1234 + other_name; fn f() ".repeat(20);

    agree::<Code>(&input);
}
