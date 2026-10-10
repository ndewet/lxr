use crate::support::token_definition;

pub const SOURCE: &str = include_str!("../sources/java_like.java");

token_definition! {
    #[lexer(skip = r"[ \t\r\n]+")]
    #[lexer(skip = r"//[^\n]*")]
    #[lexer(skip = r"/\*([^*]|\*[^/])*\*/")]
    pub enum Token {
        #[token("public")]
        Public,
        #[token("class")]
        Class,
        #[token("static")]
        Static,
        #[token("void")]
        Void,
        #[token("int")]
        Int,
        #[token("boolean")]
        Boolean,
        #[token("if")]
        If,
        #[token("else")]
        Else,
        #[token("while")]
        While,
        #[token("return")]
        Return,
        #[token("new")]
        New,
        #[token("true")]
        True,
        #[token("false")]
        False,
        #[token("null")]
        Null,
        #[token("[A-Za-z_$][A-Za-z0-9_$]*")]
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
        #[token("&&")]
        LogicalAnd,
        #[token(r"\|\|")]
        LogicalOr,
        #[token(r"[+*/%=<>{}(),;:.-]")]
        Punctuation,
    }
}
