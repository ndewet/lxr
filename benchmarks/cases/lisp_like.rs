use crate::support::token_definition;

pub const SOURCE: &str = include_str!("../sources/lisp_like.lisp");

token_definition! {
    #[lexer(skip = r"[ \t\r\n]+")]
    #[lexer(skip = r";[^\n]*")]
    pub enum Token {
        #[token(r"\(")]
        LeftParenthesis,
        #[token(r"\)")]
        RightParenthesis,
        #[token("'")]
        Quote,
        #[token(r#""([^"\\]|\\.)*""#)]
        String,
        #[token("-?[0-9]+")]
        Integer,
        #[token("#[tf]")]
        Boolean,
        #[token("[A-Za-z_+*/<>=!?-][A-Za-z0-9_+*/<>=!?-]*")]
        Symbol,
    }
}
