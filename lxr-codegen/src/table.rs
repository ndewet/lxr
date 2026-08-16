//! The tables of a lexer, in the form that the emitted source holds.
//!
//! A [`DeterministicFiniteAutomaton`] holds the transitions of one state as a list of labels, and a
//! scan finds the transition of a symbol with a binary search. An emitted lexer cannot pay for that
//! search. [`Tables`] holds the same automaton as a table, thus a step reads two indexes and no
//! label.
//!
//! The table divides the bytes into classes. Two bytes that each label treats in the same manner
//! belong to one class, thus one column of the table serves them both. A lexer of 200 rules holds a
//! few tens of classes, and not 256.
//!
//! Only lxr builds a table, thus this module panics for a table that does not agree with its
//! automaton. A state count above the capacity is a limit and not a defect, thus it gives an
//! [`Overflow`].

#![allow(dead_code)]

use crate::automata::{
    Automaton, DeterministicFiniteAutomaton, Label, Overflow, Part, Range, StateId,
};
use crate::compiler::{Accepts, ByteRange};

/// The maximum number of the states of an automaton that a table holds.
///
/// A table numbers its states in a `u16`, and it keeps the value 0 for the dead state. Thus the
/// highest state of the automaton is [`u16::MAX`].
///
/// [`MAX_STATES`](crate::automata::MAX_STATES) is the limit of determinization. This limit is the
/// limit of the emitted source, thus it belongs here and not there.
pub const MAX_STATES: usize = u16::MAX as usize;

/// The maximum number of the rules of a lexer that a table holds.
///
/// A table numbers its rules in a `u16`, and it keeps the value 0 for a state that does not accept.
pub const MAX_RULES: usize = u16::MAX as usize;

/// The transitions of a lexer, the accepts, and the start states, as tables.
///
/// State 0 is the dead state, and state `n + 1` is the state `n` of the automaton. Class 0 is the
/// dead class, and no byte of a label belongs to it. Thus a step needs no branch for a byte that no
/// label matches, and it reads the dead row and gives the dead state again.
///
/// A step reads [`classes`](Self::classes) with the byte, then it reads [`next`](Self::next) at the
/// state and that class:
///
/// ```text
/// state = next[state * width + classes[byte]]
/// ```
///
/// The scan stops when `state` is 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tables {
    classes: [u16; 256],
    representatives: Vec<u8>,
    width: usize,
    next: Vec<u16>,
    accept: Vec<u16>,
    start: Vec<u16>,
}

impl Tables {
    /// Builds the tables of `dfa`, in which each state accepts the rule that `accepts` gives.
    ///
    /// The accept of a state is the index of the rule, and the table holds that index plus one.
    /// Thus the value 0 means that the state does not accept.
    ///
    /// # Errors
    ///
    /// This function returns an [`Overflow`] if `dfa` holds more than [`MAX_STATES`] states.
    ///
    /// # Panics
    ///
    /// This function panics if `accepts` does not hold one accept for each state of `dfa`, or if an
    /// accept of `accepts` is at or above [`MAX_RULES`].
    pub fn new(
        dfa: &DeterministicFiniteAutomaton<ByteRange>,
        accepts: &Accepts<u16>,
    ) -> Result<Self, Overflow> {
        let count = dfa.state_count();
        assert_eq!(
            accepts.state_count(),
            count,
            "an automaton of {count} states needs one accept for each of them, and not {}",
            accepts.state_count()
        );
        if count > MAX_STATES {
            return Err(Overflow::new(Part::States, MAX_STATES));
        }

        let (classes, representatives) = divide(dfa);
        let width = representatives.len() + 1;

        Ok(Self {
            next: transitions(dfa, &representatives, width),
            accept: marks(dfa, accepts),
            start: starts(dfa),
            classes,
            representatives,
            width,
        })
    }

    /// Returns the class of each byte. Class 0 means that no label matches the byte.
    pub fn classes(&self) -> &[u16; 256] {
        &self.classes
    }

    /// Returns one byte of each class, the byte of the first class first.
    ///
    /// Class 0 is the dead class, and no byte belongs to it. Thus the byte of class `n` is at the
    /// index `n - 1`.
    pub fn representatives(&self) -> &[u8] {
        &self.representatives
    }

    /// Returns the number of the columns of one row of [`next`](Self::next).
    pub fn width(&self) -> usize {
        self.width
    }

    /// Returns the state that each state and each class goes to, one row for each state.
    ///
    /// The row of the state `s` starts at `s * width`, and the class `c` is at the column `c`.
    pub fn next(&self) -> &[u16] {
        &self.next
    }

    /// Returns the rule that each state accepts, plus one, or 0 if the state does not accept.
    pub fn accept(&self) -> &[u16] {
        &self.accept
    }

    /// Returns the state at which each start condition begins a scan.
    pub fn start(&self) -> &[u16] {
        &self.start
    }

    /// Returns the number of the states of the table, the dead state included.
    pub fn state_count(&self) -> usize {
        self.accept.len()
    }
}

/// Returns the class of each byte, and one byte of each class.
///
/// [`Range::classes`] divides the labels. It reads each label of each state, thus one class serves
/// each state and the table needs one column for it.
fn divide(dfa: &DeterministicFiniteAutomaton<ByteRange>) -> ([u16; 256], Vec<u8>) {
    let labels: Vec<ByteRange> = (0..dfa.state_count())
        .flat_map(|index| dfa.transitions(StateId::new(index)))
        .map(|transition| transition.label)
        .collect();

    let mut classes = [0; 256];
    let mut representatives = Vec::new();
    for (index, (class, symbol)) in ByteRange::classes(&labels).into_iter().enumerate() {
        let number = u16::try_from(index + 1).expect("a byte alphabet gives at most 256 classes");
        for byte in class.low..=class.high {
            classes[usize::from(byte)] = number;
        }
        representatives.push(symbol);
    }

    (classes, representatives)
}

/// Returns the row of each state of `dfa`, the dead row first.
///
/// A label matches each byte of a class or no byte of it, thus one byte of the class gives the
/// answer for the whole class. [`Range::classes`] gives that byte in `representatives`.
fn transitions(
    dfa: &DeterministicFiniteAutomaton<ByteRange>,
    representatives: &[u8],
    width: usize,
) -> Vec<u16> {
    let mut next = vec![0; (dfa.state_count() + 1) * width];

    for index in 0..dfa.state_count() {
        let row = (index + 1) * width;
        for transition in dfa.transitions(StateId::new(index)) {
            let target =
                u16::try_from(transition.target.index() + 1).expect("the count is below the limit");
            for (class, &symbol) in representatives.iter().enumerate() {
                if !transition.label.matches(symbol) {
                    continue;
                }
                let slot = &mut next[row + class + 1];
                debug_assert!(
                    *slot == 0,
                    "state {index} reads the byte {symbol:#04X} into two states"
                );
                *slot = target;
            }
        }
    }

    next
}

/// Returns the accept of each state of `dfa`, plus one, the dead state first.
fn marks(dfa: &DeterministicFiniteAutomaton<ByteRange>, accepts: &Accepts<u16>) -> Vec<u16> {
    let mut marks = vec![0; dfa.state_count() + 1];

    for index in 0..dfa.state_count() {
        let Some(&rule) = accepts.get(StateId::new(index)) else {
            continue;
        };
        assert!(
            usize::from(rule) < MAX_RULES,
            "a lexer holds at most {MAX_RULES} rules, thus rule {rule} has no number"
        );
        marks[index + 1] = rule + 1;
    }

    marks
}

/// Returns the state of each start condition of `dfa`.
fn starts(dfa: &DeterministicFiniteAutomaton<ByteRange>) -> Vec<u16> {
    dfa.start_states()
        .iter()
        .map(|state| u16::try_from(state.index() + 1).expect("the count is below the limit"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automata::{Execution, Scanner};
    use crate::compiler::{Bytes, Lexicon, compile};
    use crate::regex::Node;

    /// The automaton of a lexer, its accepts, and its tables.
    struct Lexer {
        dfa: DeterministicFiniteAutomaton<ByteRange>,
        accepts: Accepts<u16>,
        tables: Tables,
    }

    /// Builds a lexer of the rules of `patterns`, under the start conditions that each one names.
    ///
    /// The accept of a rule is its index, thus the first rule wins a tie.
    fn build(conditions: usize, patterns: &[(&str, &[usize])]) -> Lexer {
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
        let accepts = accepts.determinized(&determinization.subsets);
        let tables = Tables::new(&determinization.dfa, &accepts).expect("a test is small");

        Lexer {
            dfa: determinization.dfa,
            accepts,
            tables,
        }
    }

    /// Builds a lexer that has a code condition and a string condition.
    ///
    /// The rules are the rules of the tests of [`compile`], which cover a tie of the precedence, a
    /// rule of two conditions, and a character above ASCII.
    fn lexer() -> Lexer {
        build(
            2,
            &[
                ("let|fn", &[0]),
                ("[a-z][a-z0-9]*", &[0]),
                ("\"", &[0, 1]),
                ("[^\"]+", &[1]),
            ],
        )
    }

    /// Returns the longest match at the start of `input`, read from the tables alone.
    ///
    /// This is the scan that the emitted source holds. It reads no automaton.
    fn scan(tables: &Tables, start: usize, input: &str) -> Option<(u16, usize)> {
        let width = tables.width();
        let mut state = usize::from(tables.start()[start]);
        let mut best = accepted(tables, state, 0);

        for (index, &byte) in input.as_bytes().iter().enumerate() {
            let class = usize::from(tables.classes()[usize::from(byte)]);
            state = usize::from(tables.next()[state * width + class]);
            if state == 0 {
                break;
            }
            best = accepted(tables, state, index + 1).or(best);
        }

        best
    }

    /// Returns the match of `length` at `state`, or `None` if the state does not accept.
    fn accepted(tables: &Tables, state: usize, length: usize) -> Option<(u16, usize)> {
        match tables.accept()[state] {
            0 => None,
            rule => Some((rule - 1, length)),
        }
    }

    /// Returns the longest match that the automaton finds, in place of the tables.
    fn reference(lexer: &Lexer, start: usize, input: &str) -> Option<(u16, usize)> {
        lexer
            .dfa
            .execute()
            .longest_match(start, input.as_bytes(), |states| {
                lexer
                    .accepts
                    .lowest(states)
                    .expect("a state that accepts has an accept")
            })
            .map(|found| (found.accept, found.length))
    }

    /// The inputs of the differential test. Each one is a token, a part of a token, or an input
    /// that the lexer rejects.
    const INPUTS: [&str; 18] = [
        "", "l", "let", "letter", "let9", "f", "fn", "fun", "a", "z9", "9", "!", " ", "\"",
        "\"let\"", "a b", "é", "\u{80}",
    ];

    #[test]
    fn the_tables_give_the_same_match_as_the_automaton() {
        let lexer = lexer();

        for start in 0..lexer.dfa.start_count() {
            for input in INPUTS {
                assert_eq!(
                    scan(&lexer.tables, start, input),
                    reference(&lexer, start, input),
                    "input {input:?} under start {start}"
                );
            }
        }
    }

    #[test]
    fn a_row_of_the_table_gives_the_step_of_the_automaton_for_each_byte() {
        let lexer = lexer();
        let tables = &lexer.tables;

        for index in 0..lexer.dfa.state_count() {
            for byte in 0..=u8::MAX {
                let class = usize::from(tables.classes()[usize::from(byte)]);
                let target = tables.next()[(index + 1) * tables.width() + class];
                let expected = lexer
                    .dfa
                    .step(StateId::new(index), byte)
                    .map_or(0, |state| (state.index() + 1) as u16);

                assert_eq!(
                    target, expected,
                    "state {index} reads the byte {byte:#04X} into the wrong state"
                );
            }
        }
    }

    #[test]
    fn the_dead_state_reads_each_byte_into_itself() {
        let tables = lexer().tables;

        assert_eq!(&tables.next()[..tables.width()], vec![0; tables.width()]);
        assert_eq!(tables.accept()[0], 0);
    }

    #[test]
    fn the_dead_class_reads_into_the_dead_state_from_each_state() {
        let tables = lexer().tables;

        for state in 0..tables.state_count() {
            assert_eq!(tables.next()[state * tables.width()], 0, "state {state}");
        }
    }

    #[test]
    fn a_byte_that_no_label_matches_belongs_to_the_dead_class() {
        let tables = build(1, &[("[a-z]", &[0])]).tables;

        assert_eq!(tables.classes()[usize::from(b'\0')], 0);
        assert_eq!(tables.classes()[usize::from(b'A')], 0);
        assert_eq!(tables.classes()[usize::from(b'{')], 0);
        assert_ne!(tables.classes()[usize::from(b'a')], 0);
        assert_ne!(tables.classes()[usize::from(b'z')], 0);
    }

    #[test]
    fn each_class_holds_the_bytes_that_each_label_treats_in_the_same_manner() {
        let tables = lexer().tables;

        assert_eq!(
            tables.classes()[usize::from(b'b')],
            tables.classes()[usize::from(b'c')]
        );
        assert_ne!(
            tables.classes()[usize::from(b'l')],
            tables.classes()[usize::from(b'b')]
        );
        assert_eq!(tables.representatives().len(), tables.width() - 1);
    }

    #[test]
    fn each_representative_belongs_to_the_class_that_it_stands_for() {
        let tables = lexer().tables;

        for (index, &symbol) in tables.representatives().iter().enumerate() {
            let number = u16::try_from(index + 1).expect("a test holds few classes");
            assert_eq!(tables.classes()[usize::from(symbol)], number);
        }
    }

    #[test]
    fn each_start_condition_gets_the_state_of_the_automaton() {
        let lexer = lexer();

        assert_eq!(lexer.tables.start().len(), 2);
        for (start, &state) in lexer.tables.start().iter().enumerate() {
            assert_eq!(
                usize::from(state),
                lexer.dfa.start_state(start).index() + 1,
                "start {start}"
            );
        }
    }

    #[test]
    fn a_state_holds_the_rule_that_it_accepts_plus_one() {
        let lexer = lexer();

        for index in 0..lexer.dfa.state_count() {
            let expected = lexer
                .accepts
                .get(StateId::new(index))
                .map_or(0, |rule| rule + 1);
            assert_eq!(lexer.tables.accept()[index + 1], expected, "state {index}");
        }
        assert!(lexer.tables.accept().iter().any(|&rule| rule != 0));
    }

    #[test]
    fn a_lexer_of_one_rule_gives_one_class_for_each_byte_of_the_rule() {
        let lexer = build(1, &[("[ab]", &[0])]);

        assert_eq!(lexer.tables.width(), 2);
        assert_eq!(lexer.tables.state_count(), 3);
        assert_eq!(scan(&lexer.tables, 0, "a"), Some((0, 1)));
        assert_eq!(scan(&lexer.tables, 0, "z"), None);
    }

    #[test]
    fn a_lexer_of_no_rule_holds_the_dead_state_and_its_start() {
        let lexer = build(1, &[]);

        assert_eq!(lexer.tables.width(), 1);
        assert_eq!(lexer.tables.start().len(), 1);
        assert_eq!(scan(&lexer.tables, 0, "a"), None);
    }

    #[test]
    #[should_panic(expected = "needs one accept for each of them")]
    fn a_table_of_an_automaton_and_the_accepts_of_another_panics() {
        let lexer = lexer();
        let other = build(1, &[("[ab]", &[0])]);

        let _ = Tables::new(&lexer.dfa, &other.accepts);
    }
}
