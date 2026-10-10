use crate::support::token_definition;

pub const SOURCE: &str = include_str!("../sources/keywords.txt");

token_definition! {
    #[lexer(skip = r"[ \t\r\n]+")]
    pub enum Token {
        #[token("as")]
        As,
        #[token("async")]
        Async,
        #[token("await")]
        Await,
        #[token("break")]
        Break,
        #[token("const")]
        Const,
        #[token("continue")]
        Continue,
        #[token("else")]
        Else,
        #[token("enum")]
        Enum,
        #[token("false")]
        False,
        #[token("fn")]
        Function,
        #[token("for")]
        For,
        #[token("if")]
        If,
        #[token("impl")]
        Impl,
        #[token("in")]
        In,
        #[token("let")]
        Let,
        #[token("loop")]
        Loop,
        #[token("match")]
        Match,
        #[token("mod")]
        Module,
        #[token("move")]
        Move,
        #[token("mut")]
        Mut,
        #[token("pub")]
        Public,
        #[token("ref")]
        Reference,
        #[token("return")]
        Return,
        #[token("self")]
        SelfValue,
        #[token("static")]
        Static,
        #[token("struct")]
        Struct,
        #[token("trait")]
        Trait,
        #[token("true")]
        True,
        #[token("type")]
        Type,
        #[token("unsafe")]
        Unsafe,
        #[token("use")]
        Use,
        #[token("where")]
        Where,
        #[token("while")]
        While,
        #[token("[A-Za-z_][A-Za-z0-9_]*")]
        Identifier,
    }
}
