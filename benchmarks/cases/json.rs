use crate::support::token_definition;

pub const SOURCE: &str = include_str!("../sources/example.json");

token_definition! {
    #[lexer(skip = r"[ \t\r\n]+")]
    pub enum Token {
        #[token("null")]
        Null,
        #[token("true")]
        True,
        #[token("false")]
        False,
        #[token(r#""([^"\\]|\\.)*""#)]
        String,
        #[token(r"-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?")]
        Number,
        #[token(r"[{}\[\],:]")]
        Punctuation,
    }
}
