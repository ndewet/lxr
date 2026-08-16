//! Builds the source of a lexer from the rules that a lexer author wrote.
//!
//! [`generate`] joins each part of the crate. It parses each pattern, it builds the automaton, it
//! determinizes the automaton, it makes the tables, and it emits the source.
//!
//! The derive macro supplies a [`Specification`], and it holds the span of each rule. Thus this
//! module reports the index of the rule at fault, and the macro turns that index into a span.

#![allow(dead_code)]

use proc_macro2::{Ident, TokenStream};

use crate::automata::Overflow;
use crate::compiler::{BuildErrorKind, Bytes, Lexicon, compile};
use crate::emit::{self, Emission, emit};
use crate::regex::{CharSet, Node, ParseError};
use crate::table::{MAX_RULES, Tables};

/// The pattern of one rule, as the author wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    /// A regular expression. The parser reads it.
    Regex(String),
    /// A literal. Each character matches itself, thus a regex character needs no escape.
    Literal(String),
}

impl Pattern {
    /// Returns the tree of this pattern.
    ///
    /// # Errors
    ///
    /// This function returns a [`ParseError`] if a [`Regex`](Self::Regex) is not a valid regular
    /// expression. A [`Literal`](Self::Literal) always parses.
    fn node(&self) -> Result<Node, ParseError> {
        match self {
            Self::Regex(pattern) => pattern.parse(),
            Self::Literal(literal) => Ok(literal.chars().fold(Node::Epsilon, |node, character| {
                node.concat(Node::Class(CharSet::single(character)))
            })),
        }
    }
}

/// One rule of a lexer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// The pattern that the rule matches.
    pub pattern: Pattern,
    /// The variant that the rule gives, or `None` if the rule skips its match.
    pub token: Option<Ident>,
    /// The indexes of the start conditions of the rule. An empty list means the first condition.
    pub conditions: Vec<usize>,
    /// The index of the start condition that the scan changes to after the match.
    pub go: Option<u16>,
}

/// The start conditions of a lexer.
#[derive(Debug, Clone)]
pub struct Conditions {
    /// The type of the conditions, for example `Context`.
    pub kind: TokenStream,
    /// One expression for each condition, in the sequence of the indexes.
    pub names: Vec<TokenStream>,
}

/// The lexer that a lexer author wrote.
///
/// [`rules`](Self::rules) is in the sequence of precedence. The earliest rule wins a tie at the
/// same length.
#[derive(Debug, Clone)]
pub struct Specification {
    /// The name of the enum of the tokens.
    pub token: Ident,
    /// The start conditions, or `None` if the lexer reads under one condition.
    pub conditions: Option<Conditions>,
    /// The rules, in the sequence of precedence.
    pub rules: Vec<Rule>,
}

/// A failure to build the source of a lexer.
///
/// The index of the rule gives the derive macro the span for its `compile_error!`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct GenerateError {
    /// The index of the rule at fault. A fault of the whole lexer gives `None`.
    pub rule: Option<usize>,
    /// The kind of the failure.
    pub kind: GenerateErrorKind,
}

/// The kind of failure that a [`GenerateError`] reports.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GenerateErrorKind {
    /// The parser cannot read the pattern of a rule.
    Pattern(ParseError),
    /// A check on a rule or on the whole lexer failed.
    Rule(BuildErrorKind),
    /// The lexer holds more rules than a table numbers.
    TooManyRules {
        /// The number of the rules of the lexer.
        count: usize,
        /// The maximum number of the rules.
        maximum: usize,
    },
}

impl GenerateErrorKind {
    /// Joins this kind to the rule at `rule`, then gives the error.
    fn in_rule(self, rule: usize) -> GenerateError {
        GenerateError {
            rule: Some(rule),
            kind: self,
        }
    }

    /// Joins this kind to the whole lexer, then gives the error.
    fn in_lexer(self) -> GenerateError {
        GenerateError {
            rule: None,
            kind: self,
        }
    }

    /// Returns the correction that the lexer author must make.
    pub fn help(&self) -> Option<&'static str> {
        match self {
            Self::Pattern(error) => error.kind.help(),
            Self::Rule(kind) => Some(kind.help()),
            Self::TooManyRules { .. } => Some("Divide the lexer into two lexers."),
        }
    }
}

impl std::fmt::Display for GenerateErrorKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pattern(error) => write!(formatter, "{error}"),
            Self::Rule(kind) => write!(formatter, "{kind}"),
            Self::TooManyRules { count, maximum } => write!(
                formatter,
                "the lexer holds {count} rules, above the limit of {maximum}"
            ),
        }
    }
}

impl std::fmt::Display for GenerateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.kind)
    }
}

impl std::error::Error for GenerateError {}

/// Builds the source of the lexer that `specification` describes.
///
/// The function reports each fault that it can, and not the first one alone. Thus the macro marks
/// each rule at fault in one build.
///
/// # Errors
///
/// This function returns one [`GenerateError`] for each fault. A pattern that the parser cannot
/// read, a rule that the lexicon rejects, and a lexer above a limit each give one.
pub fn generate(specification: &Specification) -> Result<TokenStream, Vec<GenerateError>> {
    let nodes = parse(&specification.rules)?;
    let lexicon = build(specification, nodes)?;

    let (nfa, accepts) = compile(Bytes, lexicon).map_err(|error| vec![failed(error.kind)])?;
    let determinization = nfa.determinize().map_err(overflow)?;
    let accepts = accepts.determinized(&determinization.subsets);
    let tables = Tables::new(&determinization.dfa, &accepts).map_err(overflow)?;

    Ok(emit(&Emission {
        token: specification.token.clone(),
        condition: specification
            .conditions
            .as_ref()
            .map(|conditions| conditions.kind.clone()),
        conditions: specification
            .conditions
            .as_ref()
            .map_or_else(Vec::new, |conditions| conditions.names.clone()),
        rules: specification
            .rules
            .iter()
            .map(|rule| emit::Rule {
                token: rule.token.clone(),
                go: rule.go,
            })
            .collect(),
        tables,
    }))
}

/// Returns the tree of each pattern, or the fault of each pattern that the parser cannot read.
///
/// # Errors
///
/// This function returns one error for each rule whose pattern is not a valid regular expression.
fn parse(rules: &[Rule]) -> Result<Vec<Node>, Vec<GenerateError>> {
    let mut nodes = Vec::with_capacity(rules.len());
    let mut errors = Vec::new();

    for (index, rule) in rules.iter().enumerate() {
        match rule.pattern.node() {
            Ok(node) => nodes.push(node),
            Err(error) => errors.push(GenerateErrorKind::Pattern(error).in_rule(index)),
        }
    }

    if errors.is_empty() {
        Ok(nodes)
    } else {
        Err(errors)
    }
}

/// Returns the lexicon of `specification`, with one rule for each tree of `nodes`.
///
/// The accept of a rule is its index, thus the earliest rule wins a tie.
///
/// # Errors
///
/// This function returns one error for each rule that fails a check of the lexicon, and one error
/// for a lexer above [`MAX_RULES`] rules.
fn build(
    specification: &Specification,
    nodes: Vec<Node>,
) -> Result<Lexicon<u16>, Vec<GenerateError>> {
    let count = specification.rules.len();
    if count > MAX_RULES {
        return Err(vec![
            GenerateErrorKind::TooManyRules {
                count,
                maximum: MAX_RULES,
            }
            .in_lexer(),
        ]);
    }

    let declared = specification
        .conditions
        .as_ref()
        .map_or(1, |conditions| conditions.names.len().max(1));
    let mut lexicon = Lexicon::new();
    for _ in 1..declared {
        lexicon.condition();
    }

    let mut errors = Vec::new();
    for (index, (rule, node)) in specification.rules.iter().zip(nodes).enumerate() {
        let accept = u16::try_from(index).expect("the count is at most MAX_RULES");
        let under = if rule.conditions.is_empty() {
            vec![0]
        } else {
            rule.conditions.clone()
        };
        if let Err(error) = lexicon.rule(node, accept, &under) {
            errors.push(GenerateError {
                rule: error.rule,
                kind: GenerateErrorKind::Rule(error.kind),
            });
        }
    }

    if errors.is_empty() {
        Ok(lexicon)
    } else {
        Err(errors)
    }
}

/// Returns the error of a lexer that needs a larger automaton than one automaton holds.
fn overflow(overflow: Overflow) -> Vec<GenerateError> {
    vec![failed(BuildErrorKind::from(overflow))]
}

/// Returns the error of a fault of the whole lexer.
fn failed(kind: BuildErrorKind) -> GenerateError {
    GenerateErrorKind::Rule(kind).in_lexer()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regex::ParseErrorKind;
    use proc_macro2::Span;
    use quote::quote;

    /// Returns an identifier of `name` at the span of the call.
    fn name(name: &str) -> Ident {
        Ident::new(name, Span::call_site())
    }

    /// Returns a rule that matches the regular expression `pattern` under the first condition.
    fn rule(pattern: &str, token: &str) -> Rule {
        Rule {
            pattern: Pattern::Regex(pattern.to_owned()),
            token: Some(name(token)),
            conditions: Vec::new(),
            go: None,
        }
    }

    /// Returns a lexer of `rules` that reads under one start condition.
    fn lexer(rules: Vec<Rule>) -> Specification {
        Specification {
            token: name("Token"),
            conditions: None,
            rules,
        }
    }

    /// Returns the errors of `specification`, which does not build.
    fn errors(specification: &Specification) -> Vec<GenerateError> {
        generate(specification).expect_err("the lexer does not build")
    }

    #[test]
    fn a_lexer_of_valid_rules_gives_its_source() {
        let source = generate(&lexer(vec![
            rule("[a-z]+", "Word"),
            rule("[0-9]+", "Number"),
        ]))
        .expect("the rules are valid");

        assert!(source.to_string().contains(":: lxr :: Lexer for Token"));
    }

    #[test]
    fn a_pattern_that_the_parser_cannot_read_names_its_rule() {
        let found = errors(&lexer(vec![rule("[a-z]+", "Word"), rule("a(b", "Broken")]));

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].rule, Some(1));
        assert!(matches!(found[0].kind, GenerateErrorKind::Pattern(_)));
    }

    #[test]
    fn each_pattern_at_fault_gives_its_own_error() {
        let found = errors(&lexer(vec![rule("a(b", "One"), rule("[z-a]", "Two")]));

        assert_eq!(found.len(), 2);
        assert_eq!(found[0].rule, Some(0));
        assert_eq!(found[1].rule, Some(1));
        assert!(matches!(
            &found[1].kind,
            GenerateErrorKind::Pattern(error)
                if matches!(error.kind, ParseErrorKind::InvertedRange { low: 'z', high: 'a' })
        ));
    }

    #[test]
    fn a_pattern_that_matches_the_empty_string_names_its_rule() {
        let found = errors(&lexer(vec![rule("[a-z]+", "Word"), rule("a*", "Empty")]));

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].rule, Some(1));
        assert_eq!(
            found[0].kind,
            GenerateErrorKind::Rule(BuildErrorKind::MatchesEmpty)
        );
    }

    #[test]
    fn a_rule_that_names_a_condition_of_no_lexer_names_its_rule() {
        let mut specification = lexer(vec![rule("[a-z]+", "Word")]);
        specification.rules[0].conditions = vec![3];

        let found = errors(&specification);

        assert_eq!(found[0].rule, Some(0));
        assert!(matches!(
            found[0].kind,
            GenerateErrorKind::Rule(BuildErrorKind::UnknownCondition { .. })
        ));
    }

    #[test]
    fn each_error_gives_a_message_and_a_correction() {
        let found = errors(&lexer(vec![rule("a*", "Empty"), rule("b*", "Also")]));

        for error in found {
            assert!(!error.to_string().is_empty());
            assert!(error.kind.help().is_some_and(|help| help.ends_with('.')));
        }
    }

    #[test]
    fn a_literal_matches_a_regex_character_and_needs_no_escape() {
        let specification = Specification {
            token: name("Token"),
            conditions: None,
            rules: vec![Rule {
                pattern: Pattern::Literal("a+b".to_owned()),
                token: Some(name("Plus")),
                conditions: Vec::new(),
                go: None,
            }],
        };

        assert!(generate(&specification).is_ok());
    }

    #[test]
    fn an_empty_literal_matches_the_empty_string_and_is_rejected() {
        let specification = Specification {
            token: name("Token"),
            conditions: None,
            rules: vec![Rule {
                pattern: Pattern::Literal(String::new()),
                token: Some(name("Nothing")),
                conditions: Vec::new(),
                go: None,
            }],
        };

        assert_eq!(
            errors(&specification)[0].kind,
            GenerateErrorKind::Rule(BuildErrorKind::MatchesEmpty)
        );
    }

    #[test]
    fn a_lexer_of_two_conditions_reads_the_rules_of_each_one() {
        let specification = Specification {
            token: name("Token"),
            conditions: Some(Conditions {
                kind: quote!(Context),
                names: vec![quote!(Context::Code), quote!(Context::Text)],
            }),
            rules: vec![
                Rule {
                    pattern: Pattern::Literal("\"".to_owned()),
                    token: Some(name("Quote")),
                    conditions: vec![0],
                    go: Some(1),
                },
                Rule {
                    pattern: Pattern::Regex("[^\"]+".to_owned()),
                    token: Some(name("Text")),
                    conditions: vec![1],
                    go: None,
                },
            ],
        };

        let source = generate(&specification)
            .expect("the rules are valid")
            .to_string();

        assert!(source.contains("type Condition = Context ;"));
        assert!(source.contains("going (1)"));
    }

    #[test]
    fn a_rule_that_skips_gives_no_arm_and_still_matches() {
        let specification = lexer(vec![
            rule("[a-z]+", "Word"),
            Rule {
                pattern: Pattern::Regex("[ ]+".to_owned()),
                token: None,
                conditions: Vec::new(),
                go: None,
            },
        ]);

        let source = generate(&specification)
            .expect("the rules are valid")
            .to_string();

        assert!(source.contains(":: lxr :: Action :: skip ()"));
    }
}
