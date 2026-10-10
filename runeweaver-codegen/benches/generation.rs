//! Measures lexer generation for representative grammars.

use runeweaver_codegen::{RuleSpec, compile};
use std::hint::black_box;
use std::time::Instant;

fn emit(pattern: &str) -> RuleSpec {
    RuleSpec::emit(pattern, quote::quote!(Ok(Some(Self::Token))))
}

fn small_rules() -> Vec<RuleSpec> {
    vec![
        RuleSpec::skip(r"[ \t\r\n]+"),
        emit("[a-zA-Z_][a-zA-Z0-9_]*"),
        emit("[0-9]+"),
        emit(r"[+*/=-]"),
    ]
}

fn rust_like_rules() -> Vec<RuleSpec> {
    let mut rules = vec![
        RuleSpec::skip(r"[ \t\r\n]+"),
        RuleSpec::skip(r"//[^\n]*"),
        RuleSpec::skip(r"/\*([^*]|\*[^/])*\*/"),
    ];
    rules.extend(
        [
            "fn",
            "let",
            "if",
            "else",
            "while",
            "for",
            "return",
            "struct",
            "enum",
            "impl",
            "[a-zA-Z_][a-zA-Z0-9_]*",
            r"[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?",
            "[0-9]+",
            r#""([^"\\]|\\.)*""#,
            "==",
            "!=",
            "<=",
            ">=",
            "->",
            "=>",
            r"[+*/%=<>{}(),;.-]",
        ]
        .into_iter()
        .map(emit),
    );
    rules
}

fn huge_language_rules() -> Vec<RuleSpec> {
    let mut rules = vec![
        RuleSpec::skip(r"[ \t\r\n]+"),
        RuleSpec::skip(r"//[^\n]*"),
        RuleSpec::skip(r"/\*([^*]|\*[^/])*\*/"),
    ];
    rules.extend((0..1_000).map(|index| emit(&format!("keyword{index:04}"))));
    rules.extend(
        [
            "[a-zA-Z_][a-zA-Z0-9_]*",
            r"[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?",
            "[0-9]+",
            r#""([^"\\]|\\.)*""#,
            "==",
            "!=",
            "<=",
            ">=",
            "->",
            "=>",
            r"[+*/%=<>{}(),;.-]",
        ]
        .into_iter()
        .map(emit),
    );
    rules
}

fn measure(name: &str, iterations: u32, mut rules: impl FnMut() -> Vec<RuleSpec>) {
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(compile(black_box(rules())).expect("the benchmark grammar is valid"));
    }
    let per_iteration = start.elapsed() / iterations;
    println!("{name}: {per_iteration:?} per iteration");
}

fn main() {
    measure("small", 500, small_rules);
    measure("rust_like", 50, rust_like_rules);
    measure("huge_language", 5, huge_language_rules);
}
