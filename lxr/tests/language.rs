//! Reads a whole source of a small language, and asserts each token, each fault, and each place.
//!
//! The lexer holds the constructions that a real language needs: a keyword that an identifier can
//! hide, an operator of two characters, two forms of a number, a comment, and a string with an
//! escape. Thus the tests cover the longest match, the precedence, and the start conditions on an
//! input that a person would write.

mod common;

use common::{Step, accepts, steps, tokens};
use lxr::Lexer;

/// The start conditions of [`Token`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Context {
    Code,
    Text,
}

/// The tokens of a small language.
///
/// Each keyword comes before [`Name`](Token::Name). The two match the same length at `let`, thus
/// the keyword wins. `letter` is longer, thus the name wins there.
///
/// [`Float`](Token::Float) comes after [`Int`](Token::Int), and the longest match still gives
/// `1.5` to the float. The sequence of the rules decides a tie alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Lexer)]
#[lxr(condition = Context::Code)]
#[lxr(skip = r"[ \t\r\n]+")]
#[lxr(skip = "#[^\n]*")]
enum Token {
    #[lxr(token = "fn")]
    Fn,
    #[lxr(token = "let")]
    Let,
    #[lxr(token = "if")]
    If,
    #[lxr(token = "else")]
    Else,
    #[lxr(token = "return")]
    Return,
    #[lxr(regex = "[A-Za-z_][A-Za-z0-9_]*")]
    Name,
    #[lxr(regex = "[0-9]+")]
    Int,
    #[lxr(regex = r"[0-9]+\.[0-9]+")]
    Float,
    #[lxr(token = "==")]
    Eq,
    #[lxr(token = "!=")]
    Ne,
    #[lxr(token = "<=")]
    Le,
    #[lxr(token = ">=")]
    Ge,
    #[lxr(token = "->")]
    Arrow,
    #[lxr(token = "=")]
    Assign,
    #[lxr(token = "<")]
    Lt,
    #[lxr(token = ">")]
    Gt,
    #[lxr(token = "+")]
    Plus,
    #[lxr(token = "-")]
    Minus,
    #[lxr(token = "(")]
    Open,
    #[lxr(token = ")")]
    Close,
    #[lxr(token = "{")]
    OpenBrace,
    #[lxr(token = "}")]
    CloseBrace,
    #[lxr(token = ",")]
    Comma,
    #[lxr(token = ";")]
    Semicolon,
    #[lxr(token = "\"", go = Context::Text)]
    Quote,
    #[lxr(regex = r#"([^"\\]|\\.)+"#, in = [Context::Text])]
    Text,
    #[lxr(token = "\"", in = [Context::Text], go = Context::Code)]
    End,
}

const SOURCE: &str = "fn add(a, b) -> int {
    let total = a + b;   # sum
    if total >= 100 { return total; }
    return 0;
}";

#[test]
fn the_lexer_reads_a_whole_source_into_the_expected_tokens() {
    use Token::{
        Assign, Close, CloseBrace, Comma, Fn, Ge, If, Int, Let, Name, Open, OpenBrace, Plus,
        Return, Semicolon,
    };

    assert_eq!(
        tokens::<Token>(SOURCE),
        vec![
            Fn,
            Name,
            Open,
            Name,
            Comma,
            Name,
            Close,
            Token::Arrow,
            Name,
            OpenBrace,
            Let,
            Name,
            Assign,
            Name,
            Plus,
            Name,
            Semicolon,
            If,
            Name,
            Ge,
            Int,
            OpenBrace,
            Return,
            Name,
            Semicolon,
            CloseBrace,
            Return,
            Int,
            Semicolon,
            CloseBrace,
        ]
    );
}

#[test]
fn each_character_of_the_source_belongs_to_a_token_or_to_a_rule_that_skips() {
    assert!(accepts::<Token>(SOURCE));
}

#[test]
fn a_keyword_wins_a_tie_and_a_longer_name_wins_the_length() {
    assert_eq!(tokens::<Token>("let"), vec![Token::Let]);
    assert_eq!(tokens::<Token>("letter"), vec![Token::Name]);
    assert_eq!(tokens::<Token>("if"), vec![Token::If]);
    assert_eq!(tokens::<Token>("iff"), vec![Token::Name]);
    assert_eq!(tokens::<Token>("_let"), vec![Token::Name]);
    assert_eq!(tokens::<Token>("let1"), vec![Token::Name]);
    assert_eq!(tokens::<Token>("Let"), vec![Token::Name]);
}

#[test]
fn a_keyword_that_another_keyword_starts_still_reads_as_one_token() {
    assert_eq!(tokens::<Token>("iffy"), vec![Token::Name]);
    assert_eq!(
        tokens::<Token>("if fy"),
        vec![Token::If, Token::Name],
        "a space divides the keyword from the name"
    );
}

#[test]
fn an_operator_of_two_characters_wins_against_one_of_one() {
    assert_eq!(tokens::<Token>("=="), vec![Token::Eq]);
    assert_eq!(tokens::<Token>("="), vec![Token::Assign]);
    assert_eq!(tokens::<Token>("<="), vec![Token::Le]);
    assert_eq!(tokens::<Token>("<"), vec![Token::Lt]);
    assert_eq!(tokens::<Token>("->"), vec![Token::Arrow]);
    assert_eq!(tokens::<Token>("- >"), vec![Token::Minus, Token::Gt]);
    assert_eq!(
        tokens::<Token>("==="),
        vec![Token::Eq, Token::Assign],
        "the longest match reads two characters, thus one is left"
    );
}

#[test]
fn a_number_reads_as_an_integer_or_as_a_float() {
    assert_eq!(tokens::<Token>("100"), vec![Token::Int]);
    assert_eq!(tokens::<Token>("1.5"), vec![Token::Float]);
    assert_eq!(tokens::<Token>("0.0"), vec![Token::Float]);
    assert_eq!(
        steps::<Token>("1 .5"),
        vec![
            Step::Token(Token::Int, 0..1),
            Step::Fault(2..3),
            Step::Token(Token::Int, 3..4),
        ],
        "a space divides the digit from the point, thus no float matches"
    );
}

#[test]
fn a_float_needs_a_digit_on_each_side_of_the_point() {
    assert_eq!(
        steps::<Token>("1."),
        vec![Step::Token(Token::Int, 0..1), Step::Fault(1..2)]
    );
    assert_eq!(
        steps::<Token>(".5"),
        vec![Step::Fault(0..1), Step::Token(Token::Int, 1..2)]
    );
    assert_eq!(
        steps::<Token>("1.5.2"),
        vec![
            Step::Token(Token::Float, 0..3),
            Step::Fault(3..4),
            Step::Token(Token::Int, 4..5),
        ]
    );
}

#[test]
fn a_comment_reads_to_the_end_of_its_line_and_gives_no_token() {
    assert_eq!(tokens::<Token>("# nothing here"), vec![]);
    assert_eq!(
        tokens::<Token>("let # a comment\nx"),
        vec![Token::Let, Token::Name]
    );
    assert_eq!(
        tokens::<Token>("#\nlet"),
        vec![Token::Let],
        "a comment of no text still reads to the newline"
    );
}

#[test]
fn a_string_holds_a_character_that_means_something_else_in_code() {
    assert_eq!(
        tokens::<Token>("\"let # 1.5\""),
        vec![Token::Quote, Token::Text, Token::End]
    );
    assert_eq!(
        tokens::<Token>("\"\""),
        vec![Token::Quote, Token::End],
        "a string of no text gives no text token"
    );
}

#[test]
fn an_escape_inside_a_string_does_not_end_it() {
    let mut scan = Token::scan(r#""a \" b""#);

    assert_eq!(scan.next(), Some(Ok(Token::Quote)));
    assert_eq!(scan.next(), Some(Ok(Token::Text)));
    assert_eq!(scan.slice(), r#"a \" b"#);
    assert_eq!(scan.next(), Some(Ok(Token::End)));
    assert_eq!(scan.next(), None);
}

#[test]
fn a_string_that_no_quote_ends_reads_to_the_end_of_the_input() {
    let found = tokens::<Token>("\"abc");

    assert_eq!(
        found,
        vec![Token::Quote, Token::Text],
        "the lexer reads the text and stops. A parser reports the missing quote"
    );
}

#[test]
fn a_scan_leaves_the_code_condition_and_comes_back_to_it() {
    let scan = Token::scan("let x = \"a\"; let");

    assert_eq!(scan.condition(), Context::Code);
    let seen: Vec<Token> = scan
        .map(|found| found.expect("each character belongs to a token"))
        .collect();

    assert_eq!(
        seen,
        vec![
            Token::Let,
            Token::Name,
            Token::Assign,
            Token::Quote,
            Token::Text,
            Token::End,
            Token::Semicolon,
            Token::Let,
        ]
    );
}

#[test]
fn a_character_that_no_rule_matches_gives_a_fault_and_the_scan_reads_on() {
    assert_eq!(
        steps::<Token>("let $ x"),
        vec![
            Step::Token(Token::Let, 0..3),
            Step::Fault(4..5),
            Step::Token(Token::Name, 6..7),
        ]
    );
}

#[test]
fn each_character_at_fault_gives_its_own_report() {
    assert_eq!(
        steps::<Token>("$@%"),
        vec![Step::Fault(0..1), Step::Fault(1..2), Step::Fault(2..3)]
    );
}

#[test]
fn a_fault_of_a_character_above_ascii_covers_each_of_its_bytes() {
    assert_eq!(
        steps::<Token>("é"),
        vec![Step::Fault(0..2)],
        "the character holds two bytes, thus the fault covers both"
    );
    assert_eq!(
        steps::<Token>("a€b"),
        vec![
            Step::Token(Token::Name, 0..1),
            Step::Fault(1..4),
            Step::Token(Token::Name, 4..5),
        ]
    );
}

#[test]
fn a_string_reads_a_character_above_ascii_as_one_token() {
    let mut scan = Token::scan("\"héllo wörld\"");

    assert_eq!(scan.next(), Some(Ok(Token::Quote)));
    assert_eq!(scan.next(), Some(Ok(Token::Text)));
    assert_eq!(scan.slice(), "héllo wörld");
    assert_eq!(scan.span(), 1..14);
    assert_eq!(scan.next(), Some(Ok(Token::End)));
}

#[test]
fn a_scan_gives_the_line_and_the_column_of_each_token() {
    let mut scan = Token::scan(SOURCE);

    assert_eq!(scan.next(), Some(Ok(Token::Fn)));
    assert_eq!((scan.line(), scan.column()), (1, 1));

    let found = scan
        .by_ref()
        .take_while(|found| found != &Ok(Token::Let))
        .count();
    assert_eq!(found, 9, "the first line holds nine more tokens");
    assert_eq!((scan.line(), scan.column()), (2, 5), "let is on line 2");
}

#[test]
fn an_empty_input_gives_no_token_and_no_fault() {
    assert_eq!(steps::<Token>(""), vec![]);
    assert!(accepts::<Token>(""));
}

#[test]
fn an_input_of_only_spaces_and_comments_gives_no_token() {
    assert_eq!(steps::<Token>("   \n\t  # a comment\n  "), vec![]);
    assert!(accepts::<Token>("   \n\t  # a comment\n  "));
}

/// Each input that the lexer reads with no fault.
const ACCEPTED: [&str; 16] = [
    "",
    " ",
    "let",
    "letter",
    "_",
    "_9",
    "0",
    "100",
    "1.5",
    "==",
    "->",
    "<=",
    "# a comment",
    "\"text\"",
    "\"\"",
    "fn f() -> int { return 1; }",
];

/// Each input that holds a character that no rule of the lexer reads.
const REJECTED: [&str; 12] = [
    "$", "@", "%", "&", "|", "^", "~", ".", "1.", ".5", "let $", "é",
];

#[test]
fn the_lexer_reads_each_input_of_the_language() {
    for input in ACCEPTED {
        assert!(accepts::<Token>(input), "the lexer rejects {input:?}");
    }
}

#[test]
fn the_lexer_reads_no_input_outside_the_language() {
    for input in REJECTED {
        assert!(!accepts::<Token>(input), "the lexer accepts {input:?}");
    }
}

#[test]
fn a_scan_gives_nothing_after_it_reads_the_last_token() {
    let mut scan = Token::scan("let");

    assert_eq!(scan.next(), Some(Ok(Token::Let)));
    assert_eq!(scan.next(), None);
    assert_eq!(scan.next(), None, "a scan that ended stays ended");
}

#[test]
fn a_scan_of_only_faults_ends_and_gives_nothing_after_it() {
    let mut scan = Token::scan("$$");

    assert!(scan.next().is_some_and(|found| found.is_err()));
    assert!(scan.next().is_some_and(|found| found.is_err()));
    assert_eq!(scan.next(), None);
    assert_eq!(scan.next(), None);
}

#[test]
fn the_span_of_each_token_slices_the_input_back_to_its_text() {
    let mut scan = Token::scan(SOURCE);
    let mut read = String::new();

    while scan.next().is_some() {
        read.push_str(&SOURCE[scan.span()]);
        assert_eq!(&SOURCE[scan.span()], scan.slice());
    }

    assert_eq!(
        read, "fnadd(a,b)->int{lettotal=a+b;iftotal>=100{returntotal;}return0;}",
        "the spans hold each byte that is not a space and not a comment"
    );
}
