use crate::support::token_definition;

pub const SOURCE: &str = include_str!("../sources/punctuation.txt");

token_definition! {
    #[lexer(skip = r"[ \t\r\n]+")]
    pub enum Token {
        #[token("::")]
        Path,
        #[token("->")]
        Arrow,
        #[token("=>")]
        FatArrow,
        #[token("==")]
        Equal,
        #[token("!=")]
        NotEqual,
        #[token("<=")]
        LessOrEqual,
        #[token(">=")]
        GreaterOrEqual,
        #[token("&&")]
        LogicalAnd,
        #[token(r"\|\|")]
        LogicalOr,
        #[token(r"\+")]
        Plus,
        #[token("-")]
        Minus,
        #[token(r"\*")]
        Star,
        #[token("/")]
        Slash,
        #[token("%")]
        Percent,
        #[token("=")]
        Assign,
        #[token("<")]
        Less,
        #[token(">")]
        Greater,
        #[token(r"\{")]
        LeftBrace,
        #[token(r"\}")]
        RightBrace,
        #[token(r"\(")]
        LeftParenthesis,
        #[token(r"\)")]
        RightParenthesis,
        #[token(r"\[")]
        LeftBracket,
        #[token(r"\]")]
        RightBracket,
        #[token(",")]
        Comma,
        #[token(";")]
        Semicolon,
        #[token(":")]
        Colon,
        #[token(r"\.")]
        Dot,
        #[token(r"\?")]
        Question,
        #[token("&")]
        Ampersand,
        #[token(r"\|")]
        Pipe,
    }
}
