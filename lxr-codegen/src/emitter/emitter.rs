//! Renders minimized lexer automata as Rust source.

use crate::automata::{dfa::Dfa, encoding::ByteRange};
use crate::lexer::{Lexer, RuleId};
use proc_macro2::TokenStream;
use quote::quote;

/// Renders a minimized byte-oriented lexer DFA as Rust matcher tokens.
pub(crate) fn emit(dfa: &Dfa<ByteRange, RuleId>, lexer: &Lexer) -> TokenStream {
    let starts: Vec<_> = dfa
        .start_states()
        .iter()
        .enumerate()
        .map(|(index, state)| {
            let state = state.index();
            quote! { #index => #state, }
        })
        .collect();
    let transitions: Vec<_> = (0..dfa.state_count())
        .filter_map(|index| {
            let state = crate::automata::StateId::new(index);
            let transitions = dfa.transitions(state);
            (!transitions.is_empty()).then(|| {
                let arms = transitions.iter().map(|transition| {
                    let low = transition.label.low;
                    let high = transition.label.high;
                    let target = transition.target.index();
                    quote! { #low..=#high => Some(#target), }
                });
                quote! {
                    #index => match byte {
                        #(#arms)*
                        _ => None,
                    },
                }
            })
        })
        .collect();
    let accepts: Vec<_> = (0..dfa.state_count())
        .filter_map(|index| {
            let state = crate::automata::StateId::new(index);
            dfa.accept(state).map(|accept| {
                let accept = lexer.rule(*accept).action().rendered();
                quote! { #index => Some(#accept), }
            })
        })
        .collect();

    quote! {
        fn __lxr_scan(input: &[u8], start_condition: usize) -> Option<(Option<Self>, usize)> {
            let mut state = match start_condition {
                #(#starts)*
                _ => return None,
            };
            let mut latest = match state {
                #(#accepts)*
                _ => None,
            };
            let mut length = latest.as_ref().map(|_| 0);

            for (index, &byte) in input.iter().enumerate() {
                let Some(next) = (match state {
                    #(#transitions)*
                    _ => None,
                }) else {
                    break;
                };
                state = next;

                if let Some(accept) = match state {
                    #(#accepts)*
                    _ => None,
                } {
                    latest = Some(accept);
                    length = Some(index + 1);
                }
            }

            latest.zip(length)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::dfa::Builder;
    use crate::lexer::{Rule, RuleAction, StartCondition};
    use crate::regex::Expression;
    use std::str::FromStr;

    fn lexer(action: TokenStream) -> Lexer {
        Lexer::new(
            vec![Rule::new(
                Expression::from_str("a").expect("the test pattern is valid"),
                RuleAction::Emit(action),
                vec![crate::lexer::StartConditionId::new(0)],
            )],
            vec![StartCondition::new("INITIAL")],
        )
    }

    #[test]
    fn emitting_a_dfa_without_transitions_stops_after_selecting_its_start() {
        let mut builder: Builder<ByteRange, RuleId> = Builder::new();
        let start = builder.add_state();
        let dfa = builder
            .build(&[start])
            .expect("the test DFA is below its capacity");

        let actual = emit(&dfa, &Lexer::new(vec![], vec![]));
        let expected = quote! {
            fn __lxr_scan(input: &[u8], start_condition: usize) -> Option<(Option<Self>, usize)> {
                let mut state = match start_condition {
                    0usize => 0usize,
                    _ => return None,
                };
                let mut latest = match state {
                    _ => None,
                };
                let mut length = latest.as_ref().map(|_| 0);

                for (index, &byte) in input.iter().enumerate() {
                    let Some(next) = (match state {
                        _ => None,
                    }) else {
                        break;
                    };
                    state = next;

                    if let Some(accept) = match state {
                        _ => None,
                    } {
                        latest = Some(accept);
                        length = Some(index + 1);
                    }
                }

                latest.zip(length)
            }
        };

        assert_eq!(actual.to_string(), expected.to_string());
    }

    #[test]
    fn emitting_a_transition_and_accept_uses_the_rule_action() {
        let mut builder: Builder<ByteRange, RuleId> = Builder::new();
        let start = builder.add_state();
        let accept = builder.add_state();
        builder.add_transition(start, ByteRange::new(b'a', b'z'), accept);
        builder.set_accept(accept, RuleId::new(0));
        let dfa = builder
            .build(&[start])
            .expect("the test DFA is below its capacity");
        let emitted = emit(&dfa, &lexer(quote!(Rule::Token(7)))).to_string();

        assert!(emitted.contains("97u8 ..= 122u8 => Some (1usize)"));
        assert!(emitted.contains("1usize => Some (Some (Rule :: Token (7)))"));
    }
}
