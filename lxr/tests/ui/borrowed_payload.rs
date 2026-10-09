use lxr::Lexer;

#[derive(Lexer)]
enum Token<'input> {
    #[lxr("[a-z]+")]
    Word(&'input str),
}

fn main() {}
