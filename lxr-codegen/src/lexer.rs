//! Compiler-internal representation of one lexer declaration.
//!
//! This model separates lexer semantics from the automata built from them.
//! Automata retain [`RuleId`] values at accepting states; the associated
//! pattern, action, and start-condition membership remain here.

use crate::automata::{BuildError, dfa, encoding::Utf8, nfa};
use crate::emitter;
use crate::regex::{Expression, ParseError};
use proc_macro2::TokenStream;
use std::fmt::{Display, Formatter};

/// One rule supplied to the code generator.
pub struct RuleSpec {
    pattern: String,
    action: TokenStream,
}
impl RuleSpec {
    /// Creates a rule from its regex pattern and generated Rust action.
    pub fn new(pattern: impl Into<String>, action: TokenStream) -> Self {
        Self {
            pattern: pattern.into(),
            action,
        }
    }
}
/// Reports a lexer compilation failure.
#[derive(Debug)]
pub struct CompileError {
    message: String,
}
impl Display for CompileError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for CompileError {}
impl From<ParseError> for CompileError {
    fn from(error: ParseError) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}
impl From<BuildError> for CompileError {
    fn from(error: BuildError) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}

/// Identifies a rule in one [`Lexer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RuleId(usize);

impl RuleId {
    /// Creates an identifier for the rule at `index`.
    pub(crate) const fn new(index: usize) -> Self {
        Self(index)
    }

    /// Returns the rule's declaration index.
    pub(crate) const fn index(self) -> usize {
        self.0
    }
}

/// Identifies a start condition in one [`Lexer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct StartConditionId(usize);

impl StartConditionId {
    /// Creates an identifier for the start condition at `index`.
    pub(crate) const fn new(index: usize) -> Self {
        Self(index)
    }

    /// Returns the start condition's declaration index.
    pub(crate) const fn index(self) -> usize {
        self.0
    }
}

/// A named lexer mode that selects one DFA start state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartCondition {
    name: String,
}

impl StartCondition {
    /// Creates a start condition with its source-level name.
    pub(crate) fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Returns the source-level name.
    #[cfg(test)]
    pub(crate) fn name(&self) -> &str {
        &self.name
    }
}

/// One pattern and the Rust expression it produces when accepted.
pub(crate) struct Rule {
    pattern: Expression,
    action: RuleAction,
    start_conditions: Vec<StartConditionId>,
}

impl Rule {
    /// Parses a regex rule enabled in `start_conditions`.
    ///
    /// # Errors
    ///
    /// Returns an error when `pattern` is not valid lxr regex syntax.
    fn parse(
        pattern: &str,
        action: RuleAction,
        start_conditions: Vec<StartConditionId>,
    ) -> Result<Self, ParseError> {
        Ok(Self::new(pattern.parse()?, action, start_conditions))
    }

    /// Creates a rule enabled in `start_conditions`.
    pub(crate) fn new(
        pattern: Expression,
        action: RuleAction,
        start_conditions: Vec<StartConditionId>,
    ) -> Self {
        Self {
            pattern,
            action,
            start_conditions,
        }
    }

    /// Returns the parsed regular expression.
    #[cfg(test)]
    pub(crate) fn pattern(&self) -> &Expression {
        &self.pattern
    }

    /// Returns the generated expression for this rule's result.
    pub(crate) fn action(&self) -> &RuleAction {
        &self.action
    }

    /// Returns the start conditions in which this rule is enabled.
    #[cfg(test)]
    pub(crate) fn start_conditions(&self) -> &[StartConditionId] {
        &self.start_conditions
    }
}

/// The generated expression returned when a rule wins a match.
pub(crate) struct RuleAction(TokenStream);

impl RuleAction {
    /// Creates an action from its generated Rust expression.
    pub(crate) fn new(tokens: TokenStream) -> Self {
        Self(tokens)
    }

    /// Returns the generated Rust expression.
    pub(crate) fn tokens(&self) -> &TokenStream {
        &self.0
    }
}

/// All semantic input needed to construct and emit one lexer.
pub(crate) struct Lexer {
    rules: Vec<Rule>,
    start_conditions: Vec<StartCondition>,
}

impl Lexer {
    /// Creates a lexer from declaration-ordered rules and start conditions.
    pub(crate) fn new(rules: Vec<Rule>, start_conditions: Vec<StartCondition>) -> Self {
        Self {
            rules,
            start_conditions,
        }
    }

    /// Returns the rule identified by `id`.
    ///
    /// # Panics
    ///
    /// Panics if `id` is not a rule in this lexer.
    pub(crate) fn rule(&self, id: RuleId) -> &Rule {
        self.rules.get(id.index()).unwrap_or_else(|| {
            panic!(
                "rule {} is outside a lexer with {} rules",
                id.index(),
                self.rules.len()
            )
        })
    }

    /// Returns the lexer rules in declaration order.
    #[cfg(test)]
    pub(crate) fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// Returns the start condition identified by `id`.
    ///
    /// # Panics
    ///
    /// Panics if `id` is not a start condition in this lexer.
    #[cfg(test)]
    pub(crate) fn start_condition(&self, id: StartConditionId) -> &StartCondition {
        self.start_conditions.get(id.index()).unwrap_or_else(|| {
            panic!(
                "start condition {} is outside a lexer with {} start conditions",
                id.index(),
                self.start_conditions.len()
            )
        })
    }

    /// Returns the start conditions in declaration order.
    #[cfg(test)]
    pub(crate) fn start_conditions(&self) -> &[StartCondition] {
        &self.start_conditions
    }

    /// Builds, minimizes, and emits this lexer as one private matcher method.
    ///
    /// # Errors
    ///
    /// Returns an error if the automaton exceeds its representable capacity.
    ///
    /// # Panics
    ///
    /// Panics if a rule enables a start condition that this lexer does not
    /// define.
    fn emit(&self) -> Result<TokenStream, BuildError> {
        let mut builder = nfa::Builder::new();
        let starts: Vec<_> = self
            .start_conditions
            .iter()
            .map(|_| builder.add_state())
            .collect();

        for (index, rule) in self.rules.iter().enumerate() {
            let fragment = nfa::thompson::fragment(&rule.pattern, &Utf8, &mut builder);
            for start_condition in &rule.start_conditions {
                let start = starts.get(start_condition.index()).unwrap_or_else(|| {
                    panic!(
                        "rule {index} enables start condition {} outside a lexer with {} start conditions",
                        start_condition.index(),
                        starts.len()
                    )
                });
                builder.add_epsilon_transition(*start, fragment.entry());
            }
            builder.set_accept(fragment.exit(), RuleId::new(index));
        }

        let nfa = builder.build(&starts)?;
        let dfa = dfa::subset::construct(&nfa, |states| {
            states
                .iter()
                .filter_map(|state| nfa.accept(*state).copied())
                .min()
                .expect("an accepting subset contains an accepting NFA state")
        })?;
        let dfa = dfa.minimize()?;

        Ok(emitter::emit(&dfa, self))
    }
}

/// Builds, minimizes, and emits a lexer from declaration-ordered rules.
///
/// # Errors
///
/// Returns an error for invalid regex syntax or an automaton capacity limit.
pub fn compile(specifications: Vec<RuleSpec>) -> Result<TokenStream, CompileError> {
    let rules = specifications
        .into_iter()
        .map(|specification| {
            Rule::parse(
                &specification.pattern,
                RuleAction::new(specification.action),
                vec![StartConditionId::new(0)],
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Lexer::new(rules, vec![StartCondition::new("INITIAL")])
        .emit()
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use std::str::FromStr;

    fn example() -> Lexer {
        Lexer::new(
            vec![Rule::new(
                Expression::from_str("[a-z]+").expect("the test pattern is valid"),
                RuleAction::new(quote!(Token::Identifier)),
                vec![StartConditionId::new(0)],
            )],
            vec![StartCondition::new("INITIAL")],
        )
    }

    #[test]
    fn lexer_keeps_rule_data_by_its_identifier() {
        let lexer = example();
        let rule = lexer.rule(RuleId::new(0));

        assert_eq!(
            rule.pattern(),
            &Expression::from_str("[a-z]+").expect("the test pattern is valid")
        );
        assert_eq!(rule.action().tokens().to_string(), "Token :: Identifier");
        assert_eq!(rule.start_conditions(), &[StartConditionId::new(0)]);
        assert_eq!(lexer.rules().len(), 1);
    }

    #[test]
    fn lexer_keeps_named_start_conditions_by_identifier() {
        let lexer = example();

        assert_eq!(
            lexer.start_condition(StartConditionId::new(0)).name(),
            "INITIAL"
        );
        assert_eq!(lexer.start_conditions().len(), 1);
    }

    #[test]
    #[should_panic(expected = "rule 1 is outside a lexer with 1 rules")]
    fn reading_a_rule_outside_the_lexer_panics() {
        example().rule(RuleId::new(1));
    }

    #[test]
    #[should_panic(expected = "start condition 1 is outside a lexer with 1 start conditions")]
    fn reading_a_start_condition_outside_the_lexer_panics() {
        example().start_condition(StartConditionId::new(1));
    }
}
