use lxr::Lexer;

#[derive(Lexer)]
enum Token<'input> {
    #[token("[a-z]+")]
    Word(&'input str),
}

fn main() {}
