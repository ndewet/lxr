//! Builds the source of a lexer from the rules that a lexer author wrote.
//!
//! [`generate`] joins each part of the crate. It parses each pattern, it builds the automaton, it
//! determinizes the automaton, it makes the tables, and it emits the source.
//!
//! The derive macro supplies a [`Specification`], and it holds the span of each rule. Thus this
//! module reports the index of the rule at fault, and the macro turns that index into a span.

use std::collections::HashSet;

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
#[derive(Debug, Clone)]
pub struct Rule {
    /// The pattern that the rule matches.
    pub pattern: Pattern,
    /// The variant that the rule gives, or `None` if the rule skips its match.
    pub token: Option<Ident>,
    /// The type of the field of that variant, or `None` if the variant holds no field.
    ///
    /// The token reads the field from the text of the match with
    /// [`FromStr`](std::str::FromStr).
    pub value: Option<TokenStream>,
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
    /// A rule can never win a match, thus it gives no token. An earlier rule matches each text
    /// that this rule matches, and the earliest rule wins a tie.
    NeverWins,
    /// The scan enters a start condition that no rule reads. The scan then gives a fault at each
    /// byte, and it stays under that condition to the end of the input.
    NeverRead,
    /// A rule reads under a start condition that the scan never enters. No `go` of a rule that the
    /// scan reaches names that condition, thus the rule never matches.
    NeverEntered,
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
            Self::NeverWins => Some(
                "An earlier rule matches each text that this rule matches. Put this rule \
                 before that one, or write a pattern that only this rule matches.",
            ),
            Self::NeverRead => Some(
                "Write a rule that reads under this start condition, or remove the `go` \
                 that enters it.",
            ),
            Self::NeverEntered => Some(
                "Write `go` and this start condition on a rule that the scan reaches, or \
                 read this rule under another condition.",
            ),
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
            Self::NeverWins => write!(formatter, "the rule can never win a match"),
            Self::NeverRead => write!(
                formatter,
                "the scan enters a start condition that no rule reads"
            ),
            Self::NeverEntered => write!(
                formatter,
                "the rule reads under a start condition that the scan never enters"
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
/// read, a rule that the lexicon rejects, a start condition that the scan cannot use, a rule that
/// can never win a match, and a lexer above a limit each give one.
pub fn generate(specification: &Specification) -> Result<TokenStream, Vec<GenerateError>> {
    let nodes = parse(&specification.rules)?;
    let lexicon = build(specification, nodes)?;
    entered(specification)?;

    let (nfa, accepts) = compile(Bytes, lexicon).map_err(|error| vec![failed(error.kind)])?;
    let determinization = nfa.determinize().map_err(overflow)?;
    let accepts = accepts.determinized(&determinization.subsets);
    let tables = Tables::new(&determinization.dfa, &accepts).map_err(overflow)?;
    reachable(&tables, specification.rules.len())?;

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
                value: rule.value.clone(),
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
/// A rule that fails a check does not join the lexicon. Thus the index that [`Lexicon::rule`]
/// reports counts the rules that passed, and the error takes the index of this loop instead.
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
            errors.push(GenerateErrorKind::Rule(error.kind).in_rule(index));
        }
    }

    if errors.is_empty() {
        Ok(lexicon)
    } else {
        Err(errors)
    }
}

/// Returns the start conditions that `rule` reads under.
///
/// An empty list means the first condition, as [`build`] reads it.
fn under(rule: &Rule) -> &[usize] {
    if rule.conditions.is_empty() {
        &[0]
    } else {
        &rule.conditions
    }
}

/// Reports each start condition that the scan cannot use.
///
/// The scan begins under the first condition. A rule that the scan reaches enters another
/// condition with its `go`, thus the conditions that the scan enters grow from the first one. A
/// condition that no rule reads stops the scan, because each byte under it gives a fault. A rule
/// under a condition that the scan never enters cannot match.
///
/// [`build`] rejects a rule that names a condition of no lexicon, thus each index of
/// [`conditions`](Rule::conditions) is in range here. A `go` comes from the caller, thus this
/// function reads it with `get`.
///
/// # Errors
///
/// This function returns one error for each rule that enters a condition that no rule reads, and
/// one error for each rule that reads under no condition that the scan enters. No rule enters the
/// first condition, thus a first condition that no rule reads names the lexer.
fn entered(specification: &Specification) -> Result<(), Vec<GenerateError>> {
    let count = specification
        .conditions
        .as_ref()
        .map_or(1, |conditions| conditions.names.len().max(1));

    let mut read = vec![false; count];
    for rule in &specification.rules {
        for &condition in under(rule) {
            read[condition] = true;
        }
    }

    if !read[0] {
        return Err(vec![GenerateErrorKind::NeverRead.in_lexer()]);
    }

    let mut entered = vec![false; count];
    entered[0] = true;

    let mut moved = true;
    while moved {
        moved = false;
        for rule in &specification.rules {
            let Some(go) = rule.go.map(usize::from) else {
                continue;
            };
            if entered.get(go) != Some(&false) {
                continue;
            }
            if under(rule).iter().any(|&condition| entered[condition]) {
                entered[go] = true;
                moved = true;
            }
        }
    }

    let errors: Vec<GenerateError> = specification
        .rules
        .iter()
        .enumerate()
        .filter_map(|(index, rule)| {
            if !under(rule).iter().any(|&condition| entered[condition]) {
                Some(GenerateErrorKind::NeverEntered.in_rule(index))
            } else if rule
                .go
                .is_some_and(|go| read.get(usize::from(go)) != Some(&true))
            {
                Some(GenerateErrorKind::NeverRead.in_rule(index))
            } else {
                None
            }
        })
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Reports each rule of `count` rules that can never win a match.
///
/// A state of the tables holds the rule of the highest precedence of the rules that accept there.
/// Thus a rule that no state holds loses each match to an earlier rule, and no input gives it. Such
/// a rule is a mistake in the sequence of the rules, and not a rule of the language.
///
/// # Errors
///
/// This function returns one error for each rule that no accept of `tables` names.
fn reachable(tables: &Tables, count: usize) -> Result<(), Vec<GenerateError>> {
    let winners: HashSet<u16> = tables
        .accept()
        .iter()
        .filter_map(|accept| accept.checked_sub(1))
        .collect();

    let errors: Vec<GenerateError> = (0..count)
        .filter(|&rule| {
            let rule = u16::try_from(rule).expect("the count is at most MAX_RULES");
            !winners.contains(&rule)
        })
        .map(|rule| GenerateErrorKind::NeverWins.in_rule(rule))
        .collect();

    if errors.is_empty() {
        Ok(())
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
            value: None,
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

    /// Returns a lexer of `rules` that reads under `count` start conditions.
    fn conditioned(count: usize, rules: Vec<Rule>) -> Specification {
        Specification {
            token: name("Token"),
            conditions: Some(Conditions {
                kind: quote!(Context),
                names: (0..count).map(|_| quote!(Context::Code)).collect(),
            }),
            rules,
        }
    }

    /// Returns a rule of `pattern` that reads under `under`, and that then enters `go`.
    fn conditional(pattern: &str, token: &str, under: Vec<usize>, go: Option<u16>) -> Rule {
        Rule {
            conditions: under,
            go,
            ..rule(pattern, token)
        }
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
    fn each_rule_that_fails_a_check_names_its_own_index() {
        let found = errors(&lexer(vec![
            rule("[a-z]+", "Word"),
            rule("a*", "Empty"),
            rule("b*", "Also"),
        ]));

        assert_eq!(found.len(), 2);
        assert_eq!(found[0].rule, Some(1));
        assert_eq!(found[1].rule, Some(2));
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
                value: None,
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
                value: None,
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
    fn a_rule_that_an_earlier_rule_shadows_names_its_rule() {
        let found = errors(&lexer(vec![rule("[a-z]+", "Word"), rule("let", "Let")]));

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].rule, Some(1));
        assert_eq!(found[0].kind, GenerateErrorKind::NeverWins);
        assert_eq!(found[0].to_string(), "the rule can never win a match");
        assert!(
            found[0]
                .kind
                .help()
                .is_some_and(|help| help.starts_with("An earlier rule"))
        );
    }

    #[test]
    fn two_rules_of_one_pattern_report_the_later_one() {
        let found = errors(&lexer(vec![rule("a", "One"), rule("a", "Two")]));

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].rule, Some(1));
    }

    #[test]
    fn each_rule_that_can_never_win_gives_its_own_error() {
        let found = errors(&lexer(vec![
            rule("[a-z]+", "Word"),
            rule("let", "Let"),
            rule("fn", "Fn"),
        ]));

        assert_eq!(found.len(), 2);
        assert_eq!(found[0].rule, Some(1));
        assert_eq!(found[1].rule, Some(2));
    }

    #[test]
    fn a_rule_that_enters_a_condition_that_no_rule_reads_names_its_rule() {
        let found = errors(&conditioned(
            2,
            vec![conditional("[a-z]+", "Word", vec![0], Some(1))],
        ));

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].rule, Some(0));
        assert_eq!(found[0].kind, GenerateErrorKind::NeverRead);
        assert_eq!(
            found[0].to_string(),
            "the scan enters a start condition that no rule reads"
        );
    }

    #[test]
    fn a_first_condition_that_no_rule_reads_names_the_lexer() {
        let found = errors(&conditioned(
            2,
            vec![conditional("[a-z]+", "Word", vec![1], None)],
        ));

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].rule, None);
        assert_eq!(found[0].kind, GenerateErrorKind::NeverRead);
    }

    #[test]
    fn a_rule_under_a_condition_that_the_scan_never_enters_names_its_rule() {
        let found = errors(&conditioned(
            3,
            vec![
                conditional("[a-z]+", "Word", vec![0], Some(1)),
                conditional("[0-9]+", "Number", vec![1], None),
                conditional("[A-Z]+", "Name", vec![2], None),
            ],
        ));

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].rule, Some(2));
        assert_eq!(found[0].kind, GenerateErrorKind::NeverEntered);
        assert_eq!(
            found[0].to_string(),
            "the rule reads under a start condition that the scan never enters"
        );
    }

    #[test]
    fn each_rule_under_a_condition_that_no_rule_enters_gives_its_own_error() {
        let found = errors(&conditioned(
            2,
            vec![
                conditional("[a-z]+", "Word", vec![0], None),
                conditional("[0-9]+", "Number", vec![1], None),
                conditional("[A-Z]+", "Name", vec![1], None),
            ],
        ));

        assert_eq!(found.len(), 2);
        assert_eq!(found[0].rule, Some(1));
        assert_eq!(found[1].rule, Some(2));
    }

    #[test]
    fn a_chain_of_start_conditions_gives_no_error() {
        let source = generate(&conditioned(
            3,
            vec![
                conditional("[a-z]+", "Word", vec![0], Some(1)),
                conditional("[0-9]+", "Number", vec![1], Some(2)),
                conditional("[A-Z]+", "Name", vec![2], Some(0)),
            ],
        ));

        assert!(source.is_ok());
    }

    #[test]
    fn a_rule_that_skips_under_a_rule_that_shadows_it_names_its_rule() {
        let found = errors(&lexer(vec![
            rule("[a-z]+", "Word"),
            Rule {
                pattern: Pattern::Regex("[a-z]+".to_owned()),
                token: None,
                value: None,
                conditions: Vec::new(),
                go: None,
            },
        ]));

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].rule, Some(1));
    }

    #[test]
    fn a_keyword_before_the_rule_of_a_name_wins_its_own_match() {
        let source = generate(&lexer(vec![rule("let", "Let"), rule("[a-z]+", "Word")]));

        assert!(source.is_ok());
    }

    #[test]
    fn a_rule_of_a_longer_match_leaves_an_earlier_rule_its_own_match() {
        let source = generate(&lexer(vec![
            rule("[a-z]", "One"),
            rule("[a-z][a-z]+", "Many"),
        ]));

        assert!(source.is_ok());
    }

    #[test]
    fn a_rule_that_only_another_start_condition_holds_wins_its_own_match() {
        let specification = Specification {
            token: name("Token"),
            conditions: Some(Conditions {
                kind: quote!(Context),
                names: vec![quote!(Context::Code), quote!(Context::Text)],
            }),
            rules: vec![
                Rule {
                    pattern: Pattern::Regex("[a-z]+".to_owned()),
                    token: Some(name("Word")),
                    value: None,
                    conditions: vec![0],
                    go: Some(1),
                },
                Rule {
                    pattern: Pattern::Regex("[a-z]+".to_owned()),
                    token: Some(name("Body")),
                    value: None,
                    conditions: vec![1],
                    go: None,
                },
            ],
        };

        assert!(generate(&specification).is_ok());
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
                    value: None,
                    conditions: vec![0],
                    go: Some(1),
                },
                Rule {
                    pattern: Pattern::Regex("[^\"]+".to_owned()),
                    token: Some(name("Text")),
                    value: None,
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
                value: None,
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
