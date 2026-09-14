//! Defines the lxr derive macro.
//!
//! The macro reads token enums and emits their lexer implementations.

#![deny(dead_code)]

use lxr_codegen::{RuleSpec, compile};
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Data, DeriveInput, Error, Expr, Fields, Lit, LitStr, MetaNameValue, Result, Type,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

/// Derives [`lxr::Lexer`](::lxr::Lexer) for an enum of token kinds.
///
/// Every variant needs one `#[lxr("pattern")]` attribute. Unit variants
/// emit no payload; a single-field tuple variant parses an owned payload.
/// Rules are considered in declaration order when equal-length matches tie.
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

    let mut rules = skip_rules(&input.attrs)?;
    for variant in data.variants {
        let RuleAttribute { pattern, converter } = rule_attribute(&variant.attrs, &variant.ident)?;
        let variant_ident = variant.ident;
        let action = match variant.fields {
            Fields::Unit => {
                if converter.is_some() {
                    return Err(Error::new_spanned(
                        variant_ident,
                        "unit token variants cannot have a payload converter",
                    ));
                }
                quote!(Ok(Some(Self::#variant_ident)))
            }
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let field = fields.unnamed.first().expect("one field was checked");
                let Type::Reference(_) = field.ty else {
                    let payload_type = &field.ty;
                    let action = match converter {
                        Some(converter) => quote! {
                            (#converter)(text)
                                .map(|payload| Some(Self::#variant_ident(payload)))
                                .map_err(|error| ::lxr::PayloadError::new(error.to_string()))
                        },
                        None => quote! {
                            ::lxr::parse_payload::<#payload_type>(text)
                                .map(|payload| Some(Self::#variant_ident(payload)))
                        },
                    };
                    rules.push(RuleSpec::emit(pattern.value(), action));
                    continue;
                };
                return Err(Error::new_spanned(field, "token payloads must be owned"));
            }
            Fields::Unnamed(_) => {
                return Err(Error::new_spanned(
                    variant_ident,
                    "lexer token variants may have at most one tuple payload",
                ));
            }
            Fields::Named(_) => {
                return Err(Error::new_spanned(
                    variant_ident,
                    "lexer token variants must be unit variants or have one tuple payload",
                ));
            }
        };
        rules.push(RuleSpec::emit(pattern.value(), action));
    }

    let matcher = compile(rules).map_err(|error| Error::new_spanned(input.ident.clone(), error))?;
    let ident = input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics #ident #type_generics #where_clause {
            #matcher
        }

        impl #impl_generics ::lxr::Lexer for #ident #type_generics #where_clause {
            fn scan_one(
                input: &str,
            ) -> Option<::lxr::RuleScan<Self>> {
                let (rule, length) = Self::__lxr_scan(input.as_bytes(), 0)?;
                Some(
                    Self::__lxr_action(rule, &input[..length])
                        .map(|token| (token, length))
                        .map_err(|error| (error, length)),
                )
            }
        }
    })
}

fn skip_rules(attributes: &[syn::Attribute]) -> Result<Vec<RuleSpec>> {
    attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("lxr"))
        .map(|attribute| {
            let MetaNameValue { path, value, .. } = attribute.parse_args()?;
            if !path.is_ident("skip") {
                return Err(Error::new_spanned(path, "expected `skip = \"pattern\"`"));
            }
            let Expr::Lit(expression) = value else {
                return Err(Error::new_spanned(
                    value,
                    "a skipped pattern must be a string literal",
                ));
            };
            let Lit::Str(pattern) = expression.lit else {
                return Err(Error::new_spanned(
                    expression,
                    "a skipped pattern must be a string literal",
                ));
            };
            Ok(RuleSpec::skip(pattern.value()))
        })
        .collect()
}

struct RuleAttribute {
    pattern: LitStr,
    converter: Option<Expr>,
}

impl Parse for RuleAttribute {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let pattern = input.parse()?;
        let converter = if input.is_empty() {
            None
        } else {
            input.parse::<syn::Token![,]>()?;
            Some(input.parse()?)
        };
        Ok(Self { pattern, converter })
    }
}

fn rule_attribute(attributes: &[syn::Attribute], variant: &syn::Ident) -> Result<RuleAttribute> {
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
