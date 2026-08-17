//! Derives a lexer from an enum of tokens, then reads an input with it.
//!
//! The test uses the macro in the manner of a lexer author. Thus it covers the attributes, the
//! automaton, the emitted source, and the runtime together.

use lxr::Lexer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Context {
    Code,
    Text,
}

/// A lexer of a language that holds a keyword, a word, a number, and a string.
#[derive(Debug, PartialEq, Eq, Lexer)]
#[lxr(condition = Context::Code)]
#[lxr(skip = "[ \t\n]+")]
enum Token {
    #[lxr(token = "fn")]
    Function,
    #[lxr(token = "let")]
    Let,
    #[lxr(regex = "[a-z][a-z0-9]*")]
    Word,
    #[lxr(regex = "[0-9]+")]
    Number,
    #[lxr(token = "=")]
    Equals,
    #[lxr(token = "\"", go = Context::Text)]
    Quote,
    #[lxr(regex = "[^\"]+", in = [Context::Text])]
    Text,
    #[lxr(token = "\"", in = [Context::Text], go = Context::Code)]
    End,
}

/// A lexer that reads under one start condition, thus it names no condition enum.
#[derive(Debug, PartialEq, Eq, Lexer)]
#[lxr(skip = "[ ]+")]
enum Word {
    #[lxr(regex = "[a-z]+")]
    Letters,
    #[lxr(token = "+")]
    Plus,
}

/// Returns the tokens of `input`. Each character of the input belongs to a token.
fn tokens<T: Lexer>(input: &str) -> Vec<T> {
    T::scan(input)
        .map(|found| found.expect("each character of the input belongs to a token"))
        .collect()
}

#[test]
fn a_derived_lexer_reads_each_token_of_its_input() {
    assert_eq!(
        tokens::<Token>("let x9 = 12"),
        vec![Token::Let, Token::Word, Token::Equals, Token::Number,]
    );
}

#[test]
fn a_literal_wins_a_tie_against_a_regular_expression() {
    assert_eq!(tokens::<Token>("fn"), vec![Token::Function]);
    assert_eq!(tokens::<Token>("let"), vec![Token::Let]);
}

#[test]
fn the_longest_match_wins_against_an_earlier_rule() {
    assert_eq!(tokens::<Token>("fnord"), vec![Token::Word]);
    assert_eq!(tokens::<Token>("letter"), vec![Token::Word]);
}

#[test]
fn a_rule_that_skips_gives_no_token() {
    assert_eq!(tokens::<Token>("  \n\t "), vec![]);
    assert_eq!(tokens::<Token>(" fn "), vec![Token::Function]);
}

#[test]
fn a_rule_changes_the_start_condition_and_the_same_bytes_give_another_token() {
    assert_eq!(
        tokens::<Token>("fn\"fn\"fn"),
        vec![
            Token::Function,
            Token::Quote,
            Token::Text,
            Token::End,
            Token::Function,
        ]
    );
}

#[test]
fn a_scan_reads_the_condition_that_it_is_under() {
    let mut scan = Token::scan("\"ab\"");

    assert_eq!(scan.condition(), Context::Code);
    assert_eq!(scan.next(), Some(Ok(Token::Quote)));
    assert_eq!(scan.condition(), Context::Text);
    assert_eq!(scan.next(), Some(Ok(Token::Text)));
    assert_eq!(scan.next(), Some(Ok(Token::End)));
    assert_eq!(scan.condition(), Context::Code);
}

#[test]
fn a_space_inside_a_string_is_not_skipped() {
    assert_eq!(
        tokens::<Token>("\"a b\""),
        vec![Token::Quote, Token::Text, Token::End]
    );
}

#[test]
fn a_scan_gives_the_text_the_span_and_the_place_of_each_token() {
    let mut scan = Token::scan("let\n  x9");

    assert_eq!(scan.next(), Some(Ok(Token::Let)));
    assert_eq!(scan.slice(), "let");
    assert_eq!(scan.span(), 0..3);
    assert_eq!((scan.line(), scan.column()), (1, 1));

    assert_eq!(scan.next(), Some(Ok(Token::Word)));
    assert_eq!(scan.slice(), "x9");
    assert_eq!(scan.span(), 6..8);
    assert_eq!((scan.line(), scan.column()), (2, 3));
}

#[test]
fn a_for_loop_reads_the_place_of_each_token_with_the_token() {
    let source = "let x9\n= 12";
    let mut places = Vec::new();

    for found in Token::scan(source).located() {
        let found = found.expect("each character of the input belongs to a token");
        places.push((
            found.token,
            source[found.span].to_owned(),
            found.line,
            found.column,
        ));
    }

    assert_eq!(
        places,
        vec![
            (Token::Let, "let".to_owned(), 1, 1),
            (Token::Word, "x9".to_owned(), 1, 5),
            (Token::Equals, "=".to_owned(), 2, 1),
            (Token::Number, "12".to_owned(), 2, 3),
        ]
    );
}

#[test]
fn a_character_that_no_rule_matches_gives_an_error_and_the_scan_reads_on() {
    let found: Vec<_> = Token::scan("fn % fn").collect();

    assert_eq!(found[0], Ok(Token::Function));
    assert!(found[1].is_err());
    assert_eq!(found[2], Ok(Token::Function));
    assert_eq!(found.len(), 3);
}

#[test]
fn an_error_names_the_place_of_the_character_at_fault() {
    let error = Token::scan("fn\n%")
        .nth(1)
        .expect("the scan gives two results")
        .expect_err("no rule matches the percent sign");

    assert_eq!(error.span, 3..4);
    assert_eq!((error.line, error.column), (2, 1));
}

#[test]
fn a_lexer_of_one_start_condition_needs_no_condition_enum() {
    assert_eq!(
        tokens::<Word>("ab + cd"),
        vec![Word::Letters, Word::Plus, Word::Letters]
    );
}

#[test]
fn a_literal_needs_no_escape_of_a_regex_character() {
    assert_eq!(tokens::<Word>("+"), vec![Word::Plus]);
}

#[test]
fn two_lexers_live_in_one_module() {
    assert_eq!(tokens::<Word>("ab"), vec![Word::Letters]);
    assert_eq!(tokens::<Token>("ab"), vec![Token::Word]);
}

#[test]
fn a_lexer_reads_a_character_above_ascii_inside_a_string() {
    let mut scan = Token::scan("\"héllo\"");

    assert_eq!(scan.next(), Some(Ok(Token::Quote)));
    assert_eq!(scan.next(), Some(Ok(Token::Text)));
    assert_eq!(scan.slice(), "héllo");
    assert_eq!(scan.next(), Some(Ok(Token::End)));
}
