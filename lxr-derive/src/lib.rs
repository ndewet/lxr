//! Defines the lxr derive macro.
//!
//! The macro reads token enums and emits their lexer implementations.

use lxr_codegen::{Lexer, Rule, RuleAction, StartCondition, StartConditionId};
use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, LitStr, Result, parse_macro_input};

/// Derives [`lxr::Lexer`](::lxr::Lexer) for an enum of token kinds.
///
/// Every unit variant needs one `#[lxr("pattern")]` attribute. Rules are
/// considered in declaration order when equal-length matches tie.
#[proc_macro_derive(Lexer, attributes(lxr))]
pub fn derive_lexer(input: TokenStream) -> TokenStream {
    match derive_lexer_inner(parse_macro_input!(input as DeriveInput)) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn derive_lexer_inner(input: DeriveInput) -> Result<proc_macro2::TokenStream> {
    let Data::Enum(data) = input.data else {
        return Err(Error::new_spanned(
            input.ident,
            "`Lexer` can only be derived for enums",
        ));
    };

    let mut rules = Vec::with_capacity(data.variants.len());
    for variant in data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(Error::new_spanned(
                variant.ident,
                "lexer token variants must be unit variants",
            ));
        }
        let pattern = rule_pattern(&variant.attrs, &variant.ident)?;
        let variant_ident = variant.ident;
        let action = RuleAction::new(quote!(Self::#variant_ident));
        let rule = Rule::parse(&pattern.value(), action, vec![StartConditionId::new(0)])
            .map_err(|error| Error::new_spanned(pattern, error))?;
        rules.push(rule);
    }

    let lexer = Lexer::new(rules, vec![StartCondition::new("INITIAL")]);
    let matcher = lexer
        .emit()
        .map_err(|error| Error::new_spanned(input.ident.clone(), error))?;
    let ident = input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics #ident #type_generics #where_clause {
            #matcher
        }

        impl #impl_generics ::lxr::Lexer for #ident #type_generics #where_clause {
            fn scan(input: &str) -> Option<(Self, usize)> {
                Self::__lxr_scan(input.as_bytes(), 0)
            }
        }
    })
}

fn rule_pattern(attributes: &[syn::Attribute], variant: &syn::Ident) -> Result<LitStr> {
    let mut patterns = attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("lxr"));
    let Some(attribute) = patterns.next() else {
        return Err(Error::new_spanned(
            variant,
            "lexer token variants need one `#[lxr(\"pattern\")]` attribute",
        ));
    };
    if patterns.next().is_some() {
        return Err(Error::new_spanned(
            variant,
            "lexer token variants can have only one `#[lxr(...)]` attribute",
        ));
    }
    attribute.parse_args()
}
