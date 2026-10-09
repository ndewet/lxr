use lxr::Lexer;

#[derive(Lexer)]
#[lexer(max_token_bytes = 0)]
enum Token {
    #[token("x")]
    X,
}

fn main() {}
