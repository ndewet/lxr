use lxr::Lexer;

#[derive(Lexer)]
enum Token {
    #[token("[")]
    Word,
}

fn main() {}
