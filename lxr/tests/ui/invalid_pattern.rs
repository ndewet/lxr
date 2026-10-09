use lxr::Lexer;

#[derive(Lexer)]
enum Token {
    #[lxr("[")]
    Word,
}

fn main() {}
