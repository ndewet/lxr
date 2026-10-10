pub mod generation;
pub mod scanner;

macro_rules! token_definition {
    (
        $(#[lexer(skip = $skip:literal)])*
        $visibility:vis enum $token:ident {
            $(#[token($pattern:literal)] $variant:ident),* $(,)?
        }
    ) => {
        #[derive(runeweaver::Lexer)]
        $(#[lexer(skip = $skip)])*
        $visibility enum $token {
            $(#[token($pattern)] $variant),*
        }

        pub fn definition() -> Vec<runeweaver_codegen::RuleSpec> {
            let _ = std::marker::PhantomData::<$token>;
            let mut rules = vec![
                $(runeweaver_codegen::RuleSpec::skip($skip)),*
            ];
            rules.extend([
                $(runeweaver_codegen::RuleSpec::emit(
                    $pattern,
                    quote::quote!(Ok(Some(Self::$variant))),
                )),*
            ]);
            rules
        }
    };
}

pub(crate) use token_definition;
