use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, LitStr, Path, Token, bracketed, token};

/// The options of one `#[lxr(...)]` attribute.
///
/// One attribute carries the options of one rule, or the configuration of the lexer. A field is
/// `None` if the attribute does not name that option.
///
/// The parser reads `in` with [`Token`], because `in` is a keyword of Rust and not an identifier.
#[derive(Default)]
pub struct Attribute {
    /// The start condition at which the scan begins, for example `Context::Code`.
    pub condition: Option<Path>,
    /// The pattern of a rule that gives no token.
    pub skip: Option<LitStr>,
    /// The literal that a rule matches.
    pub token: Option<LitStr>,
    /// The regular expression that a rule matches.
    pub regex: Option<LitStr>,
    /// The start conditions under which a rule is applicable.
    pub under: Option<Vec<Path>>,
    /// The start condition that the scan changes to after a match.
    pub go: Option<Path>,
}

impl Attribute {
    /// Returns the pattern of this attribute, and `true` if the rule gives no token.
    ///
    /// The result is `None` if the attribute names no pattern, thus it carries no rule.
    ///
    /// # Errors
    ///
    /// This function returns an error if the attribute names more than one pattern.
    pub fn pattern(&self) -> syn::Result<Option<(&LitStr, bool)>> {
        let named: Vec<(&LitStr, bool)> = [
            self.token.as_ref().map(|literal| (literal, false)),
            self.regex.as_ref().map(|literal| (literal, false)),
            self.skip.as_ref().map(|literal| (literal, true)),
        ]
        .into_iter()
        .flatten()
        .collect();

        match named.len() {
            0 => Ok(None),
            1 => Ok(Some(named[0])),
            _ => Err(syn::Error::new(
                named[1].0.span(),
                "a rule holds one pattern. Write `token`, `regex`, or `skip`, and not two of them",
            )),
        }
    }

    /// Returns `true` if the pattern of this attribute is a literal and not a regular expression.
    pub fn is_literal(&self) -> bool {
        self.token.is_some()
    }

    /// Reads the option that `key` names, and reads its value from `input`.
    ///
    /// # Errors
    ///
    /// This function returns an error if lxr holds no option of that name, if this attribute
    /// already names the option, or if the value does not parse.
    fn option(&mut self, key: &Ident, input: ParseStream<'_>) -> syn::Result<()> {
        let at = key.span();
        match key.to_string().as_str() {
            "condition" => once(&mut self.condition, input.parse()?, "condition", at),
            "skip" => once(&mut self.skip, input.parse()?, "skip", at),
            "token" => once(&mut self.token, input.parse()?, "token", at),
            "regex" => once(&mut self.regex, input.parse()?, "regex", at),
            "go" => once(&mut self.go, input.parse()?, "go", at),
            other => Err(syn::Error::new(
                at,
                format!(
                    "`{other}` is not an option of lxr. Write `token`, `regex`, `skip`, `in`, \
                     `go`, or `condition`"
                ),
            )),
        }
    }
}

impl Parse for Attribute {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut attribute = Self::default();

        while !input.is_empty() {
            if input.peek(Token![in]) {
                let key = input.parse::<Token![in]>()?;
                input.parse::<Token![=]>()?;
                once(&mut attribute.under, conditions(input)?, "in", key.span)?;
            } else {
                let key: Ident = input.parse()?;
                input.parse::<Token![=]>()?;
                attribute.option(&key, input)?;
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        Ok(attribute)
    }
}

/// Puts `value` in `slot`, which the option `name` at `at` fills.
///
/// A repeated option keeps the last value and drops each earlier one, thus a rule loses its
/// pattern without a word. This function reports the repeat instead.
///
/// # Errors
///
/// This function returns an error if `slot` already holds a value.
fn once<T>(slot: &mut Option<T>, value: T, name: &str, at: Span) -> syn::Result<()> {
    if slot.is_some() {
        return Err(syn::Error::new(
            at,
            format!("the attribute already names `{name}`. Write it one time"),
        ));
    }

    *slot = Some(value);
    Ok(())
}

/// Returns the start conditions that `input` names, as one path or as a list of paths.
///
/// # Errors
///
/// This function returns an error if the list names no start condition, or if a path does not
/// parse.
fn conditions(input: ParseStream<'_>) -> syn::Result<Vec<Path>> {
    if !input.peek(token::Bracket) {
        return Ok(vec![input.parse()?]);
    }

    let list;
    let bracket = bracketed!(list in input);
    let paths = Punctuated::<Path, Token![,]>::parse_terminated(&list)?;
    if paths.is_empty() {
        return Err(syn::Error::new(
            bracket.span.join(),
            "`in` names no start condition. Name one, or remove `in`",
        ));
    }

    Ok(paths.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::ToTokens;

    /// Returns the message of the fault of `source`, which is not a valid attribute.
    fn fault(source: &str) -> String {
        match syn::parse_str::<Attribute>(source) {
            Ok(_) => panic!("the attribute holds no fault"),
            Err(error) => error.to_string(),
        }
    }

    /// Returns the options that `source` names.
    fn options(source: &str) -> Attribute {
        syn::parse_str(source).expect("the attribute is valid")
    }

    /// Returns the text of each start condition that `source` names.
    fn under(source: &str) -> Vec<String> {
        options(source)
            .under
            .expect("the attribute names a start condition")
            .iter()
            .map(|path| path.to_token_stream().to_string())
            .collect()
    }

    #[test]
    fn an_option_that_the_attribute_names_two_times_is_rejected() {
        assert_eq!(
            fault(r#"token = "a", token = "b""#),
            "the attribute already names `token`. Write it one time"
        );
        for source in [
            r#"regex = "a", regex = "b""#,
            r#"skip = "a", skip = "b""#,
            "in = [A::B], in = [A::C]",
            "go = A::B, go = A::C",
            "condition = A::B, condition = A::C",
        ] {
            assert!(fault(source).contains("already names"), "{source}");
        }
    }

    #[test]
    fn one_start_condition_needs_no_list() {
        assert_eq!(
            under("regex = \"a\", in = Context::Text"),
            ["Context :: Text"]
        );
    }

    #[test]
    fn a_list_holds_each_start_condition_that_it_names() {
        assert_eq!(
            under("regex = \"a\", in = [Context::Code, Context::Text]"),
            ["Context :: Code", "Context :: Text"]
        );
    }

    #[test]
    fn a_list_of_no_start_condition_is_rejected() {
        assert_eq!(
            fault(r#"regex = "a", in = []"#),
            "`in` names no start condition. Name one, or remove `in`"
        );
    }

    #[test]
    fn an_option_of_no_lexer_names_each_option_that_lxr_holds() {
        let message = fault(r#"tokens = "a""#);

        assert!(message.starts_with("`tokens` is not an option of lxr"));
        for name in ["token", "regex", "skip", "in", "go", "condition"] {
            assert!(message.contains(&format!("`{name}`")), "{name}");
        }
    }

    #[test]
    fn an_attribute_of_two_patterns_holds_no_pattern() {
        let attribute = options(r#"token = "a", regex = "b""#);

        assert!(attribute.pattern().is_err());
    }

    #[test]
    fn a_literal_is_a_pattern_that_needs_no_escape() {
        let attribute = options(r#"token = "a+b""#);

        assert!(attribute.is_literal());
        assert!(!options(r#"regex = "a""#).is_literal());
    }
}
