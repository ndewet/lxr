use crate::support::token_definition;

pub const SOURCE: &str = include_str!("../sources/modes.txt");

token_definition! {
    #[lexer(mode = Directive)]
    #[lexer(skip = r"[ \t\r\n]+", modes = [INITIAL, Directive])]
    pub enum Token {
        #[token("@", begin = Directive)]
        DirectiveStart,
        #[token(";", modes = Directive, begin = INITIAL)]
        DirectiveEnd,
        #[token("[a-z][a-z0-9_]*")]
        Word,
        #[token("[A-Z][A-Z0-9_]*", modes = Directive)]
        Command,
        #[token("[0-9]+", modes = Directive)]
        Argument,
        #[token("[,=]", modes = Directive)]
        Punctuation,
    }
}
