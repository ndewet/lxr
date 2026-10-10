use crate::support::token_definition;

pub const SOURCE: &str = include_str!("../sources/example.xml");

token_definition! {
    #[lexer(skip = r"[ \t\r\n]+")]
    pub enum Token {
        #[token(r"<\?xml")]
        DeclarationOpen,
        #[token(r"\?>")]
        DeclarationClose,
        #[token("</")]
        EndTagOpen,
        #[token("/>")]
        SelfClose,
        #[token("<")]
        TagOpen,
        #[token(">")]
        TagClose,
        #[token("=")]
        Equal,
        #[token(r#""([^"\\]|\\.)*""#)]
        String,
        #[token("[A-Za-z_:][A-Za-z0-9_.:-]*")]
        Name,
    }
}
