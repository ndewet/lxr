use crate::support::token_definition;

pub const SOURCE: &str = include_str!("../sources/rust_like.rs");

token_definition! {
    #[lexer(skip = r"[ \t\r\n]+")]
    #[lexer(skip = r"//[^\n]*")]
    #[lexer(skip = r"/\*([^*]|\*[^/])*\*/")]
    pub enum Token {
        #[token("fn")]
        Function,
        #[token("let")]
        Let,
        #[token("if")]
        If,
        #[token("else")]
        Else,
        #[token("while")]
        While,
        #[token("for")]
        For,
        #[token("return")]
        Return,
        #[token("struct")]
        Struct,
        #[token("enum")]
        Enum,
        #[token("impl")]
        Impl,
        #[token("[a-zA-Z_][a-zA-Z0-9_]*")]
        Identifier,
        #[token(r"[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?")]
        Float,
        #[token("[0-9]+")]
        Integer,
        #[token(r#""([^"\\]|\\.)*""#)]
        String,
        #[token("==")]
        Equal,
        #[token("!=")]
        NotEqual,
        #[token("<=")]
        LessOrEqual,
        #[token(">=")]
        GreaterOrEqual,
        #[token("->")]
        Arrow,
        #[token("=>")]
        FatArrow,
        #[token(r"[+*/%=<>{}(),;:.-]")]
        Punctuation,
    }
}
