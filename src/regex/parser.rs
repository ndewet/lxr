use crate::regex::ast::{Node, Repetitions};
use crate::regex::charset::CharSet;
use crate::regex::cursor::Cursor;
use crate::regex::error::{ParseError, ParseErrorKind as Kind};
use crate::regex::escape::Escape;

const MAX_REPETITION: usize = 65535;
const MAX_NESTING_DEPTH: usize = 250;

pub(crate) struct RegexParser<'a> {
    cursor: Cursor<'a>,
    depth: usize,
}

impl<'a> RegexParser<'a> {
    pub(crate) fn new(pattern: &'a str) -> Self {
        Self {
            cursor: Cursor::new(pattern),
            depth: 0,
        }
    }

    pub(crate) fn parse(mut self) -> Result<Node, ParseError> {
        let pattern = self.parse_alternation()?;
        match self.cursor.peek() {
            None => Ok(pattern),
            Some(')') => Err(Kind::UnmatchedCloseParenthesis.at(self.position())),
            Some(_) => Err(self.unexpected()),
        }
    }

    fn parse_alternation(&mut self) -> Result<Node, ParseError> {
        let mut alternation = self.parse_sequence()?;
        while self.cursor.accept('|') {
            alternation = alternation.alternate(self.parse_sequence()?);
        }
        Ok(alternation)
    }

    fn parse_sequence(&mut self) -> Result<Node, ParseError> {
        let mut sequence = Node::Epsilon;
        while !self.at_sequence_end() {
            sequence = sequence.concat(self.parse_quantified()?);
        }
        Ok(sequence)
    }

    fn parse_quantified(&mut self) -> Result<Node, ParseError> {
        let atom = self.parse_atom()?;
        let quantified = match self.cursor.peek() {
            Some('*') => {
                self.cursor.pop();
                atom.star()
            }
            Some('+') => {
                self.cursor.pop();
                atom.plus()
            }
            Some('?') => {
                self.cursor.pop();
                atom.optional()
            }
            Some('{') => match self.parse_repetition()? {
                Some(repetitions) => atom.repeated(repetitions),
                None => return Ok(atom),
            },
            _ => return Ok(atom),
        };
        self.reject_stacked_quantifier()?;
        Ok(quantified)
    }

    fn reject_stacked_quantifier(&mut self) -> Result<(), ParseError> {
        let position = self.position();
        match self.cursor.peek() {
            Some(found @ ('*' | '+' | '?')) => Err(Kind::RepeatedQuantifier(found).at(position)),
            Some('{') => match self.parse_repetition()? {
                Some(_) => Err(Kind::RepeatedQuantifier('{').at(position)),
                None => Ok(()),
            },
            _ => Ok(()),
        }
    }

    fn parse_atom(&mut self) -> Result<Node, ParseError> {
        let position = self.position();
        match self.cursor.peek() {
            None => Err(Kind::UnexpectedEnd.at(position)),
            Some('(') => {
                self.cursor.pop();
                if self.cursor.peek() == Some('?') {
                    return Err(Kind::UnsupportedGroup.at(position));
                }
                self.depth += 1;
                if self.depth > MAX_NESTING_DEPTH {
                    return Err(Kind::NestingTooDeep(MAX_NESTING_DEPTH).at(position));
                }
                let group = self.parse_alternation()?;
                self.depth -= 1;
                if !self.cursor.accept(')') {
                    return Err(Kind::UnclosedGroup.at(position));
                }
                Ok(group)
            }
            Some('[') => Ok(Node::Class(self.parse_class()?)),
            Some('.') => {
                self.cursor.pop();
                Ok(Node::Class(CharSet::single('\n').negate()))
            }
            Some('\\') => Ok(Node::Class(self.parse_escape()?.into_set())),
            Some(quantifier @ ('*' | '+' | '?')) => {
                Err(Kind::NothingToRepeat(quantifier).at(position))
            }
            Some('{') if self.parse_repetition()?.is_some() => {
                Err(Kind::NothingToRepeat('{').at(position))
            }
            Some(anchor @ ('^' | '$')) => Err(Kind::UnsupportedAnchor(anchor).at(position)),
            Some(literal) => {
                self.cursor.pop();
                Ok(Node::Class(CharSet::single(literal)))
            }
        }
    }

    fn parse_repetition(&mut self) -> Result<Option<Repetitions>, ParseError> {
        let checkpoint = self.cursor.clone();
        let position = self.position();
        self.cursor.pop();
        match self.parse_repetition_bounds()? {
            Some(Repetitions::Range(minimum, maximum)) if maximum < minimum => {
                Err(Kind::InvertedRepetition { minimum, maximum }.at(position))
            }
            Some(repetitions) => Ok(Some(repetitions)),
            None => {
                self.cursor = checkpoint;
                Ok(None)
            }
        }
    }

    fn parse_repetition_bounds(&mut self) -> Result<Option<Repetitions>, ParseError> {
        let Some(minimum) = self.parse_bound()? else {
            return Ok(None);
        };
        if self.cursor.accept('}') {
            return Ok(Some(Repetitions::Range(minimum, minimum)));
        }
        if !self.cursor.accept(',') {
            return Ok(None);
        }
        if self.cursor.accept('}') {
            return Ok(Some(Repetitions::AtLeast(minimum)));
        }
        let Some(maximum) = self.parse_bound()? else {
            return Ok(None);
        };
        if !self.cursor.accept('}') {
            return Ok(None);
        }
        Ok(Some(Repetitions::Range(minimum, maximum)))
    }

    fn parse_bound(&mut self) -> Result<Option<usize>, ParseError> {
        let position = self.position();
        let Some(first) = self.cursor.pop_digit(10) else {
            return Ok(None);
        };
        let mut bound = first as usize;
        while let Some(digit) = self.cursor.pop_digit(10) {
            bound = bound * 10 + digit as usize;
            if bound > MAX_REPETITION {
                return Err(Kind::RepetitionTooLarge.at(position));
            }
        }
        Ok(Some(bound))
    }

    fn parse_class(&mut self) -> Result<CharSet, ParseError> {
        let opened_at = self.position();
        self.cursor.pop();
        let negated = self.cursor.accept('^');
        let mut set = if self.cursor.accept(']') {
            CharSet::single(']')
        } else {
            CharSet::empty()
        };
        loop {
            match self.cursor.peek() {
                None => return Err(Kind::UnclosedClass.at(opened_at)),
                Some(']') => {
                    self.cursor.pop();
                    break;
                }
                Some(_) => set = set.union(&self.parse_class_item(opened_at)?),
            }
        }
        let set = if negated { set.negate() } else { set };
        if set.is_empty() {
            return Err(Kind::EmptyClass.at(opened_at));
        }
        Ok(set)
    }

    fn parse_class_item(&mut self, opened_at: usize) -> Result<CharSet, ParseError> {
        let position = self.position();
        let low = match self.parse_class_atom(opened_at)? {
            Escape::Set(_, set) => return Ok(set),
            Escape::Char(character) => character,
        };
        if !self.at_range_dash() {
            return Ok(CharSet::single(low));
        }
        self.cursor.pop();

        let end_position = self.position();
        let high = match self.parse_class_atom(opened_at)? {
            Escape::Set(escape, _) => {
                return Err(Kind::ClassEscapeInRange(escape).at(end_position));
            }
            Escape::Char(character) => character,
        };
        if low > high {
            return Err(Kind::InvertedRange { low, high }.at(position));
        }
        Ok(CharSet::range(low, high))
    }

    fn parse_class_atom(&mut self, opened_at: usize) -> Result<Escape, ParseError> {
        match self.cursor.peek() {
            None => Err(Kind::UnclosedClass.at(opened_at)),
            Some('[') if self.cursor.peek_ahead() == Some(':') => {
                Err(Kind::UnsupportedPosixClass.at(self.position()))
            }
            Some('\\') => self.parse_escape(),
            Some(literal) => {
                self.cursor.pop();
                Ok(Escape::Char(literal))
            }
        }
    }

    fn parse_escape(&mut self) -> Result<Escape, ParseError> {
        let position = self.position();
        self.cursor.pop();
        match self.cursor.pop() {
            None => Err(Kind::UnexpectedEnd.at(position)),
            Some('x') => Ok(Escape::Char(self.parse_hex_escape(position)?)),
            Some('0') => Err(Kind::UnsupportedOctalEscape.at(position)),
            Some('1'..='9') => Err(Kind::UnsupportedBackreference.at(position)),
            Some(escape) => {
                Escape::from_char(escape).ok_or_else(|| Kind::UnknownEscape(escape).at(position))
            }
        }
    }

    fn parse_hex_escape(&mut self, position: usize) -> Result<char, ParseError> {
        let code_point = if self.cursor.accept('{') {
            let code_point = self.parse_code_point()?;
            self.expect('}')?;
            code_point
        } else {
            let high = self.expect_hex_digit()?;
            match self.cursor.pop_digit(16) {
                Some(low) => u64::from(high) * 16 + u64::from(low),
                None => u64::from(high),
            }
        };
        u32::try_from(code_point)
            .ok()
            .and_then(char::from_u32)
            .ok_or_else(|| Kind::InvalidCodePoint(code_point).at(position))
    }

    fn parse_code_point(&mut self) -> Result<u64, ParseError> {
        let mut code_point = u64::from(self.expect_hex_digit()?);
        while let Some(digit) = self.cursor.pop_digit(16) {
            code_point = code_point
                .saturating_mul(16)
                .saturating_add(u64::from(digit));
        }
        Ok(code_point)
    }

    fn expect_hex_digit(&mut self) -> Result<u32, ParseError> {
        match self.cursor.pop_digit(16) {
            Some(digit) => Ok(digit),
            None => Err(self.unexpected()),
        }
    }

    fn expect(&mut self, wanted: char) -> Result<(), ParseError> {
        if self.cursor.accept(wanted) {
            return Ok(());
        }
        Err(Kind::Expected {
            wanted,
            found: self.cursor.peek(),
        }
        .at(self.position()))
    }

    fn unexpected(&self) -> ParseError {
        match self.cursor.peek() {
            Some(found) => Kind::UnexpectedCharacter(found).at(self.position()),
            None => Kind::UnexpectedEnd.at(self.position()),
        }
    }

    fn at_sequence_end(&self) -> bool {
        matches!(self.cursor.peek(), None | Some('|' | ')'))
    }

    fn at_range_dash(&self) -> bool {
        self.cursor.peek() == Some('-') && self.cursor.peek_ahead() != Some(']')
    }

    fn position(&self) -> usize {
        self.cursor.position()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(pattern: &str) -> Result<Node, ParseError> {
        RegexParser::new(pattern).parse()
    }

    fn message(pattern: &str) -> String {
        parse(pattern).unwrap_err().to_string()
    }

    fn single(character: char) -> Node {
        Node::Class(CharSet::single(character))
    }

    fn class(set: CharSet) -> Node {
        Node::Class(set)
    }

    fn set_of(characters: &str) -> CharSet {
        characters.chars().fold(CharSet::empty(), |set, character| {
            set.union(&CharSet::single(character))
        })
    }

    fn literals_after(node: Node, text: &str) -> Node {
        text.chars()
            .fold(node, |node, character| node.concat(single(character)))
    }

    fn literals(text: &str) -> Node {
        literals_after(Node::Epsilon, text)
    }

    fn class_of(pattern: &str) -> CharSet {
        match parse(pattern) {
            Ok(Node::Class(set)) => set,
            other => panic!("expected {pattern} to parse to a class, got {other:?}"),
        }
    }

    #[test]
    fn empty_pattern_parses_to_epsilon() {
        assert_eq!(parse(""), Ok(Node::Epsilon));
    }

    #[test]
    fn single_character_parses_to_a_class() {
        assert_eq!(parse("a"), Ok(single('a')));
    }

    #[test]
    fn concatenation_associates_to_the_left() {
        let expected = single('a').concat(single('b')).concat(single('c'));
        assert_eq!(parse("abc"), Ok(expected));
    }

    #[test]
    fn alternation_associates_to_the_left() {
        let expected = single('a').alternate(single('b')).alternate(single('c'));
        assert_eq!(parse("a|b|c"), Ok(expected));
    }

    #[test]
    fn concatenation_binds_tighter_than_alternation() {
        let expected = single('a').concat(single('b')).alternate(single('c'));
        assert_eq!(parse("ab|c"), Ok(expected));
    }

    #[test]
    fn empty_alternation_branch_parses_to_epsilon() {
        let expected = single('a').alternate(Node::Epsilon);
        assert_eq!(parse("a|"), Ok(expected));
    }

    #[test]
    fn group_does_not_appear_in_the_tree() {
        let expected = single('a').concat(single('b')).concat(single('c'));
        assert_eq!(parse("(ab)c"), Ok(expected));
    }

    #[test]
    fn empty_group_parses_to_epsilon() {
        assert_eq!(parse("()"), Ok(Node::Epsilon));
    }

    #[test]
    fn quantifier_applies_to_the_preceding_atom_only() {
        let expected = single('a').concat(single('b').star());
        assert_eq!(parse("ab*"), Ok(expected));
    }

    #[test]
    fn quantifier_applies_to_a_whole_group() {
        let expected = single('a').concat(single('b')).star();
        assert_eq!(parse("(ab)*"), Ok(expected));
    }

    #[test]
    fn plus_wraps_the_preceding_atom() {
        assert_eq!(parse("a+"), Ok(single('a').plus()));
    }

    #[test]
    fn optional_wraps_the_preceding_atom() {
        assert_eq!(parse("a?"), Ok(single('a').optional()));
    }

    #[test]
    fn exact_repetition_parses_to_an_equal_range() {
        let expected = single('a').repeated(Repetitions::Range(3, 3));
        assert_eq!(parse("a{3}"), Ok(expected));
    }

    #[test]
    fn open_ended_repetition_parses_to_at_least() {
        let expected = single('a').repeated(Repetitions::AtLeast(3));
        assert_eq!(parse("a{3,}"), Ok(expected));
    }

    #[test]
    fn bounded_repetition_parses_to_a_range() {
        let expected = single('a').repeated(Repetitions::Range(3, 5));
        assert_eq!(parse("a{3,5}"), Ok(expected));
    }

    #[test]
    fn dot_matches_any_character_except_a_newline() {
        let dot = class_of(".");
        assert!(!dot.contains('\n'));
        assert!(dot.contains('\r'));
        assert!(dot.contains('a'));
        assert!(dot.contains('\u{10FFFF}'));
    }

    #[test]
    fn negated_classes_match_a_newline() {
        assert!(class_of("[^a]").contains('\n'));
        assert!(class_of(r"\D").contains('\n'));
        assert!(class_of(r"\W").contains('\n'));
    }

    #[test]
    fn any_character_including_a_newline_is_written_as_a_class_union() {
        assert_eq!(class_of(r"[\s\S]"), CharSet::any());
    }

    #[test]
    fn class_unions_its_items() {
        let expected = CharSet::single('a')
            .union(&CharSet::single('b'))
            .union(&CharSet::single('c'));
        assert_eq!(parse("[abc]"), Ok(class(expected)));
    }

    #[test]
    fn class_range_covers_the_whole_range() {
        assert_eq!(parse("[a-c]"), Ok(class(CharSet::range('a', 'c'))));
    }

    #[test]
    fn negated_class_matches_everything_else() {
        assert_eq!(parse("[^a]"), Ok(class(CharSet::single('a').negate())));
    }

    #[test]
    fn leading_close_bracket_is_a_literal() {
        assert_eq!(parse("[]]"), Ok(class(CharSet::single(']'))));
    }

    #[test]
    fn negated_class_keeps_a_leading_close_bracket_literal() {
        assert_eq!(parse("[^]]"), Ok(class(CharSet::single(']').negate())));
    }

    #[test]
    fn trailing_dash_is_a_literal() {
        let expected = CharSet::single('a').union(&CharSet::single('-'));
        assert_eq!(parse("[a-]"), Ok(class(expected)));
    }

    #[test]
    fn dash_after_a_class_escape_is_a_literal() {
        let expected = CharSet::digits().union(&set_of("-z"));
        assert_eq!(parse(r"[\d-z]"), Ok(class(expected)));
        let expected = CharSet::word().union(&set_of("-~"));
        assert_eq!(parse(r"[\w-~]"), Ok(class(expected)));
    }

    #[test]
    fn dash_before_a_close_bracket_is_a_literal_after_a_class_escape() {
        let expected = CharSet::digits().union(&CharSet::single('-'));
        assert_eq!(parse(r"[\d-]"), Ok(class(expected)));
    }

    #[test]
    fn escaped_dash_after_a_class_escape_is_a_literal() {
        let expected = CharSet::digits()
            .union(&CharSet::single('-'))
            .union(&CharSet::single('z'));
        assert_eq!(parse(r"[\d\-z]"), Ok(class(expected)));
    }

    #[test]
    fn escape_is_not_parsed_as_backslash_at_the_low_end_of_a_range() {
        assert_eq!(parse(r"[\t-\r]"), Ok(class(CharSet::range('\t', '\r'))));
    }

    #[test]
    fn escape_is_not_parsed_as_backslash_at_the_high_end_of_a_range() {
        let expected = Kind::InvertedRange {
            low: '0',
            high: '\t',
        }
        .at(1);
        assert_eq!(parse(r"[0-\t]"), Err(expected));
    }

    #[test]
    fn class_escapes_expand_to_their_sets() {
        assert_eq!(parse(r"\d"), Ok(class(CharSet::digits())));
        assert_eq!(parse(r"\D"), Ok(class(CharSet::digits().negate())));
        assert_eq!(parse(r"\w"), Ok(class(CharSet::word())));
        assert_eq!(parse(r"\W"), Ok(class(CharSet::word().negate())));
        assert_eq!(parse(r"\s"), Ok(class(CharSet::whitespace())));
        assert_eq!(parse(r"\S"), Ok(class(CharSet::whitespace().negate())));
    }

    #[test]
    fn control_escapes_expand_to_single_characters() {
        assert_eq!(parse(r"\n"), Ok(single('\n')));
        assert_eq!(parse(r"\t"), Ok(single('\t')));
        assert_eq!(parse(r"\r"), Ok(single('\r')));
    }

    #[test]
    fn punctuation_escapes_are_literal() {
        assert_eq!(parse(r"\."), Ok(single('.')));
        assert_eq!(parse(r"\*"), Ok(single('*')));
        assert_eq!(parse(r"\\"), Ok(single('\\')));
        assert_eq!(parse(r"\["), Ok(single('[')));
    }

    #[test]
    fn quantifier_without_an_atom_is_rejected() {
        assert_eq!(parse("*"), Err(Kind::NothingToRepeat('*').at(0)));
        assert_eq!(parse("+"), Err(Kind::NothingToRepeat('+').at(0)));
        assert_eq!(parse("?"), Err(Kind::NothingToRepeat('?').at(0)));
        assert_eq!(parse("a|*"), Err(Kind::NothingToRepeat('*').at(2)));
        assert_eq!(parse("(*)"), Err(Kind::NothingToRepeat('*').at(1)));
    }

    #[test]
    fn repeated_quantifier_is_rejected() {
        assert_eq!(parse("a**"), Err(Kind::RepeatedQuantifier('*').at(2)));
    }

    #[test]
    fn lazy_quantifier_is_rejected() {
        assert_eq!(parse("a*?"), Err(Kind::RepeatedQuantifier('?').at(2)));
    }

    #[test]
    fn inverted_range_is_rejected() {
        let expected = Kind::InvertedRange {
            low: 'z',
            high: 'a',
        }
        .at(1);
        assert_eq!(parse("[z-a]"), Err(expected));
    }

    #[test]
    fn inverted_repetition_bounds_are_rejected() {
        let expected = Kind::InvertedRepetition {
            minimum: 3,
            maximum: 1,
        }
        .at(1);
        assert_eq!(parse("a{3,1}"), Err(expected));
    }

    #[test]
    fn repetition_bound_that_does_not_fit_is_rejected() {
        assert_eq!(
            parse("a{99999999999999999999999}"),
            Err(Kind::RepetitionTooLarge.at(2))
        );
    }

    #[test]
    fn unclosed_class_is_rejected() {
        assert_eq!(parse("[abc"), Err(Kind::UnclosedClass.at(0)));
    }

    #[test]
    fn class_ending_after_a_range_dash_is_rejected() {
        assert_eq!(parse("[a-"), Err(Kind::UnclosedClass.at(0)));
    }

    #[test]
    fn class_matching_nothing_is_rejected() {
        assert_eq!(parse(r"[^\d\D]"), Err(Kind::EmptyClass.at(0)));
    }

    #[test]
    fn unclosed_group_is_rejected() {
        assert_eq!(parse("(ab"), Err(Kind::UnclosedGroup.at(0)));
    }

    #[test]
    fn unopened_group_is_rejected() {
        assert_eq!(parse("a)"), Err(Kind::UnmatchedCloseParenthesis.at(1)));
    }

    #[test]
    fn unknown_escape_is_rejected() {
        assert_eq!(parse(r"\q"), Err(Kind::UnknownEscape('q').at(0)));
    }

    #[test]
    fn escape_at_end_of_pattern_is_rejected() {
        assert_eq!(parse(r"a\"), Err(Kind::UnexpectedEnd.at(1)));
    }

    #[test]
    fn braces_that_do_not_form_a_quantifier_are_literal_text() {
        assert_eq!(parse("a{"), Ok(literals("a{")));
        assert_eq!(parse("a{}"), Ok(literals("a{}")));
        assert_eq!(parse("a{x}"), Ok(literals("a{x}")));
        assert_eq!(parse("a{,3}"), Ok(literals("a{,3}")));
        assert_eq!(parse("a{2x}"), Ok(literals("a{2x}")));
        assert_eq!(parse("a{2,"), Ok(literals("a{2,")));
        assert_eq!(parse("a{2,3"), Ok(literals("a{2,3")));
    }

    #[test]
    fn quantifier_shaped_braces_with_nothing_to_repeat_are_rejected() {
        assert_eq!(parse("{3}"), Err(Kind::NothingToRepeat('{').at(0)));
        assert_eq!(parse("{3,5}"), Err(Kind::NothingToRepeat('{').at(0)));
        assert_eq!(parse("a|{2}"), Err(Kind::NothingToRepeat('{').at(2)));
        assert_eq!(parse("({3})"), Err(Kind::NothingToRepeat('{').at(1)));
    }

    #[test]
    fn braces_that_cannot_be_a_quantifier_are_literal_wherever_they_appear() {
        assert_eq!(parse("{x}"), Ok(literals("{x}")));
        assert_eq!(parse("{,3}"), Ok(literals("{,3}")));
    }

    #[test]
    fn literal_braces_after_a_quantifier_are_not_a_stacked_quantifier() {
        let expected = literals_after(single('a').star(), "{x}");
        assert_eq!(parse("a*{x}"), Ok(expected));
    }

    #[test]
    fn braces_that_do_form_a_quantifier_after_a_quantifier_are_rejected() {
        assert_eq!(parse("a*{3}"), Err(Kind::RepeatedQuantifier('{').at(2)));
    }

    #[test]
    fn class_escape_as_a_range_endpoint_is_rejected() {
        assert_eq!(parse(r"[a-\d]"), Err(Kind::ClassEscapeInRange('d').at(3)));
    }

    #[test]
    fn errors_name_the_offending_character_and_position() {
        assert_eq!(message("*"), "'*' has nothing to repeat at position 0");
        assert_eq!(message("a*+"), "repeated quantifier '+' at position 2");
        assert_eq!(message("(ab"), "unclosed '(' at position 0");
        assert_eq!(message("a)"), "unmatched ')' at position 1");
        assert_eq!(message("[abc"), "unclosed '[' at position 0");
        assert_eq!(message("[z-a]"), "invalid range 'z-a' at position 1");
        assert_eq!(message("a{3,1}"), "invalid repetition {3,1} at position 1");
        assert_eq!(message(r"a\"), "unexpected end of pattern at position 1");
        assert_eq!(message(r"\xZZ"), "unexpected 'Z' at position 2");
        assert_eq!(message(r"\x{41x}"), "expected '}', found 'x' at position 5");
        assert_eq!(
            message(r"\x{1"),
            "expected '}', found end of pattern at position 4"
        );
        assert_eq!(
            message("a{99999999999999999999999}"),
            "repetition count is too large at position 2"
        );
        assert_eq!(
            message(r"[^\d\D]"),
            "character class matches nothing at position 0"
        );
        assert_eq!(
            message(r"[a-\d]"),
            "'\\d' cannot be a range endpoint at position 3"
        );
        assert_eq!(message(r"\q"), "unknown escape '\\q' at position 0");
    }

    #[test]
    fn error_messages_escape_control_characters() {
        assert_eq!(message(r"[0-\t]"), "invalid range '0-\\t' at position 1");
    }

    #[test]
    fn shorthand_classes_match_their_definitions() {
        assert_eq!(class_of(r"\d"), class_of("[0-9]"));
        assert_eq!(class_of(r"\w"), class_of("[0-9A-Za-z_]"));
        assert_eq!(class_of(r"\s"), class_of(r"[ \t\n\v\f\r]"));
        assert_eq!(class_of(r"\D"), class_of("[^0-9]"));
        assert_eq!(class_of(r"\W"), class_of("[^0-9A-Za-z_]"));
        assert_eq!(class_of(r"\S"), class_of(r"[^ \t\n\v\f\r]"));
    }

    #[test]
    fn control_escapes_cover_every_whitespace_character() {
        assert_eq!(parse(r"\a"), Ok(single('\u{07}')));
        assert_eq!(parse(r"\v"), Ok(single('\u{0B}')));
        assert_eq!(parse(r"\f"), Ok(single('\u{0C}')));
    }

    #[test]
    fn two_digit_hex_escapes_name_a_code_point() {
        assert_eq!(parse(r"\x41"), Ok(single('A')));
        assert_eq!(parse(r"\x00"), Ok(single('\0')));
        assert_eq!(parse(r"\x7f"), Ok(single('\u{7F}')));
    }

    #[test]
    fn braced_hex_escapes_name_a_code_point() {
        assert_eq!(parse(r"\x{a}"), Ok(single('\n')));
        assert_eq!(parse(r"\x{1F600}"), Ok(single('\u{1F600}')));
        assert_eq!(parse(r"\x{10FFFF}"), Ok(single('\u{10FFFF}')));
    }

    #[test]
    fn hex_escapes_are_range_endpoints() {
        let expected = CharSet::range('\0', '\u{7F}');
        assert_eq!(parse(r"[\x00-\x{7F}]"), Ok(class(expected)));
    }

    #[test]
    fn hex_escape_outside_the_unicode_range_is_rejected() {
        assert_eq!(
            parse(r"\x{110000}"),
            Err(Kind::InvalidCodePoint(0x0011_0000).at(0))
        );
    }

    #[test]
    fn hex_escape_naming_a_surrogate_is_rejected() {
        assert_eq!(
            parse(r"\x{D800}"),
            Err(Kind::InvalidCodePoint(0xD800).at(0))
        );
    }

    #[test]
    fn incomplete_hex_escape_is_rejected() {
        assert_eq!(parse(r"\x"), Err(Kind::UnexpectedEnd.at(2)));
        assert_eq!(parse(r"\xZZ"), Err(Kind::UnexpectedCharacter('Z').at(2)));
        assert_eq!(parse(r"\x{}"), Err(Kind::UnexpectedCharacter('}').at(3)));
    }

    #[test]
    fn hex_escapes_allow_leading_zeros_and_a_single_digit() {
        assert_eq!(parse(r"\x4"), Ok(single('\u{4}')));
        assert_eq!(parse(r"\x{041}"), Ok(single('A')));
        assert_eq!(parse(r"\x{000000000041}"), Ok(single('A')));
    }

    #[test]
    fn repetition_bound_above_the_pcre_limit_is_rejected() {
        let expected = single('a').repeated(Repetitions::Range(65535, 65535));
        assert_eq!(parse("a{65535}"), Ok(expected));
        assert_eq!(parse("a{65536}"), Err(Kind::RepetitionTooLarge.at(2)));
        assert_eq!(parse("a{0,65536}"), Err(Kind::RepetitionTooLarge.at(4)));
    }

    #[test]
    fn anchors_are_rejected_rather_than_read_as_literals() {
        assert_eq!(parse("^a"), Err(Kind::UnsupportedAnchor('^').at(0)));
        assert_eq!(parse("a$"), Err(Kind::UnsupportedAnchor('$').at(1)));
        assert_eq!(parse("(^)"), Err(Kind::UnsupportedAnchor('^').at(1)));
    }

    #[test]
    fn escaped_anchors_are_literals() {
        assert_eq!(parse(r"\^"), Ok(single('^')));
        assert_eq!(parse(r"\$"), Ok(single('$')));
    }

    #[test]
    fn anchor_characters_inside_a_class_are_literals() {
        let expected = CharSet::single('$').union(&CharSet::single('^'));
        assert_eq!(parse("[$^]"), Ok(class(expected)));
    }

    #[test]
    fn posix_classes_are_rejected_rather_than_read_as_literals() {
        assert_eq!(parse("[[:alpha:]]"), Err(Kind::UnsupportedPosixClass.at(1)));
        assert_eq!(
            parse("a[[:digit:]]b"),
            Err(Kind::UnsupportedPosixClass.at(2))
        );
    }

    #[test]
    fn group_modifiers_are_rejected() {
        assert_eq!(parse("(?:ab)"), Err(Kind::UnsupportedGroup.at(0)));
        assert_eq!(parse("(?i)ab"), Err(Kind::UnsupportedGroup.at(0)));
        assert_eq!(parse("(?=ab)"), Err(Kind::UnsupportedGroup.at(0)));
    }

    #[test]
    fn octal_escapes_are_rejected_rather_than_read_as_a_null_byte() {
        assert_eq!(parse(r"\012"), Err(Kind::UnsupportedOctalEscape.at(0)));
        assert_eq!(parse(r"\0"), Err(Kind::UnsupportedOctalEscape.at(0)));
        assert_eq!(parse(r"a\07"), Err(Kind::UnsupportedOctalEscape.at(1)));
    }

    #[test]
    fn a_null_character_is_written_as_a_hex_escape() {
        assert_eq!(parse(r"\x00"), Ok(single('\0')));
        assert_eq!(parse(r"\x{0}a"), Ok(single('\0').concat(single('a'))));
    }

    #[test]
    fn backreferences_are_rejected() {
        assert_eq!(parse(r"(a)\1"), Err(Kind::UnsupportedBackreference.at(3)));
        assert_eq!(parse(r"\9"), Err(Kind::UnsupportedBackreference.at(0)));
    }

    #[test]
    fn unsupported_features_say_what_is_unsupported() {
        assert_eq!(message("^a"), "anchor '^' is not supported at position 0");
        assert_eq!(
            message("(?:a)"),
            "'(?' groups are not supported at position 0"
        );
        assert_eq!(
            message("[[:alpha:]]"),
            "POSIX character classes are not supported at position 1"
        );
        assert_eq!(
            message(r"\012"),
            "octal escapes are not supported at position 0"
        );
        assert_eq!(
            message(r"\1"),
            "backreferences are not supported at position 0"
        );
        assert_eq!(
            message(r"\x{D800}"),
            "invalid code point U+D800 at position 0"
        );
    }

    #[test]
    fn quantifiers_apply_to_every_kind_of_atom() {
        let dot = CharSet::single('\n').negate();
        assert_eq!(parse(".*"), Ok(class(dot).star()));
        assert_eq!(parse(r"\d+"), Ok(class(CharSet::digits()).plus()));
        assert_eq!(parse("[abc]?"), Ok(class(set_of("abc")).optional()));
        let group = single('a').concat(single('b'));
        let expected = group.repeated(Repetitions::Range(2, 3));
        assert_eq!(parse("(ab){2,3}"), Ok(expected));
    }

    #[test]
    fn alternation_inside_a_group_does_not_escape_it() {
        let expected = single('a').alternate(single('b')).concat(single('c'));
        assert_eq!(parse("(a|b)c"), Ok(expected));
    }

    #[test]
    fn empty_group_disappears_from_a_concatenation() {
        assert_eq!(parse("()a"), Ok(single('a')));
        assert_eq!(parse("a()"), Ok(single('a')));
    }

    #[test]
    fn quantified_empty_group_quantifies_epsilon() {
        assert_eq!(parse("()*"), Ok(Node::Epsilon.star()));
    }

    #[test]
    fn zero_repetition_is_allowed() {
        let expected = single('a').repeated(Repetitions::Range(0, 0));
        assert_eq!(parse("a{0}"), Ok(expected));
    }

    #[test]
    fn positions_count_characters_not_bytes() {
        assert_eq!(parse("é)"), Err(Kind::UnmatchedCloseParenthesis.at(1)));
        assert_eq!(parse("αβ("), Err(Kind::UnclosedGroup.at(2)));
        assert_eq!(parse(r"αβ\q"), Err(Kind::UnknownEscape('q').at(2)));
    }

    #[test]
    fn non_ascii_characters_are_ordinary_literals() {
        assert_eq!(parse("é"), Ok(single('é')));
        assert_eq!(parse("[α-ω]"), Ok(class(CharSet::range('α', 'ω'))));
    }

    #[test]
    fn leading_dash_is_a_literal() {
        assert_eq!(parse("[-a]"), Ok(class(set_of("-a"))));
        assert_eq!(parse("[^-a]"), Ok(class(set_of("-a").negate())));
    }

    #[test]
    fn dash_between_two_ranges_is_a_literal() {
        let expected = CharSet::range('a', 'b').union(&set_of("-c"));
        assert_eq!(parse("[a-b-c]"), Ok(class(expected)));
    }

    #[test]
    fn open_bracket_inside_a_class_is_a_literal() {
        assert_eq!(parse("[[]"), Ok(class(CharSet::single('['))));
        assert_eq!(parse("[[a]"), Ok(class(set_of("[a"))));
    }

    #[test]
    fn metacharacters_inside_a_class_are_literals() {
        assert_eq!(parse(r"[.*+?{|()]"), Ok(class(set_of(".*+?{|()"))));
    }

    #[test]
    fn escaped_close_bracket_inside_a_class_is_a_literal() {
        assert_eq!(parse(r"[\]]"), Ok(class(CharSet::single(']'))));
    }

    #[test]
    fn class_escape_inside_a_class_expands_to_its_set() {
        let expected = CharSet::digits().union(&CharSet::single('x'));
        assert_eq!(parse(r"[\dx]"), Ok(class(expected)));
    }

    #[test]
    fn close_bracket_and_close_brace_outside_their_context_are_literals() {
        assert_eq!(parse("a]"), Ok(literals("a]")));
        assert_eq!(parse("a}"), Ok(literals("a}")));
    }

    #[test]
    fn deeply_nested_groups_are_rejected_instead_of_overflowing_the_stack() {
        let nested = |depth: usize| format!("{}a{}", "(".repeat(depth), ")".repeat(depth));
        assert_eq!(parse(&nested(MAX_NESTING_DEPTH)), Ok(single('a')));
        assert_eq!(
            parse(&nested(MAX_NESTING_DEPTH + 1)),
            Err(Kind::NestingTooDeep(MAX_NESTING_DEPTH).at(MAX_NESTING_DEPTH))
        );
        assert_eq!(
            message(&nested(MAX_NESTING_DEPTH + 1)),
            "groups nest more than 250 deep at position 250"
        );
    }

    #[test]
    fn no_pattern_makes_the_parser_panic_or_misreport_a_position() {
        const ALPHABET: [char; 16] = [
            'a', '1', 'x', '\\', '[', ']', '(', ')', '-', '^', '*', '{', ',', '}', '|', '.',
        ];
        const MAX_LENGTH: u32 = 4;

        let mut pattern = String::new();
        for length in 1..=MAX_LENGTH {
            for mut encoded in 0..ALPHABET.len().pow(length) {
                pattern.clear();
                for _ in 0..length {
                    pattern.push(ALPHABET[encoded % ALPHABET.len()]);
                    encoded /= ALPHABET.len();
                }
                if let Err(error) = parse(&pattern) {
                    assert!(
                        error.position <= pattern.chars().count(),
                        "{pattern:?} reported {error}, past the end of the pattern"
                    );
                }
            }
        }
    }

    #[test]
    fn identifier_pattern() {
        let first = CharSet::range('A', 'Z')
            .union(&CharSet::range('a', 'z'))
            .union(&CharSet::single('_'));
        let rest = first.union(&CharSet::range('0', '9'));
        let expected = class(first).concat(class(rest).star());
        assert_eq!(parse("[A-Za-z_][A-Za-z0-9_]*"), Ok(expected));
    }

    #[test]
    fn floating_point_pattern() {
        let digits = CharSet::range('0', '9');
        let fraction = single('.').concat(class(digits.clone()).plus());
        let exponent = class(set_of("eE"))
            .concat(class(set_of("+-")).optional())
            .concat(class(digits.clone()).plus());
        let expected = class(digits)
            .plus()
            .concat(fraction.optional())
            .concat(exponent.optional());
        assert_eq!(parse(r"[0-9]+(\.[0-9]+)?([eE][+-]?[0-9]+)?"), Ok(expected));
    }

    #[test]
    fn string_literal_pattern() {
        let ordinary = set_of("\"\\").negate();
        let escaped = single('\\').concat(class(CharSet::single('\n').negate()));
        let expected = single('"')
            .concat(class(ordinary).alternate(escaped).star())
            .concat(single('"'));
        assert_eq!(parse(r#""([^"\\]|\\.)*""#), Ok(expected));
    }

    #[test]
    fn integer_literal_pattern() {
        let hexadecimal = CharSet::range('0', '9')
            .union(&CharSet::range('a', 'f'))
            .union(&CharSet::range('A', 'F'));
        let prefixed = single('0')
            .concat(class(set_of("xX")))
            .concat(class(hexadecimal).plus());
        let decimal = class(CharSet::range('0', '9')).plus();
        assert_eq!(
            parse("0[xX][0-9a-fA-F]+|[0-9]+"),
            Ok(prefixed.alternate(decimal))
        );
    }

    #[test]
    fn date_pattern() {
        let digits = || class(CharSet::digits());
        let expected = digits()
            .repeated(Repetitions::Range(4, 4))
            .concat(single('-'))
            .concat(digits().repeated(Repetitions::Range(2, 2)))
            .concat(single('-'))
            .concat(digits().repeated(Repetitions::Range(2, 2)));
        assert_eq!(parse(r"\d{4}-\d{2}-\d{2}"), Ok(expected));
    }
}
