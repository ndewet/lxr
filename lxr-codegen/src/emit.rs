//! Emits the source of a lexer from its tables.
//!
//! [`emit`] gives the `impl` of the [`Lexer`] trait of the runtime crate. It holds the tables as
//! statics, and it holds the maps between a number and a name. The derive macro places the result
//! in the crate of the lexer author.
//!
//! The emitted source names the runtime as `::lxr`, thus it does not depend on what the author
//! imported.
//!
//! [`Lexer`]: https://docs.rs/lxr/latest/lxr/trait.Lexer.html

#![allow(dead_code)]

use proc_macro2::{Ident, Literal, TokenStream};
use quote::quote;

use crate::table::Tables;

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

/// A lexer that is ready to emit.
///
/// [`rules`](Self::rules) is in the sequence of the rules of the lexicon, thus the rule at the
/// index `n` is the rule that the accept `n` of [`tables`](Self::tables) names.
#[derive(Debug, Clone)]
pub struct Emission {
    /// The name of the enum of the tokens.
    pub token: Ident,
    /// The type of the start conditions, or `None` if the lexer reads under one condition.
    pub condition: Option<TokenStream>,
    /// One expression for each start condition, in the sequence of the indexes.
    pub conditions: Vec<TokenStream>,
    /// What each rule gives.
    pub rules: Vec<Rule>,
    /// The automaton of the lexer.
    pub tables: Tables,
}

/// Emits the `impl` of the runtime trait for the token enum of `lexer`.
///
/// The statics live inside an anonymous `const`, thus they do not reach the module of the author
/// and two lexers in one module do not collide.
pub fn emit(lexer: &Emission) -> TokenStream {
    let token = &lexer.token;
    let tables = &lexer.tables;

    let classes = array(tables.classes());
    let next = array(tables.next());
    let accept = array(tables.accept());
    let start = array(tables.start());
    let actions = actions(&lexer.rules);

    let class_count = count(tables.classes().len());
    let next_count = count(tables.next().len());
    let accept_count = count(tables.accept().len());
    let start_count = count(tables.start().len());
    let action_count = count(lexer.rules.len());
    let width = count(tables.width());

    let condition = condition_type(lexer);
    let of_index = of_index(lexer);
    let to_index = to_index(lexer);
    let of_rule = of_rule(lexer);

    quote! {
        const _: () = {
            static CLASSES: [u16; #class_count] = [#(#classes),*];
            static NEXT: [u16; #next_count] = [#(#next),*];
            static ACCEPT: [u16; #accept_count] = [#(#accept),*];
            static START: [u16; #start_count] = [#(#start),*];
            static ACTIONS: [::lxr::Action; #action_count] = [#(#actions),*];

            #[automatically_derived]
            impl ::lxr::Lexer for #token {
                type Condition = #condition;

                const TABLES: ::lxr::Tables<'static> = ::lxr::Tables {
                    classes: &CLASSES,
                    next: &NEXT,
                    width: #width,
                    accept: &ACCEPT,
                    start: &START,
                    actions: &ACTIONS,
                };

                #of_rule
                #of_index
                #to_index
            }
        };
    }
}

/// Returns the literal of each value of `values`.
fn array(values: &[u16]) -> Vec<Literal> {
    values
        .iter()
        .map(|&value| Literal::u16_unsuffixed(value))
        .collect()
}

/// Returns the literal of `value` as a length or an index.
fn count(value: usize) -> Literal {
    Literal::usize_unsuffixed(value)
}

/// Returns the [`Action`](lxr::Action) of each rule.
fn actions(rules: &[Rule]) -> Vec<TokenStream> {
    rules
        .iter()
        .map(|rule| {
            let make = if rule.token.is_some() {
                quote!(::lxr::Action::token())
            } else {
                quote!(::lxr::Action::skip())
            };
            match rule.go {
                Some(condition) => {
                    let condition = Literal::u16_unsuffixed(condition);
                    quote!(#make.going(#condition))
                }
                None => make,
            }
        })
        .collect()
}

/// Returns the type of the start conditions of `lexer`.
fn condition_type(lexer: &Emission) -> TokenStream {
    lexer.condition.clone().unwrap_or_else(|| quote!(()))
}

/// Returns the `token` function, which maps a rule and the text of its match onto the variant that
/// the rule gives.
///
/// A variant that holds a field reads that field from the text with [`FromStr`](std::str::FromStr).
/// A text that the field does not hold gives `None`, and the scan reports it.
fn of_rule(lexer: &Emission) -> TokenStream {
    let token = &lexer.token;
    let arms = lexer.rules.iter().enumerate().filter_map(|(index, rule)| {
        let variant = rule.token.as_ref()?;
        let index = count(index);
        Some(match &rule.value {
            Some(value) => quote! {
                #index => ::core::result::Result::ok(
                    <#value as ::core::str::FromStr>::from_str(text)
                )
                .map(#token::#variant)
            },
            None => quote!(#index => ::core::option::Option::Some(#token::#variant)),
        })
    });

    let text = if lexer.rules.iter().any(|rule| rule.value.is_some()) {
        quote!(text)
    } else {
        quote!(_text)
    };

    quote! {
        fn token(rule: u16, #text: &str) -> ::core::option::Option<Self> {
            match rule {
                #(#arms,)*
                rule => panic!("rule {rule} of this lexer gives no token"),
            }
        }
    }
}

/// Returns the `condition` function, which maps an index onto the start condition at it.
fn of_index(lexer: &Emission) -> TokenStream {
    if lexer.condition.is_none() {
        return quote! {
            fn condition(_index: u16) {}
        };
    }

    let condition = condition_type(lexer);
    let arms = lexer.conditions.iter().enumerate().map(|(index, name)| {
        let index = count(index);
        quote!(#index => #name)
    });

    quote! {
        fn condition(index: u16) -> #condition {
            match index {
                #(#arms,)*
                index => panic!("condition {index} is not a start condition of this lexer"),
            }
        }
    }
}

/// Returns the `index` function, which maps a start condition onto its index.
fn to_index(lexer: &Emission) -> TokenStream {
    if lexer.condition.is_none() {
        return quote! {
            fn index(_condition: Self::Condition) -> u16 {
                0
            }
        };
    }

    let condition = condition_type(lexer);
    let arms = lexer.conditions.iter().enumerate().map(|(index, name)| {
        let index = count(index);
        quote!(#name => #index)
    });

    quote! {
        fn index(condition: #condition) -> u16 {
            match condition {
                #(#arms,)*
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::{Accepts, Bytes, Lexicon, compile};
    use crate::regex::Node;
    use proc_macro2::Span;

    /// Returns an identifier of `name` at the span of the call.
    fn name(name: &str) -> Ident {
        Ident::new(name, Span::call_site())
    }

    /// Returns `true` if the text of `source` holds the text of `part`.
    ///
    /// Both go through the same tokenizer, thus the spacing of the two agrees.
    fn holds(source: &TokenStream, part: &TokenStream) -> bool {
        source.to_string().contains(&part.to_string())
    }

    /// Builds the tables of a lexer of `conditions` start conditions and the rules of `patterns`.
    fn tables(conditions: usize, patterns: &[(&str, &[usize])]) -> Tables {
        let mut lexicon = Lexicon::new();
        for _ in 1..conditions {
            lexicon.condition();
        }
        for (index, (pattern, under)) in patterns.iter().enumerate() {
            let pattern: Node = pattern.parse().expect("the pattern is valid");
            let rule = u16::try_from(index).expect("a test holds few rules");
            lexicon
                .rule(pattern, rule, under)
                .expect("the rule passes each check");
        }

        let (nfa, accepts) = compile(Bytes, lexicon).expect("a test stays below the capacity");
        let determinization = nfa.determinize().expect("a test stays below the capacity");
        let accepts: Accepts<u16> = accepts.determinized(&determinization.subsets);
        Tables::new(&determinization.dfa, &accepts).expect("a test is small")
    }

    /// Builds a lexer of a code condition and a string condition.
    ///
    /// Rule 0 opens a string, rule 1 reads a word, rule 2 reads the text of a string, and rule 3
    /// skips a space.
    fn lexer() -> Emission {
        Emission {
            token: name("Token"),
            condition: Some(quote!(Context)),
            conditions: vec![quote!(Context::Code), quote!(Context::Text)],
            rules: vec![
                Rule {
                    token: Some(name("Quote")),
                    value: None,
                    go: Some(1),
                },
                Rule {
                    token: Some(name("Word")),
                    value: None,
                    go: None,
                },
                Rule {
                    token: Some(name("Text")),
                    value: None,
                    go: Some(0),
                },
                Rule {
                    token: None,
                    value: None,
                    go: None,
                },
            ],
            tables: tables(
                2,
                &[
                    ("\"", &[0]),
                    ("[a-z]+", &[0]),
                    ("[^\"]+", &[1]),
                    (" +", &[0]),
                ],
            ),
        }
    }

    /// Builds a lexer that reads under one start condition.
    fn simple() -> Emission {
        Emission {
            token: name("Word"),
            condition: None,
            conditions: Vec::new(),
            rules: vec![Rule {
                token: Some(name("Letters")),
                value: None,
                go: None,
            }],
            tables: tables(1, &[("[a-z]+", &[0])]),
        }
    }

    #[test]
    fn the_source_implements_the_runtime_trait_for_the_enum_of_the_tokens() {
        let source = emit(&lexer());

        assert!(holds(&source, &quote!(impl ::lxr::Lexer for Token)));
        assert!(holds(
            &source,
            &quote!(
                type Condition = Context;
            )
        ));
    }

    #[test]
    fn the_statics_live_inside_an_anonymous_const() {
        let source = emit(&lexer());

        assert!(
            source
                .to_string()
                .starts_with(&quote!(const _: () =).to_string())
        );
        assert!(holds(&source, &quote!(static CLASSES: [u16; 256])));
    }

    #[test]
    fn each_array_holds_the_values_of_its_table() {
        let lexer = lexer();
        let source = emit(&lexer);
        let tables = &lexer.tables;

        let width = count(tables.width());
        assert!(holds(&source, &quote!(width: #width)));

        let next = count(tables.next().len());
        assert!(holds(&source, &quote!(static NEXT: [u16; #next])));

        let accept = count(tables.accept().len());
        assert!(holds(&source, &quote!(static ACCEPT: [u16; #accept])));

        let classes = array(tables.classes());
        assert!(holds(&source, &quote!([#(#classes),*])));
    }

    #[test]
    fn the_table_of_the_source_names_each_static() {
        let lexer = lexer();
        let source = emit(&lexer);
        let width = count(lexer.tables.width());

        assert!(holds(
            &source,
            &quote!(const TABLES: ::lxr::Tables<'static> = ::lxr::Tables {
                classes: &CLASSES,
                next: &NEXT,
                width: #width,
                accept: &ACCEPT,
                start: &START,
                actions: &ACTIONS,
            })
        ));
    }

    #[test]
    fn each_rule_that_gives_a_token_gets_an_arm() {
        let source = emit(&lexer());

        for (index, variant) in [(0, "Quote"), (1, "Word"), (2, "Text")] {
            let index = count(index);
            let variant = name(variant);
            assert!(holds(
                &source,
                &quote!(#index => ::core::option::Option::Some(Token::#variant))
            ));
        }
    }

    /// Builds a lexer of one rule whose token carries a value.
    fn valued() -> Emission {
        Emission {
            token: name("Token"),
            condition: None,
            conditions: Vec::new(),
            rules: vec![Rule {
                token: Some(name("Int")),
                value: Some(quote!(u64)),
                go: None,
            }],
            tables: tables(1, &[("[0-9]+", &[0])]),
        }
    }

    #[test]
    fn a_rule_that_carries_a_value_reads_its_field_from_the_text() {
        let source = emit(&valued());

        assert!(holds(&source, &quote!(fn token(rule: u16, text: &str))));
        assert!(holds(
            &source,
            &quote!(<u64 as ::core::str::FromStr>::from_str(text))
        ));
        assert!(holds(&source, &quote!(.map(Token::Int))));
    }

    #[test]
    fn a_lexer_whose_tokens_carry_no_value_ignores_the_text_of_the_match() {
        let source = emit(&simple());

        assert!(holds(&source, &quote!(fn token(rule: u16, _text: &str))));
        assert!(!holds(&source, &quote!(FromStr)));
    }

    #[test]
    fn a_rule_that_skips_gets_no_arm_and_the_scan_never_asks_for_it() {
        let source = emit(&lexer());

        assert!(!holds(&source, &quote!(3 => Token::)));
        assert!(holds(
            &source,
            &quote!(rule => panic!("rule {rule} of this lexer gives no token"))
        ));
    }

    #[test]
    fn the_action_of_a_rule_says_that_it_skips_and_where_it_goes() {
        let source = emit(&lexer());

        assert!(holds(
            &source,
            &quote!([
                ::lxr::Action::token().going(1),
                ::lxr::Action::token(),
                ::lxr::Action::token().going(0),
                ::lxr::Action::skip()
            ])
        ));
    }

    #[test]
    fn a_start_condition_maps_onto_its_index_and_back() {
        let source = emit(&lexer());

        assert!(holds(&source, &quote!(0 => Context::Code)));
        assert!(holds(&source, &quote!(1 => Context::Text)));
        assert!(holds(&source, &quote!(Context::Code => 0)));
        assert!(holds(&source, &quote!(Context::Text => 1)));
    }

    #[test]
    fn a_lexer_of_one_start_condition_reads_the_unit_type() {
        let source = emit(&simple());

        assert!(holds(
            &source,
            &quote!(
                type Condition = ();
            )
        ));
        assert!(holds(
            &source,
            &quote!(
                fn condition(_index: u16) {}
            )
        ));
        assert!(holds(
            &source,
            &quote!(
                fn index(_condition: Self::Condition) -> u16 {
                    0
                }
            )
        ));
        assert!(!holds(&source, &quote!(Context)));
    }

    #[test]
    fn the_source_names_the_runtime_and_not_an_import_of_the_author() {
        let source = emit(&simple()).to_string();

        assert!(!source.contains("use "));
        assert!(source.contains(":: lxr :: Lexer"));
    }
}
