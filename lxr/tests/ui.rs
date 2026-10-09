//! Checks the compiler diagnostics for invalid lexer declarations.

#[test]
fn lexer_declaration_errors_name_the_invalid_input() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
}
