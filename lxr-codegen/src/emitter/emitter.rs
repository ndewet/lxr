//! Renders minimized lexer automata as Rust source.

use crate::automata::{dfa::Dfa, encoding::ByteRange};
use crate::lexer::{Lexer, ResolvedTransition, RuleId};
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
                let accept = accept.index();
                quote! { #index => Some(#accept), }
            })
        })
        .collect();
    let actions: Vec<_> = lexer
        .rules()
        .iter()
        .enumerate()
        .map(|(index, rule)| {
            let action = rule.action().rendered();
            let transition = match rule.transition() {
                ResolvedTransition::Stay => quote!(::lxr::Transition::Stay),
                ResolvedTransition::Begin(id) => {
                    let id = id.index();
                    quote!(::lxr::Transition::Begin(#id))
                }
                ResolvedTransition::Push(id) => {
                    let id = id.index();
                    quote!(::lxr::Transition::Push(#id))
                }
                ResolvedTransition::Pop => quote!(::lxr::Transition::Pop),
            };
            quote! { #index => (#action).map(|token| (token, #transition)), }
        })
        .collect();
    let mode_names = lexer
        .start_conditions()
        .iter()
        .enumerate()
        .map(|(index, mode)| {
            let name = mode.name();
            quote!(#index => #name,)
        });

    let continuing: Vec<_> = (0..dfa.state_count())
        .filter(|&index| {
            !dfa.transitions(crate::automata::StateId::new(index))
                .is_empty()
        })
        .collect();

    quote! {
        fn __lxr_start(start_condition: usize) -> usize {
            match start_condition {
                #(#starts)*
                _ => unreachable!("unknown lexer mode"),
            }
        }

        fn __lxr_step(state: usize, byte: u8) -> Option<usize> {
            match state { #(#transitions)* _ => None }
        }

        fn __lxr_accept(state: usize) -> Option<usize> {
            match state { #(#accepts)* _ => None }
        }

        fn __lxr_continues(state: usize) -> bool {
            match state { #(#continuing => true,)* _ => false }
        }

        fn __lxr_scan(input: &[u8], start_condition: usize) -> Option<(usize, usize)> {
            let mut state = match start_condition {
                #(#starts)*
                _ => return None,
            };
            let mut latest = match state {
                #(#accepts)*
                _ => None,
            };
            let mut length = latest.map(|_| 0);

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

        fn __lxr_action(
            rule: usize,
            text: &str,
        ) -> Result<(Option<Self>, ::lxr::Transition), ::lxr::PayloadError> {
            match rule {
                #(#actions)*
                _ => unreachable!("generated lexer selected an unknown rule"),
            }
        }

        fn __lxr_mode_name(mode: usize) -> &'static str {
            match mode { #(#mode_names)* _ => "<unknown>", }
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
                ResolvedTransition::Stay,
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

        let actual = emit(&dfa, &Lexer::new(vec![], vec![])).to_string();
        assert!(actual.contains("0usize => 0usize"));
        assert!(actual.contains("Result < (Option < Self > , :: lxr :: Transition)"));
        assert!(actual.contains("fn __lxr_mode_name"));
    }

    #[test]
    fn emitting_a_transition_and_accept_selects_the_rule_before_its_action() {
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
        assert!(emitted.contains("1usize => Some (0usize)"));
        assert!(emitted.contains("Rule :: Token (7)"));
        assert!(emitted.contains("Transition :: Stay"));
    }
}
