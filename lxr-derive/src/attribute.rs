use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, LitStr, Path, Token, bracketed};

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
}

impl Parse for Attribute {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut attribute = Self::default();

        while !input.is_empty() {
            if input.peek(Token![in]) {
                input.parse::<Token![in]>()?;
                input.parse::<Token![=]>()?;
                let list;
                bracketed!(list in input);
                let paths = Punctuated::<Path, Token![,]>::parse_terminated(&list)?;
                attribute.under = Some(paths.into_iter().collect());
            } else {
                let key: Ident = input.parse()?;
                input.parse::<Token![=]>()?;
                match key.to_string().as_str() {
                    "condition" => attribute.condition = Some(input.parse()?),
                    "skip" => attribute.skip = Some(input.parse()?),
                    "token" => attribute.token = Some(input.parse()?),
                    "regex" => attribute.regex = Some(input.parse()?),
                    "go" => attribute.go = Some(input.parse()?),
                    other => {
                        return Err(syn::Error::new(
                            key.span(),
                            format!(
                                "`{other}` is not an option of lxr. Write `token`, `regex`, \
                                 `skip`, `in`, `go`, or `condition`"
                            ),
                        ));
                    }
                }
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        Ok(attribute)
    }
}
