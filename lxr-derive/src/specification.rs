use lxr_codegen::{Conditions, Pattern, Rule, Specification};
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Fields, Ident, LitStr, Path, Token, Variant};

use crate::attribute::Attribute;

/// The variant that a rule gives, and the type of the field of that variant.
#[derive(Clone)]
struct Gives {
    /// The name of the variant.
    variant: Ident,
    /// The type of its field, or `None` if the variant holds no field.
    value: Option<TokenStream>,
}

/// The lexer that an enum of tokens describes, and the span of each of its parts.
///
/// [`generate`](lxr_codegen::generate) reports the index of the rule at fault.
/// [`spans`](Self::spans) turns that index into the span of the pattern, thus the compiler marks
/// the rule that the author wrote.
pub struct Read {
    /// The lexer, in the form that the codegen crate reads.
    pub specification: Specification,
    /// The literal of the pattern of each rule, in the sequence of the rules.
    pub spans: Vec<LitStr>,
    /// The name of the enum of the tokens.
    pub name: Span,
}

/// Reads the lexer that `input` describes.
///
/// A rule of a variant comes before a rule of the container, thus a token wins a tie against a
/// rule that skips.
///
/// # Errors
///
/// This function returns an error for each of these faults:
///
/// - `input` is not an enum, or a variant holds a named field or two fields.
/// - An attribute names an option that lxr does not hold, or it names one two times.
/// - A variant holds no rule, or the lexer holds no rule.
/// - An attribute names a start condition and no pattern.
/// - A rule on the enum gives a token, or a rule on a variant skips.
/// - A variant names the first start condition, which belongs to the enum.
/// - A rule names a start condition, and the lexer names no condition enum.
pub fn read(input: &DeriveInput) -> syn::Result<Read> {
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new(
            input.ident.span(),
            "lxr derives a lexer from an enum of tokens",
        ));
    };

    let mut rules = Vec::new();
    let mut spans = Vec::new();
    let mut skips = Vec::new();
    let mut skip_spans = Vec::new();
    let mut initial = None;

    for attribute in options(&input.attrs)? {
        if let Some(condition) = &attribute.condition {
            if initial.is_some() {
                return Err(syn::Error::new(
                    condition.span(),
                    "the lexer already names its first start condition",
                ));
            }
            initial = Some(condition.clone());
        }
        attached(&attribute)?;
        let named = attribute
            .pattern()?
            .map(|(literal, skip)| (literal.clone(), skip));
        if let Some((literal, skip)) = named {
            if !skip {
                return Err(syn::Error::new(
                    literal.span(),
                    "a rule on the enum gives no token. Write `skip`, or write the rule on a \
                     variant",
                ));
            }
            skip_spans.push(literal);
            skips.push(attribute);
        }
    }

    for variant in &data.variants {
        let gives = Gives {
            variant: variant.ident.clone(),
            value: field(variant)?,
        };
        let mut ruled = false;
        for attribute in options(&variant.attrs)? {
            if let Some(path) = &attribute.condition {
                return Err(syn::Error::new(
                    path.span(),
                    "the first start condition belongs to the lexer. Write \
                     `#[lxr(condition = ...)]` on the enum, and not on a variant",
                ));
            }
            attached(&attribute)?;
            let Some((literal, skip)) = attribute.pattern()? else {
                continue;
            };
            if skip {
                return Err(syn::Error::new(
                    literal.span(),
                    "a rule that skips gives no token. Write it on the enum, and not on a variant",
                ));
            }
            spans.push(literal.clone());
            ruled = true;
            rules.push((attribute, Some(gives.clone())));
        }
        if !ruled {
            return Err(syn::Error::new(
                variant.ident.span(),
                "the variant holds no rule. Write `#[lxr(token = \"...\")]` or \
                 `#[lxr(regex = \"...\")]` on it",
            ));
        }
    }

    for attribute in skips {
        rules.push((attribute, None));
    }
    spans.extend(skip_spans);

    if rules.is_empty() {
        return Err(syn::Error::new(
            input.ident.span(),
            "the lexer holds no rule. Write `#[lxr(token = \"...\")]` on a variant of the enum",
        ));
    }

    let conditions = number(&initial, &rules)?;
    let specification = Specification {
        token: input.ident.clone(),
        rules: rules
            .iter()
            .map(|(attribute, gives)| rule(attribute, gives.clone(), &conditions))
            .collect::<syn::Result<Vec<Rule>>>()?,
        conditions: conditions.map(|conditions| conditions.0),
    };

    Ok(Read {
        specification,
        spans,
        name: input.ident.span(),
    })
}

/// Returns an error if `attribute` names a start condition and no pattern.
///
/// `in` and `go` belong to a rule. An attribute that names no pattern carries no rule, thus the
/// start condition attaches to nothing and the whole attribute does nothing.
///
/// # Errors
///
/// This function returns an error if the attribute names `in` or `go` and no pattern, or if it
/// names more than one pattern.
fn attached(attribute: &Attribute) -> syn::Result<()> {
    if attribute.pattern()?.is_some() {
        return Ok(());
    }

    let named = attribute
        .under
        .iter()
        .flatten()
        .chain(attribute.go.iter())
        .next();
    match named {
        Some(path) => Err(syn::Error::new(
            path.span(),
            "the attribute names a start condition and no pattern. Write `token`, `regex`, or \
             `skip` in the same attribute",
        )),
        None => Ok(()),
    }
}

/// Returns the type of the field of `variant`, or `None` if the variant holds no field.
///
/// # Errors
///
/// This function returns an error if `variant` holds more than one field, or if it holds a named
/// field.
fn field(variant: &Variant) -> syn::Result<Option<TokenStream>> {
    match &variant.fields {
        Fields::Unit => Ok(None),
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            let field = fields
                .unnamed
                .first()
                .expect("the list holds exactly one field");
            Ok(Some(field.ty.to_token_stream()))
        }
        Fields::Unnamed(fields) => Err(syn::Error::new(
            fields.span(),
            "a token of lxr holds one field or none. The field takes the text of the match, thus \
             a second field has no value to take",
        )),
        Fields::Named(fields) => Err(syn::Error::new(
            fields.span(),
            "a token of lxr holds an unnamed field. Write `Name(String)` in place of \
             `Name { text: String }`",
        )),
    }
}

/// The start conditions of the lexer, and the text of each one for a comparison.
struct Numbered(Conditions, Vec<String>);

/// Returns the parsed options of each `#[lxr(...)]` attribute of `attrs`.
///
/// # Errors
///
/// This function returns an error if an attribute does not parse.
fn options(attrs: &[syn::Attribute]) -> syn::Result<Vec<Attribute>> {
    attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("lxr"))
        .map(|attribute| attribute.parse_args::<Attribute>())
        .collect()
}

/// Numbers the start conditions, the first one at index 0.
///
/// `initial` is the condition at which the scan begins. Each other condition takes the next index,
/// in the sequence in which the rules name it.
///
/// # Errors
///
/// This function returns an error if a rule names a start condition and the lexer names no first
/// condition, or if the first condition is not the variant of an enum.
fn number(
    initial: &Option<Path>,
    rules: &[(Attribute, Option<Gives>)],
) -> syn::Result<Option<Numbered>> {
    let Some(initial) = initial else {
        for (attribute, _) in rules {
            if let Some(path) = attribute.under.as_ref().and_then(|paths| paths.first()) {
                return Err(syn::Error::new(
                    path.span(),
                    "the lexer names no start condition. Write `#[lxr(condition = ...)]` on the \
                     enum, and name the condition at which the scan begins",
                ));
            }
            if let Some(path) = &attribute.go {
                return Err(syn::Error::new(
                    path.span(),
                    "the lexer names no start condition. Write `#[lxr(condition = ...)]` on the \
                     enum, and name the condition at which the scan begins",
                ));
            }
        }
        return Ok(None);
    };

    let kind = kind(initial)?;
    let mut names = vec![initial.to_token_stream()];
    let mut texts = vec![text(initial)];

    for (attribute, _) in rules {
        let named = attribute.under.iter().flatten().chain(attribute.go.iter());
        for path in named {
            if !texts.contains(&text(path)) {
                texts.push(text(path));
                names.push(path.to_token_stream());
            }
        }
    }

    Ok(Some(Numbered(Conditions { kind, names }, texts)))
}

/// Returns the type of the start conditions, which is `path` without its last segment.
///
/// # Errors
///
/// This function returns an error if `path` holds one segment, thus it names no variant.
fn kind(path: &Path) -> syn::Result<TokenStream> {
    if path.segments.len() < 2 {
        return Err(syn::Error::new(
            path.span(),
            "name the start condition at which the scan begins, for example `Context::Code`",
        ));
    }

    let mut segments = Punctuated::<syn::PathSegment, Token![::]>::new();
    for segment in path.segments.iter().take(path.segments.len() - 1) {
        segments.push(segment.clone());
    }

    Ok(Path {
        leading_colon: path.leading_colon,
        segments,
    }
    .to_token_stream())
}

/// Returns the text of `path`, for a comparison of two conditions.
fn text(path: &Path) -> String {
    path.to_token_stream().to_string()
}

/// Returns the rule of `attribute`, which gives the variant of `gives`.
///
/// # Errors
///
/// This function returns an error if the attribute names a start condition that `conditions` does
/// not hold.
fn rule(
    attribute: &Attribute,
    gives: Option<Gives>,
    conditions: &Option<Numbered>,
) -> syn::Result<Rule> {
    let (literal, _) = attribute
        .pattern()?
        .expect("each rule of the lexer holds a pattern");
    let pattern = if attribute.is_literal() {
        Pattern::Literal(literal.value())
    } else {
        Pattern::Regex(literal.value())
    };

    let under = attribute
        .under
        .iter()
        .flatten()
        .map(|path| index(path, conditions))
        .collect::<syn::Result<Vec<usize>>>()?;
    let go = match &attribute.go {
        Some(path) => Some(u16::try_from(index(path, conditions)?).map_err(|_| {
            syn::Error::new(
                path.span(),
                "the lexer holds more start conditions than lxr numbers",
            )
        })?),
        None => None,
    };

    Ok(Rule {
        pattern,
        token: gives.as_ref().map(|gives| gives.variant.clone()),
        value: gives.and_then(|gives| gives.value),
        conditions: under,
        go,
    })
}

/// Returns the index of the start condition that `path` names.
///
/// # Errors
///
/// This function returns an error if the lexer holds no start condition.
fn index(path: &Path, conditions: &Option<Numbered>) -> syn::Result<usize> {
    let Some(Numbered(_, texts)) = conditions else {
        return Err(syn::Error::new(
            path.span(),
            "the lexer names no start condition",
        ));
    };

    texts
        .iter()
        .position(|held| held == &text(path))
        .ok_or_else(|| syn::Error::new(path.span(), "the lexer does not hold this start condition"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::ToTokens;

    /// Returns the lexer that `source` describes.
    fn lexer(source: &str) -> Read {
        let input: DeriveInput = syn::parse_str(source).expect("the source is an item");
        read(&input).expect("the source describes a lexer")
    }

    /// Returns the message of the fault of `source`, which describes no lexer.
    fn fault(source: &str) -> String {
        let input: DeriveInput = syn::parse_str(source).expect("the source is an item");
        match read(&input) {
            Ok(_) => panic!("the source holds no fault"),
            Err(error) => error.to_string(),
        }
    }

    /// Returns the pattern of each rule of `source`, in the sequence of the precedence.
    fn patterns(source: &str) -> Vec<Pattern> {
        lexer(source)
            .specification
            .rules
            .into_iter()
            .map(|rule| rule.pattern)
            .collect()
    }

    #[test]
    fn a_rule_of_a_variant_holds_its_pattern_and_the_variant_that_it_gives() {
        let specification = lexer(
            r#"
            enum Token {
                #[lxr(token = "let")]
                Let,
                #[lxr(regex = "[0-9]+")]
                Int(u64),
            }
            "#,
        )
        .specification;

        assert_eq!(specification.token.to_string(), "Token");
        assert_eq!(
            specification.rules[0].pattern,
            Pattern::Literal("let".to_owned())
        );
        assert_eq!(
            specification.rules[1].pattern,
            Pattern::Regex("[0-9]+".to_owned())
        );
        assert_eq!(
            specification.rules[1].value.to_token_stream().to_string(),
            "u64"
        );
    }

    #[test]
    fn a_rule_that_skips_comes_after_each_rule_of_a_variant() {
        let patterns = patterns(
            r#"
            #[lxr(skip = "[ ]+")]
            enum Token {
                #[lxr(regex = "[a-z]+")]
                Word,
            }
            "#,
        );

        assert_eq!(
            patterns,
            vec![
                Pattern::Regex("[a-z]+".to_owned()),
                Pattern::Regex("[ ]+".to_owned()),
            ]
        );
    }

    #[test]
    fn a_variant_that_holds_no_rule_is_rejected() {
        let message = fault(
            r#"
            enum Token {
                #[lxr(regex = "[a-z]+")]
                Word,
                Forgotten,
            }
            "#,
        );

        assert!(
            message.starts_with("the variant holds no rule"),
            "{message}"
        );
    }

    #[test]
    fn a_lexer_that_holds_no_rule_is_rejected() {
        assert!(fault("enum Token {}").starts_with("the lexer holds no rule"));
    }

    #[test]
    fn a_variant_that_names_the_first_start_condition_is_rejected() {
        let message = fault(
            r#"
            #[lxr(condition = Context::Code)]
            enum Token {
                #[lxr(condition = Context::Text)]
                #[lxr(regex = "[a-z]+")]
                Word,
            }
            "#,
        );

        assert!(message.contains("belongs to the lexer"), "{message}");
    }

    #[test]
    fn an_attribute_that_names_a_start_condition_and_no_pattern_is_rejected() {
        for source in [
            r#"
            #[lxr(condition = Context::Code)]
            enum Token {
                #[lxr(in = [Context::Text])]
                #[lxr(regex = "[a-z]+")]
                Word,
            }
            "#,
            r#"
            #[lxr(condition = Context::Code)]
            #[lxr(go = Context::Text)]
            enum Token {
                #[lxr(regex = "[a-z]+")]
                Word,
            }
            "#,
        ] {
            let message = fault(source);
            assert!(
                message.starts_with("the attribute names a start condition and no pattern"),
                "{message}"
            );
        }
    }

    #[test]
    fn a_rule_on_the_enum_that_gives_a_token_is_rejected() {
        let message = fault(
            r#"
            #[lxr(token = "let")]
            enum Token {
                #[lxr(regex = "[a-z]+")]
                Word,
            }
            "#,
        );

        assert!(
            message.starts_with("a rule on the enum gives no token"),
            "{message}"
        );
    }

    #[test]
    fn a_rule_that_skips_on_a_variant_is_rejected() {
        let message = fault(
            r#"
            enum Token {
                #[lxr(skip = "[ ]+")]
                Space,
            }
            "#,
        );

        assert!(
            message.starts_with("a rule that skips gives no token"),
            "{message}"
        );
    }

    #[test]
    fn an_item_that_is_not_an_enum_is_rejected() {
        let message = fault("struct Token { text: String }");

        assert_eq!(message, "lxr derives a lexer from an enum of tokens");
    }

    #[test]
    fn a_variant_of_a_named_field_or_of_two_fields_is_rejected() {
        let named = fault(
            r#"
            enum Token {
                #[lxr(regex = "[a-z]+")]
                Word { text: String },
            }
            "#,
        );
        let two = fault(
            r#"
            enum Token {
                #[lxr(regex = "[a-z]+")]
                Word(String, u32),
            }
            "#,
        );

        assert!(named.contains("unnamed field"), "{named}");
        assert!(two.contains("one field or none"), "{two}");
    }

    #[test]
    fn a_rule_that_names_a_start_condition_needs_a_condition_enum() {
        let message = fault(
            r#"
            enum Token {
                #[lxr(regex = "[a-z]+", in = [Context::Text])]
                Word,
            }
            "#,
        );

        assert!(
            message.contains("the lexer names no start condition"),
            "{message}"
        );
    }

    #[test]
    fn the_conditions_take_the_index_at_which_the_rules_name_them() {
        let specification = lexer(
            r#"
            #[lxr(condition = Context::Code)]
            enum Token {
                #[lxr(token = "\"", go = Context::Text)]
                Quote,
                #[lxr(regex = "[^\"]+", in = [Context::Text])]
                Text,
            }
            "#,
        )
        .specification;
        let conditions = specification
            .conditions
            .expect("the lexer names its start conditions");

        assert_eq!(conditions.kind.to_string(), "Context");
        assert_eq!(specification.rules[0].go, Some(1));
        assert_eq!(specification.rules[1].conditions, vec![1]);
    }

    #[test]
    fn one_start_condition_needs_no_list() {
        let specification = lexer(
            r#"
            #[lxr(condition = Context::Code)]
            enum Token {
                #[lxr(token = "\"", go = Context::Text)]
                Quote,
                #[lxr(regex = "[^\"]+", in = Context::Text)]
                Text,
            }
            "#,
        )
        .specification;

        assert_eq!(specification.rules[1].conditions, vec![1]);
    }

    #[test]
    fn the_span_of_each_rule_is_the_literal_of_its_pattern() {
        let read = lexer(
            r#"
            #[lxr(skip = "[ ]+")]
            enum Token {
                #[lxr(regex = "[a-z]+")]
                Word,
            }
            "#,
        );

        let held: Vec<String> = read.spans.iter().map(LitStr::value).collect();
        assert_eq!(held, ["[a-z]+", "[ ]+"]);
    }
}
