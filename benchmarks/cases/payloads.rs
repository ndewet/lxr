use crate::support::token_definition;

pub const SOURCE: &str = include_str!("../sources/payloads.txt");

token_definition! {
    #[lexer(skip = r"[ \t\r\n,;]+")]
    #[allow(dead_code)]
    pub enum Token {
        #[token("true|false")]
        Boolean(bool),
        #[token("-?[0-9]+")]
        Integer(i64),
        #[token("[0-9]+\\.[0-9]+")]
        Decimal(f64),
        #[token("[a-z][a-z0-9_]*")]
        Text(String),
    }
}
