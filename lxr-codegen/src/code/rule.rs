use proc_macro2::{Ident, TokenStream};

/// What one rule of a lexer gives when it matches.
#[derive(Debug, Clone)]
pub struct Rule {
    /// The variant of the token enum that the rule gives, or `None` if the rule skips its match.
    pub token: Option<Ident>,
    /// The type of the field of that variant, or `None` if the variant holds no field.
    ///
    /// The emitted source reads the field from the text of the match with
    /// [`FromStr`](std::str::FromStr).
    pub value: Option<TokenStream>,
    /// The index of the start condition that the scan changes to, or `None` if it keeps the
    /// condition.
    pub go: Option<u16>,
}
