//! Compiler-internal representation of one lexer declaration.
//!
//! This model separates lexer semantics from the automata built from them.
//! Automata retain [`RuleId`] values at accepting states; the associated
//! pattern, action, and start-condition membership remain here.

use crate::automata::{BuildError, dfa, encoding::Utf8, nfa};
use crate::emitter;
use crate::regex::{Expression, ParseError};
use proc_macro2::TokenStream;
use quote::quote;
use std::fmt::{Display, Formatter};

/// One rule supplied to the code generator.
///
/// # Examples
///
/// ```
/// use lxr_codegen::RuleSpec;
///
/// let rule = RuleSpec::skip(r"\s+");
/// let _ = rule;
/// ```
pub struct RuleSpec {
    pattern: String,
    action: RuleAction,
    modes: Vec<String>,
    transition: Transition,
}

/// A state-stack operation performed after a rule is accepted.
///
/// # Examples
///
/// ```
/// use lxr_codegen::Transition;
///
/// let transition = Transition::Push("String".into());
/// assert!(matches!(transition, Transition::Push(name) if name == "String"));
/// ```
#[derive(Clone)]
pub enum Transition {
    /// Leave the stack unchanged.
    Stay,
    /// Replace the current mode.
    Begin(String),
    /// Push a mode onto the stack.
    Push(String),
    /// Pop the current mode.
    Pop,
}

impl RuleSpec {
    /// Creates a rule from its regex pattern and generated Rust action.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr_codegen::RuleSpec;
    ///
    /// let rule = RuleSpec::emit("[a-z]+", quote::quote!(Ok(Some(Self::Word))));
    /// let _ = rule;
    /// ```
    pub fn emit(pattern: impl Into<String>, action: TokenStream) -> Self {
        Self {
            pattern: pattern.into(),
            action: RuleAction::Emit(action),
            modes: vec!["INITIAL".into()],
            transition: Transition::Stay,
        }
    }
    /// Creates a rule that consumes input without producing a token.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr_codegen::RuleSpec;
    ///
    /// let rule = RuleSpec::skip(r"\s+");
    /// let _ = rule;
    /// ```
    pub fn skip(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            action: RuleAction::Skip,
            modes: vec!["INITIAL".into()],
            transition: Transition::Stay,
        }
    }

    /// Restricts this rule to named start conditions.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr_codegen::RuleSpec;
    ///
    /// let rule = RuleSpec::skip("end").in_modes(vec!["String".into()]);
    /// let _ = rule;
    /// ```
    pub fn in_modes(mut self, modes: Vec<String>) -> Self {
        self.modes = modes;
        self
    }

    /// Sets the start-condition-stack transition after this rule matches.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr_codegen::{RuleSpec, Transition};
    ///
    /// let rule = RuleSpec::skip("end").transition(Transition::Pop);
    /// let _ = rule;
    /// ```
    pub fn transition(mut self, transition: Transition) -> Self {
        self.transition = transition;
        self
    }
}
/// Reports a lexer compilation failure.
///
/// # Examples
///
/// ```
/// use lxr_codegen::{RuleSpec, compile};
///
/// let error = compile(vec![RuleSpec::skip("^")]).expect_err("anchors are invalid");
/// assert!(error.to_string().contains("anchor"));
/// ```
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
    pub(crate) fn name(&self) -> &str {
        &self.name
    }
}

/// One pattern and the Rust expression it produces when accepted.
pub(crate) struct Rule {
    pattern: Expression,
    action: RuleAction,
    start_conditions: Vec<StartConditionId>,
    transition: ResolvedTransition,
}

#[derive(Clone, Copy)]
pub(crate) enum ResolvedTransition {
    Stay,
    Begin(StartConditionId),
    Push(StartConditionId),
    Pop,
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
        transition: ResolvedTransition,
    ) -> Result<Self, ParseError> {
        Ok(Self::new(
            pattern.parse()?,
            action,
            start_conditions,
            transition,
        ))
    }

    /// Creates a rule enabled in `start_conditions`.
    pub(crate) fn new(
        pattern: Expression,
        action: RuleAction,
        start_conditions: Vec<StartConditionId>,
        transition: ResolvedTransition,
    ) -> Self {
        Self {
            pattern,
            action,
            start_conditions,
            transition,
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

    pub(crate) fn transition(&self) -> ResolvedTransition {
        self.transition
    }

    /// Returns the start conditions in which this rule is enabled.
    #[cfg(test)]
    pub(crate) fn start_conditions(&self) -> &[StartConditionId] {
        &self.start_conditions
    }
}

/// The generated expression returned when a rule wins a match.
pub(crate) enum RuleAction {
    Emit(TokenStream),
    Skip,
}

impl RuleAction {
    /// Renders the generated action for the selected rule.
    pub(crate) fn rendered(&self) -> TokenStream {
        match self {
            Self::Emit(tokens) => quote!(#tokens),
            Self::Skip => quote!(Ok(None)),
        }
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
    #[cfg(test)]
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
///
/// # Examples
///
/// ```
/// use lxr_codegen::{RuleSpec, compile};
///
/// let generated = compile(vec![
///     RuleSpec::skip(r"\s+"),
///     RuleSpec::emit("[a-z]+", quote::quote!(Ok(Some(Self::Word)))),
/// ])?;
/// assert!(generated.to_string().contains("__lxr_transition"));
/// # Ok::<(), lxr_codegen::CompileError>(())
/// ```
pub fn compile(specifications: Vec<RuleSpec>) -> Result<TokenStream, CompileError> {
    compile_with_modes(Vec::new(), specifications)
}

/// Builds a lexer with named start conditions in addition to `INITIAL`.
/// Builds a lexer with named start conditions in addition to `INITIAL`.
///
/// # Errors
///
/// Returns an error for invalid rules, duplicate or unknown modes, invalid
/// stack transitions, regex syntax, or an automaton capacity limit.
///
/// # Examples
///
/// ```
/// use lxr_codegen::{RuleSpec, Transition, compile_with_modes};
///
/// let generated = compile_with_modes(
///     vec!["String".into()],
///     vec![
///         RuleSpec::skip(r#"\""#).transition(Transition::Push("String".into())),
///         RuleSpec::skip(r#"\""#)
///             .in_modes(vec!["String".into()])
///             .transition(Transition::Pop),
///     ],
/// )?;
/// assert!(generated.to_string().contains("Transition :: Push"));
/// # Ok::<(), lxr_codegen::CompileError>(())
/// ```
pub fn compile_with_modes(
    modes: Vec<String>,
    specifications: Vec<RuleSpec>,
) -> Result<TokenStream, CompileError> {
    let mut conditions = vec![StartCondition::new("INITIAL")];
    for mode in modes {
        if mode == "INITIAL" || mode == "Initial" || conditions.iter().any(|item| item.name == mode)
        {
            return Err(CompileError {
                message: format!("duplicate or reserved lexer mode `{mode}`"),
            });
        }
        conditions.push(StartCondition::new(mode));
    }
    let resolve = |name: &str| -> Result<StartConditionId, CompileError> {
        let name = if name == "Initial" { "INITIAL" } else { name };
        conditions
            .iter()
            .position(|condition| condition.name == name)
            .map(StartConditionId::new)
            .ok_or_else(|| CompileError {
                message: format!("unknown lexer mode `{name}`"),
            })
    };
    let rules = specifications
        .into_iter()
        .map(|specification| {
            let enabled = specification
                .modes
                .iter()
                .map(|name| resolve(name))
                .collect::<Result<Vec<_>, _>>()?;
            if enabled.is_empty() {
                return Err(CompileError {
                    message: "a lexer rule must be enabled in at least one mode".into(),
                });
            }
            let transition = match specification.transition {
                Transition::Stay => ResolvedTransition::Stay,
                Transition::Begin(name) => ResolvedTransition::Begin(resolve(&name)?),
                Transition::Push(name) => ResolvedTransition::Push(resolve(&name)?),
                Transition::Pop => {
                    if enabled.iter().any(|id| id.index() == 0) {
                        return Err(CompileError {
                            message: "a `pop` rule cannot be enabled in INITIAL".into(),
                        });
                    }
                    ResolvedTransition::Pop
                }
            };
            Rule::parse(
                &specification.pattern,
                specification.action,
                enabled,
                transition,
            )
            .map_err(Into::into)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if rules.iter().any(|rule| rule.pattern.is_nullable()) {
        return Err(CompileError {
            message: "a lexer rule must consume at least one byte".into(),
        });
    }
    Lexer::new(rules, conditions).emit().map_err(Into::into)
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
                RuleAction::Emit(quote!(Token::Identifier)),
                vec![StartConditionId::new(0)],
                ResolvedTransition::Stay,
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
        assert_eq!(rule.action().rendered().to_string(), "Token :: Identifier");
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
