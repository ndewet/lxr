use crate::scan::Scan;
use crate::tables::Tables;

/// A lexer that reads an input into the tokens of `Self`.
///
/// The derive macro implements this trait. It reads the rules of an enum of tokens, it builds the
/// automaton, and it emits the tables and the two maps between a number and a name.
///
/// Do not implement this trait by hand. A table that lxr did not build can make a scan panic. The
/// conditions of [`Tables`] state what a table must obey.
///
/// # Examples
///
/// ```
/// use lxr::{Action, Lexer, Tables};
///
/// # #[derive(Debug, PartialEq)]
/// # enum Token { A }
/// # static CLASSES: [u16; 256] = { let mut c = [0; 256]; c[b'a' as usize] = 1; c };
/// # static NEXT: [u16; 6] = [0, 0, 0, 2, 0, 0];
/// # static ACCEPT: [u16; 3] = [0, 0, 1];
/// # static START: [u16; 1] = [1];
/// # static ACTIONS: [Action; 1] = [Action::token()];
/// impl Lexer for Token {
///     type Condition = ();
///
///     const TABLES: Tables<'static> = Tables {
///         classes: &CLASSES,
///         next: &NEXT,
///         width: 2,
///         accept: &ACCEPT,
///         start: &START,
///         actions: &ACTIONS,
///     };
///
///     fn token(_rule: u16, _text: &str) -> Option<Self> {
///         Some(Token::A)
///     }
///
///     fn condition(_index: u16) {}
///
///     fn index(_condition: Self::Condition) -> u16 {
///         0
///     }
/// }
///
/// let tokens: Vec<_> = Token::scan("aa").collect();
/// assert_eq!(tokens, vec![Ok(Token::A), Ok(Token::A)]);
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

    /// Returns the index of `condition`.
    fn index(condition: Self::Condition) -> u16;

    /// Starts a scan of `input` under the first start condition.
    ///
    /// The scan gives one token at a time. It gives a [`ScanError`](crate::ScanError) for each
    /// character that no rule matches, then it reads the input after that character.
    fn scan(input: &str) -> Scan<'_, Self> {
        Scan::new(input)
    }
}
