//! Emits the source of a lexer from its rule graph.
//!
//! [`emit`] gives the `impl` of the [`Lexer`] trait of the runtime crate. It holds the step that
//! [`code`](crate::code) writes, and it maps each index of a start condition onto the name that
//! the author wrote. The derive macro places the result in the crate of the lexer author.
//!
//! The emitted source names the runtime as `::lxr`, thus it does not depend on what the author
//! imported.
//!
//! [`Lexer`]: https://docs.rs/lxr/latest/lxr/trait.Lexer.html

use proc_macro2::{Ident, Literal, TokenStream};
use quote::quote;

use crate::code::Rule;

/// The maximum number of the rules of a lexer.
///
/// A leaf of the graph names its rule in a [`u16`], thus the lexer holds at most that many rules.
pub const MAX_RULES: usize = u16::MAX as usize;

/// A lexer that is ready to emit.
///
/// [`rules`](Self::rules) is in the sequence of the rules of the lexicon, thus the rule at the
/// index `n` is the rule that a leaf of the graph names `n`.
#[derive(Debug, Clone)]
pub struct Emission {
    /// The name of the enum of the tokens.
    pub token: Ident,
    /// The type of the start conditions, or `None` if the lexer reads under one condition.
    pub condition: Option<TokenStream>,
    /// One name for each start condition, in the sequence of the indexes.
    ///
    /// A name serves as the body of a match arm, thus it is an expression of the type of
    /// [`condition`](Self::condition).
    ///
    /// A lexer that reads under one condition holds an empty list.
    pub conditions: Vec<TokenStream>,
    /// What each rule gives.
    pub rules: Vec<Rule>,
    /// The number of the start conditions of the graph.
    pub starts: usize,
    /// The scan of the lexer as code.
    ///
    /// [`code::step`](crate::code::step) writes it.
    pub step: TokenStream,
}

/// Emits the `impl` of the runtime trait for the token enum of `lexer`.
///
/// The `impl` lives inside an anonymous `const`, thus two lexers in one module do not collide.
///
/// # Panics
///
/// This function panics if `lexer` breaks a condition of [`check`]. The emitted source would then
/// fail in the crate of the author, and the message there would name no cause.
pub fn emit(lexer: &Emission) -> TokenStream {
    check(lexer);

    let token = &lexer.token;
    let condition = condition_type(lexer);
    let of_index = of_index(lexer);
    let step = &lexer.step;

    quote! {
        const _: () = {
            #[automatically_derived]
            impl ::lxr::Lexer for #token {
                type Condition = #condition;

                #step
                #of_index
            }
        };
    }
}

/// Checks that the parts of `lexer` agree with each other.
///
/// The graph and the rules arrive from two places, and the emitted source joins them. A
/// disagreement compiles here, and it breaks in the crate of the author.
///
/// # Panics
///
/// This function panics if any of these conditions fails:
///
/// - The `go` of each rule is a start condition of the graph.
/// - The names of the conditions number one for each start condition of the graph.
fn check(lexer: &Emission) {
    for (index, rule) in lexer.rules.iter().enumerate() {
        if let Some(go) = rule.go {
            assert!(
                usize::from(go) < lexer.starts,
                "rule {index} goes to the start condition {go}, and the lexer holds {} of them",
                lexer.starts
            );
        }
    }

    let named = lexer.conditions.len();
    let expected = if lexer.condition.is_some() {
        lexer.starts
    } else {
        0
    };
    assert_eq!(
        named, expected,
        "a lexer of {} start conditions needs {expected} names for them, and not {named}",
        lexer.starts
    );
}

/// Returns the type of the start conditions of `lexer`.
fn condition_type(lexer: &Emission) -> TokenStream {
    lexer.condition.clone().unwrap_or_else(|| quote!(()))
}

/// Returns the `condition` function, which maps an index onto the start condition at it.
fn of_index(lexer: &Emission) -> TokenStream {
    if lexer.condition.is_none() {
        return quote! {
            fn condition(_index: u16) {}
        };
    }

    let condition = condition_type(lexer);
    let arms = lexer.conditions.iter().enumerate().map(|(index, name)| {
        let index = Literal::usize_unsuffixed(index);
        quote!(#index => #name)
    });

    quote! {
        fn condition(index: u16) -> #condition {
            match index {
                #(#arms,)*
                index => panic!("condition {index} is not a start condition of this lexer"),
            }
        }
    }
}
