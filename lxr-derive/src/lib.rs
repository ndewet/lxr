//! Derives a lexer from an enum of tokens.
//!
//! The crate is the thin half of the macro. It parses the attributes with `syn`, then it gives the
//! lexer to `lxr-codegen`, which builds the automaton and emits the source.
//!
//! Do not depend on this crate. `lxr` re-exports the macro, and the emitted source calls the
//! runtime of `lxr`.

use lxr_codegen::{GenerateError, GenerateErrorKind, generate};
use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, LitStr, parse_macro_input};

mod attribute;
mod fallback;
mod specification;

use self::fallback::fallback;

/// Derives a lexer from an enum of tokens.
///
/// Write one `#[lxr(...)]` attribute for each rule, and the macro implements the `Lexer` trait of
/// the runtime. `token` matches a literal, `regex` matches a regular expression, and `skip` reads
/// the match and gives no token. `in` and `go` carry the start conditions.
///
/// Read the documentation of `lxr`, which re-exports this macro. The module `lxr::syntax` holds
/// each attribute, the sequence of the rules, the pattern language, and each limit. This crate
/// holds no example, because an example of the macro needs the runtime.
#[proc_macro_derive(Lexer, attributes(lxr))]
pub fn lexer(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let read = match specification::read(&input) {
        Ok(read) => read,
        Err(fault) => {
            let report = fault.error.to_compile_error();
            let fallback = fallback(&input, fault.condition.as_ref());
            return quote!(#report #fallback).into();
        }
    };

    match generate(&read.specification) {
        Ok(source) => source.into(),
        Err(errors) => {
            let reports = errors
                .iter()
                .map(|error| report(error, &read.spans, read.name));
            let fallback = fallback(
                &input,
                read.specification
                    .conditions
                    .as_ref()
                    .map(|conditions| &conditions.kind),
            );
            quote!(#(#reports)* #fallback).into()
        }
    }
}

/// Returns the `compile_error!` of `error`, at the span of the rule at fault.
///
/// A fault of the whole lexer marks the name of the enum.
fn report(
    error: &GenerateError,
    spans: &[LitStr],
    name: proc_macro2::Span,
) -> proc_macro2::TokenStream {
    let span = error
        .rule
        .and_then(|rule| spans.get(rule))
        .map_or(name, LitStr::span);

    syn::Error::new(span, message(error)).to_compile_error()
}

/// Returns the text of the message of `error`.
///
/// `Literal::subspan` is not stable, thus the macro cannot mark one part of the pattern. It gives
/// the bytes of the fault in the text instead, and it marks the whole literal.
fn message(error: &GenerateError) -> String {
    let mut message = error.kind.to_string();

    if let GenerateErrorKind::Pattern(parse) = &error.kind {
        message.push_str(&format!(
            "\nthe fault is at the bytes {}..{} of the pattern",
            parse.span.start, parse.span.end
        ));
    }
    if let Some(help) = error.kind.help() {
        message.push_str(&format!("\nhelp: {help}"));
    }

    message
}
