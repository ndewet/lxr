use crate::scan::Scan;
use crate::step::Step;

/// A lexer that reads an input into the tokens of `Self`.
///
/// The derive macro implements this trait. It reads the rules of an enum of tokens, it builds the
/// rule graph, and it emits that graph as code.
///
/// Derive it, and do not implement it by hand. [`step`](Self::step) carries the graph that the
/// macro built, and a step that lxr did not build can make a scan give the wrong token or panic.
///
/// A bound of `T: Lexer` reads any lexer, thus one function serves each enum of tokens.
/// [`syntax`](crate::syntax) holds the reference of the rules.
///
/// # Examples
///
#[cfg_attr(feature = "derive", doc = "```")]
#[cfg_attr(not(feature = "derive"), doc = "```ignore")]
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

    /// Writes the longest match of `input` at `at` into `step`.
    ///
    /// The derive macro emits one function for each node of the rule graph, thus a step of the
    /// scan is a comparison on the byte and not a read of a table.
    ///
    /// The step reads the start condition from [`Step::condition`], and it writes the condition of
    /// the next step there. It writes the token, the length of the match, and the bytes that it
    /// read.
    ///
    /// `at` is below the length of `input`, and it is at the start of a character.
    ///
    /// # Panics
    ///
    /// This function panics if [`Step::condition`] is not a start condition of the lexer.
    fn step(input: &str, at: usize, step: &mut Step<Self>);

    /// Returns the start condition at `index`.
    ///
    /// # Panics
    ///
    /// This function panics if `index` is not a start condition of the lexer.
    fn condition(index: u16) -> Self::Condition;

    /// Starts a scan of `input` under the first start condition.
    ///
    /// The scan gives one token at a time. It gives a [`ScanError`](crate::ScanError) for each
    /// fault of the input, then it reads the input after the part at fault. A character that no
    /// rule matches is one fault, and a match whose text does not fit the field of its token is
    /// the other one. Thus the span of an error covers one character or the whole match.
    fn scan(input: &str) -> Scan<'_, Self> {
        Scan::new(input)
    }
}
