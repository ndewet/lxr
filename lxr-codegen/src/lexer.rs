//! Compiler-internal representation of one lexer declaration.
//!
//! This model separates lexer semantics from the automata built from them.
//! Automata retain [`RuleId`] values at accepting states; the associated
//! pattern, action, and start-condition membership remain here.

use crate::regex::Expression;
use proc_macro2::TokenStream;

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
    pub(crate) fn pattern(&self) -> &Expression {
        &self.pattern
    }

    /// Returns the generated expression for this rule's result.
    pub(crate) fn action(&self) -> &RuleAction {
        &self.action
    }

    /// Returns the start conditions in which this rule is enabled.
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
    pub(crate) fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// Returns the start condition identified by `id`.
    ///
    /// # Panics
    ///
    /// Panics if `id` is not a start condition in this lexer.
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
    pub(crate) fn start_conditions(&self) -> &[StartCondition] {
        &self.start_conditions
    }
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
