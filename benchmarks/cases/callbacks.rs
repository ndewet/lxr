use crate::support::token_definition;
use std::num::ParseIntError;

pub const SOURCE: &str = include_str!("../sources/callbacks.txt");

fn unquote(text: &str) -> String {
    text[1..text.len() - 1].replace("\\\"", "\"")
}

fn parse_hex(text: &str) -> Result<u64, ParseIntError> {
    u64::from_str_radix(&text[2..], 16)
}

fn uppercase(text: &str) -> String {
    text[1..].to_uppercase()
}

token_definition! {
    #[lexer(skip = r"[ \t\r\n,;]+")]
    #[allow(dead_code)]
    pub enum Token {
        #[token(r#""([^"\\]|\\.)*""#, with = unquote)]
        Quoted(String),
        #[token("0x[0-9A-Fa-f]+", with = parse_hex)]
        Hexadecimal(u64),
        #[token("![a-z]+", with = uppercase)]
        Uppercase(String),
    }
}
