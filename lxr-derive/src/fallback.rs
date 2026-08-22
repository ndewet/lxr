use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

/// Returns an `impl` of the runtime trait for the type of `input`, which matches nothing.
///
/// A fault gives a `compile_error!` and no lexer. Each call of `scan` then fails as well, and the
/// compiler reports a trait that the author never named. This `impl` keeps each of those calls
/// valid, thus one fault gives one error.
///
/// The `impl` carries the generic parameters of `input`. lxr rejects a token enum that holds one,
/// and this `impl` stands in for that enum as well, thus it names the type as the author wrote it.
///
/// `condition` is the type of the start conditions of the author, or `None` if the macro did not
/// read it. The tables hold the dead state alone. Thus a scan of them reports each character of the
/// input, and it panics never.
pub fn fallback(input: &DeriveInput, condition: Option<&TokenStream>) -> TokenStream {
    let token = &input.ident;
    let (parameters, arguments, bounds) = input.generics.split_for_impl();
    let kind = condition.cloned().unwrap_or_else(|| quote!(()));
    let of_index = match condition {
        Some(_) => quote! {
            fn condition(index: u16) -> #kind {
                panic!("condition {index} is not a start condition of this lexer")
            }
        },
        None => quote! {
            fn condition(_index: u16) {}
        },
    };

    quote! {
        const _: () = {
            static CLASSES: [u16; 256] = [0; 256];
            static NEXT: [u16; 1] = [0];
            static REPEATS: [u64; 4] = [0; 4];
            static LEAVES: [u64; 1] = [0];
            static ACCEPT: [u16; 1] = [0];
            static START: [u16; 1] = [0];
            static ACTIONS: [::lxr::Action; 0] = [];

            #[automatically_derived]
            impl #parameters ::lxr::Lexer for #token #arguments #bounds {
                type Condition = #kind;

                const TABLES: ::lxr::Tables<'static> = ::lxr::Tables {
                    classes: &CLASSES,
                    next: &NEXT,
                    repeats: &REPEATS,
                    leaves: &LEAVES,
                    width: 1,
                    accept: &ACCEPT,
                    start: &START,
                    actions: &ACTIONS,
                };

                fn token(rule: u16, _text: &str) -> ::core::option::Option<Self> {
                    panic!("rule {rule} of this lexer gives no token")
                }

                #of_index
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns the enum of the tokens that `source` declares.
    fn tokens(source: &str) -> DeriveInput {
        syn::parse_str(source).expect("the source is an item")
    }

    /// Returns the enum `Token`, which holds no generic parameter and no rule.
    fn simple() -> DeriveInput {
        tokens("enum Token { Word }")
    }

    /// Returns `true` if the text of `source` holds the text of `part`.
    fn holds(source: &TokenStream, part: &TokenStream) -> bool {
        source.to_string().contains(&part.to_string())
    }

    #[test]
    fn the_fallback_implements_the_runtime_trait_for_the_enum_of_the_tokens() {
        let source = fallback(&simple(), None);

        assert!(holds(&source, &quote!(impl ::lxr::Lexer for Token)));
        assert!(holds(
            &source,
            &quote!(
                type Condition = ();
            )
        ));
    }

    #[test]
    fn the_fallback_names_the_generic_parameters_of_the_enum() {
        let source = fallback(&tokens("enum Token<'a, T: Copy> { Word(&'a T) }"), None);

        assert!(holds(
            &source,
            &quote!(impl<'a, T: Copy> ::lxr::Lexer for Token<'a, T>)
        ));
    }

    #[test]
    fn the_fallback_keeps_the_bounds_of_the_enum() {
        let source = fallback(&tokens("enum Token<T> where T: Copy { Word(T) }"), None);

        assert!(holds(&source, &quote!(for Token<T> where T: Copy)));
    }

    #[test]
    fn the_tables_of_the_fallback_hold_the_dead_state_alone() {
        let source = fallback(&simple(), None);

        assert!(holds(
            &source,
            &quote!(
                static NEXT: [u16; 1] = [0];
            )
        ));
        assert!(holds(
            &source,
            &quote!(
                static ACCEPT: [u16; 1] = [0];
            )
        ));
        assert!(holds(&source, &quote!(width: 1)));
    }

    #[test]
    fn the_fallback_keeps_the_start_conditions_of_the_author() {
        let source = fallback(&simple(), Some(&quote!(Context)));

        assert!(holds(
            &source,
            &quote!(
                type Condition = Context;
            )
        ));
        assert!(holds(&source, &quote!(fn condition(index: u16) -> Context)));
    }

    #[test]
    fn the_statics_of_the_fallback_live_inside_an_anonymous_const() {
        let source = fallback(&simple(), None).to_string();

        assert!(source.starts_with(&quote!(const _: () =).to_string()));
    }
}
