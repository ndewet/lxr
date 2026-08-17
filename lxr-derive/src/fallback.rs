use proc_macro2::{Ident, TokenStream};
use quote::quote;

/// Returns an `impl` of the runtime trait for `token`, which matches nothing.
///
/// A fault gives a `compile_error!` and no lexer. Each call of `scan` then fails as well, and the
/// compiler reports a trait that the author never named. This `impl` keeps each of those calls
/// valid, thus one fault gives one error.
///
/// `condition` is the type of the start conditions of the author, or `None` if the macro did not
/// read it. The tables hold the dead state alone. Thus a scan of them reports each character of the
/// input, and it panics never.
pub fn fallback(token: &Ident, condition: Option<&TokenStream>) -> TokenStream {
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
            static ACCEPT: [u16; 1] = [0];
            static START: [u16; 1] = [0];
            static ACTIONS: [::lxr::Action; 0] = [];

            #[automatically_derived]
            impl ::lxr::Lexer for #token {
                type Condition = #kind;

                const TABLES: ::lxr::Tables<'static> = ::lxr::Tables {
                    classes: &CLASSES,
                    next: &NEXT,
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
    use proc_macro2::Span;

    /// Returns an identifier of `name` at the span of the call.
    fn name(name: &str) -> Ident {
        Ident::new(name, Span::call_site())
    }

    /// Returns `true` if the text of `source` holds the text of `part`.
    fn holds(source: &TokenStream, part: &TokenStream) -> bool {
        source.to_string().contains(&part.to_string())
    }

    #[test]
    fn the_fallback_implements_the_runtime_trait_for_the_enum_of_the_tokens() {
        let source = fallback(&name("Token"), None);

        assert!(holds(&source, &quote!(impl ::lxr::Lexer for Token)));
        assert!(holds(
            &source,
            &quote!(
                type Condition = ();
            )
        ));
    }

    #[test]
    fn the_tables_of_the_fallback_hold_the_dead_state_alone() {
        let source = fallback(&name("Token"), None);

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
        let source = fallback(&name("Token"), Some(&quote!(Context)));

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
        let source = fallback(&name("Token"), None).to_string();

        assert!(source.starts_with(&quote!(const _: () =).to_string()));
    }
}
