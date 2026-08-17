use crate::scan::Scan;
use crate::tables::Tables;

/// A lexer that reads an input into the tokens of `Self`.
///
/// The derive macro implements this trait. It reads the rules of an enum of tokens, it builds the
/// automaton, and it emits the tables and the map from a number of a table onto a name.
///
/// Derive it, and do not implement it by hand. [`TABLES`](Self::TABLES),
/// [`token`](Self::token), and [`condition`](Self::condition) carry the automaton that the macro
/// built, and a table that lxr did not build can make a scan panic. The conditions of [`Tables`]
/// state what a table must obey.
///
/// A bound of `T: Lexer` reads any lexer, thus one function serves each enum of tokens.
/// [`syntax`](crate::syntax) holds the reference of the rules.
///
/// # Examples
///
/// ```
/// use lxr::Lexer;
///
/// #[derive(Debug, PartialEq, Lexer)]
/// #[lxr(skip = " +")]
/// enum Token {
///     #[lxr(token = "let")]
///     Let,
///     #[lxr(regex = "[a-z]+")]
///     Word,
/// }
///
/// /// Returns the tokens of `input`, whichever lexer reads it.
/// fn tokens<T: Lexer>(input: &str) -> Vec<T> {
///     T::scan(input)
///         .map(|found| found.expect("each character belongs to a token"))
///         .collect()
/// }
///
/// assert_eq!(tokens::<Token>("let it"), vec![Token::Let, Token::Word]);
/// ```
pub trait Lexer: Sized {
    /// The start conditions of the lexer.
    ///
    /// A lexer that reads under one condition gives `()`. A lexer that reads a string or a comment
    /// gives its own enum.
    type Condition: Copy;

    /// The automaton of the lexer.
    const TABLES: Tables<'static>;

    /// Returns the token of the rule at `rule`, which matched `text`.
    ///
    /// A variant that holds a field takes its value from `text`, through
    /// [`FromStr`](std::str::FromStr). The result is `None` if `text` does not fit that field. A
    /// rule of `[0-9]+` matches a number of any length, thus a field of `u32` gives `None` for a
    /// number above 4294967295.
    ///
    /// A variant that holds no field ignores `text`, thus it always gives a token.
    ///
    /// # Panics
    ///
    /// This function panics if `rule` is not a rule of the lexer, or if the rule skips its match
    /// and gives no token.
    fn token(rule: u16, text: &str) -> Option<Self>;

    /// Returns the start condition at `index`.
    ///
    /// # Panics
    ///
    /// This function panics if `index` is not a start condition of the lexer.
    fn condition(index: u16) -> Self::Condition;

    /// Starts a scan of `input` under the first start condition.
    ///
    /// The scan gives one token at a time. It gives a [`ScanError`](crate::ScanError) for each
    /// character that no rule matches, then it reads the input after that character.
    fn scan(input: &str) -> Scan<'_, Self> {
        Scan::new(input)
    }
}
