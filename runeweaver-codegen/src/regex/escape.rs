use crate::regex::charset::CharSet;

pub(crate) enum Escape {
    Set(char, CharSet),
    Char(char),
}

impl Escape {
    pub(crate) fn from_char(character: char) -> Option<Self> {
        Some(match character {
            'd' => Self::Set(character, CharSet::digits()),
            'D' => Self::Set(character, CharSet::digits().negate()),
            'w' => Self::Set(character, CharSet::word()),
            'W' => Self::Set(character, CharSet::word().negate()),
            's' => Self::Set(character, CharSet::whitespace()),
            'S' => Self::Set(character, CharSet::whitespace().negate()),
            'n' => Self::Char('\n'),
            't' => Self::Char('\t'),
            'r' => Self::Char('\r'),
            'f' => Self::Char('\u{0C}'),
            'v' => Self::Char('\u{0B}'),
            'a' => Self::Char('\u{07}'),
            punctuation if !punctuation.is_alphanumeric() => Self::Char(punctuation),
            _ => return None,
        })
    }

    pub(crate) fn into_set(self) -> CharSet {
        match self {
            Self::Set(_, set) => set,
            Self::Char(character) => CharSet::single(character),
        }
    }
}
