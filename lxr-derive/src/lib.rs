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
mod specification;

/// Derives a lexer from an enum of tokens.
///
/// Write one `#[lxr(...)]` attribute for each rule. A rule holds one pattern:
///
/// - `token = "fn"` matches the literal. A regex character in it needs no escape.
/// - `regex = "[a-z]+"` matches the regular expression.
/// - `skip = "[ \t]+"` reads the match and gives no token. Write it on the enum.
///
/// A rule takes two more options:
///
/// - `in = [Context::Text]` gives the start conditions of the rule. It defaults to the first
///   condition.
/// - `go = Context::Code` changes the start condition after the match.
///
/// Write `#[lxr(condition = Context::Code)]` on the enum to name the start conditions. The type of
/// the conditions is the path without its last segment, and the path is the condition at which the
/// scan begins.
///
/// The rules are in the sequence of precedence. The longest match wins, and the earliest rule wins
/// a tie. A rule of a variant comes before a rule that skips.
///
/// # Examples
///
/// ```ignore
/// use lxr::Lexer;
///
/// #[derive(Clone, Copy)]
/// enum Context {
///     Code,
///     Text,
/// }
///
/// #[derive(Debug, PartialEq, Lexer)]
/// #[lxr(condition = Context::Code)]
/// #[lxr(skip = "[ \t\n]+")]
/// enum Token {
///     #[lxr(token = "fn")]
///     Function,
///     #[lxr(regex = "[a-z][a-z0-9]*")]
///     Word,
///     #[lxr(token = "\"", go = Context::Text)]
///     Quote,
///     #[lxr(regex = "[^\"]+", in = [Context::Text])]
///     Text,
///     #[lxr(token = "\"", in = [Context::Text], go = Context::Code)]
///     End,
/// }
/// ```
#[proc_macro_derive(Lexer, attributes(lxr))]
pub fn lexer(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let read = match specification::read(&input) {
        Ok(read) => read,
        Err(error) => return error.to_compile_error().into(),
    };

    match generate(&read.specification) {
        Ok(source) => source.into(),
        Err(errors) => {
            let reports = errors
                .iter()
                .map(|error| report(error, &read.spans, read.name));
            quote!(#(#reports)*).into()
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
