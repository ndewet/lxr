//! Defines the lxr derive macro.
//!
//! The macro reads token enums and emits their lexer implementations.

#![deny(dead_code)]

use lxr_codegen::{RuleSpec, Transition, compile_with_modes};
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Data, DeriveInput, Error, Expr, Fields, LitStr, Result, Type,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

/// Derives the `lxr::Lexer` trait for an enum of token kinds.
///
/// Every variant needs one `#[lxr("pattern")]` attribute. Unit variants
/// emit no payload; a single-field tuple variant parses an owned payload.
/// Rules are considered in declaration order when equal-length matches tie.
///
/// # Examples
///
/// ```
/// # extern crate self as lxr;
/// # pub struct PayloadError;
/// # impl PayloadError {
/// #     pub fn new(_: impl Into<String>) -> Self {
/// #         Self
/// #     }
/// # }
/// # pub enum Transition {
/// #     Stay,
/// #     Begin(usize),
/// #     Push(usize),
/// #     Pop,
/// # }
/// # pub type RuleScan<T> = Result<(Option<T>, usize, Transition), (PayloadError, usize)>;
/// # pub mod __private { pub trait Sealed {} }
/// # pub trait Lexer: __private::Sealed + Sized {
/// #     fn scan_one(input: &str, mode: usize) -> Option<RuleScan<Self>>;
/// #     fn mode_name(mode: usize) -> &'static str;
/// # }
/// # fn main() {
/// use lxr_derive::Lexer;
///
/// #[derive(Debug, PartialEq, Lexer)]
/// enum Token {
///     #[lxr("[a-z]+")]
///     Word,
/// }
///
/// assert!(<Token as lxr::Lexer>::scan_one("word", 0).is_some());
/// # }
/// ```
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

    let (modes, mut rules) = lexer_attributes(&input.attrs)?;
    for variant in data.variants {
        let attribute = rule_attribute(&variant.attrs, &variant.ident)?;
        let pattern = attribute.pattern.clone();
        let converter = attribute.converter;
        let variant_ident = variant.ident;
        let action = match variant.fields {
            Fields::Unit => match converter {
                Some(converter) => quote! {
                    ::lxr::PayloadResult::<()>::into_payload((#converter)(text))
                        .map(|()| Some(Self::#variant_ident))
                },
                None => quote!(Ok(Some(Self::#variant_ident))),
            },
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let field = fields.unnamed.first().expect("one field was checked");
                let Type::Reference(_) = field.ty else {
                    let payload_type = &field.ty;
                    let action = match converter {
                        Some(converter) => quote! {
                            ::lxr::PayloadResult::<#payload_type>::into_payload(
                                (#converter)(text),
                            )
                            .map(|payload| Some(Self::#variant_ident(payload)))
                        },
                        None => quote! {
                            ::lxr::parse_payload::<#payload_type>(text)
                                .map(|payload| Some(Self::#variant_ident(payload)))
                        },
                    };
                    rules.push(rule_spec(
                        RuleSpec::emit(pattern.value(), action),
                        &attribute.modes,
                        &attribute.transition,
                    ));
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
        rules.push(rule_spec(
            RuleSpec::emit(pattern.value(), action),
            &attribute.modes,
            &attribute.transition,
        ));
    }

    let matcher = compile_with_modes(modes, rules)
        .map_err(|error| Error::new_spanned(input.ident.clone(), error))?;
    let ident = input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics #ident #type_generics #where_clause {
            #matcher
        }

        impl #impl_generics ::lxr::__private::Sealed for #ident #type_generics #where_clause {}

        impl #impl_generics ::lxr::Lexer for #ident #type_generics #where_clause {
            fn scan_one(
                input: &str,
                mode: usize,
            ) -> Option<::lxr::RuleScan<Self>> {
                let (rule, length) = Self::__lxr_scan(input.as_bytes(), mode)?;
                Some(
                    Self::__lxr_action(rule, &input[..length])
                        .map(|(token, transition)| (token, length, transition))
                        .map_err(|error| (error, length)),
                )
            }

            fn mode_name(mode: usize) -> &'static str {
                Self::__lxr_mode_name(mode)
            }
        }
    })
}

fn lexer_attributes(attributes: &[syn::Attribute]) -> Result<(Vec<String>, Vec<RuleSpec>)> {
    let mut modes = Vec::new();
    let mut rules = Vec::new();
    attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("lxr"))
        .try_for_each(|attribute| {
            let config: LexerAttribute = attribute.parse_args()?;
            if config.name == "mode" {
                let Some(mode) = config.mode else {
                    unreachable!()
                };
                modes.push(mode);
                return Ok(());
            }
            if config.name != "skip" {
                return Err(Error::new_spanned(
                    attribute,
                    "expected `mode = Name` or `skip = \"pattern\"`",
                ));
            }
            let pattern = config.pattern.expect("skip attribute has a pattern");
            let spec = match &config.rule.converter {
                Some(converter) => RuleSpec::emit(
                    pattern,
                    quote! {
                        ::lxr::PayloadResult::<()>::into_payload((#converter)(text))
                            .map(|()| None)
                    },
                ),
                None => RuleSpec::skip(pattern),
            };
            rules.push(rule_spec(spec, &config.rule.modes, &config.rule.transition));
            Ok(())
        })?;
    Ok((modes, rules))
}

struct LexerAttribute {
    name: String,
    mode: Option<String>,
    pattern: Option<String>,
    rule: RuleConfig,
}

struct RuleConfig {
    modes: Vec<String>,
    transition: Transition,
    converter: Option<Expr>,
}

impl Parse for LexerAttribute {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name: syn::Ident = input.parse()?;
        input.parse::<syn::Token![=]>()?;
        if name == "mode" {
            let mode: syn::Ident = input.parse()?;
            if !input.is_empty() {
                return Err(input.error("unexpected lexer mode attribute content"));
            }
            return Ok(Self {
                name: name.to_string(),
                mode: Some(mode.to_string()),
                pattern: None,
                rule: RuleConfig::stay(),
            });
        }
        let pattern: LitStr = input.parse()?;
        let rule = parse_rule_modifiers(input)?;
        Ok(Self {
            name: name.to_string(),
            mode: None,
            pattern: Some(pattern.value()),
            rule,
        })
    }
}

impl RuleConfig {
    fn stay() -> Self {
        Self {
            modes: vec!["INITIAL".into()],
            transition: Transition::Stay,
            converter: None,
        }
    }
}

fn parse_rule_modifiers(input: ParseStream<'_>) -> Result<RuleConfig> {
    let mut config = RuleConfig::stay();
    while !input.is_empty() {
        input.parse::<syn::Token![,]>()?;
        let name: syn::Ident = input.parse()?;
        match name.to_string().as_str() {
            "modes" => {
                input.parse::<syn::Token![=]>()?;
                config.modes = parse_modes(input)?;
            }
            "push" => {
                input.parse::<syn::Token![=]>()?;
                config.transition = Transition::Push(input.parse::<syn::Ident>()?.to_string());
            }
            "begin" => {
                input.parse::<syn::Token![=]>()?;
                config.transition = Transition::Begin(input.parse::<syn::Ident>()?.to_string());
            }
            "with" => {
                input.parse::<syn::Token![=]>()?;
                config.converter = Some(input.parse()?);
            }
            "pop" => config.transition = Transition::Pop,
            _ => {
                return Err(Error::new_spanned(
                    name,
                    "expected `modes`, `with`, `push`, `begin`, or `pop`",
                ));
            }
        }
    }
    Ok(config)
}

fn parse_modes(input: ParseStream<'_>) -> Result<Vec<String>> {
    if input.peek(syn::token::Bracket) {
        let content;
        syn::bracketed!(content in input);
        let modes = content.parse_terminated(syn::Ident::parse, syn::Token![,])?;
        Ok(modes.into_iter().map(|mode| mode.to_string()).collect())
    } else {
        Ok(vec![input.parse::<syn::Ident>()?.to_string()])
    }
}

fn rule_spec(spec: RuleSpec, modes: &[String], transition: &Transition) -> RuleSpec {
    spec.in_modes(modes.to_vec()).transition(transition.clone())
}

struct RuleAttribute {
    pattern: LitStr,
    converter: Option<Expr>,
    modes: Vec<String>,
    transition: Transition,
}

impl Parse for RuleAttribute {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let pattern = input.parse()?;
        let config = parse_rule_modifiers(input)?;
        Ok(Self {
            pattern,
            converter: config.converter,
            modes: config.modes,
            transition: config.transition,
        })
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
