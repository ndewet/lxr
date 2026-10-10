use crate::support::token_definition;

token_definition! {
    #[lexer(skip = r"[ \t\r\n]+")]
    pub enum Token {
        #[token("[a-zA-Z_][a-zA-Z0-9_]*")]
        Identifier,
        #[token("[0-9]+")]
        Integer,
        #[token(r"[+*/=-]")]
        Operator,
    }
}
