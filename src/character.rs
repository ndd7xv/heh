use std::fmt::{Debug, Display, Formatter};

use ratatui::style::Color;

pub(crate) const CHARACTER_NULL: char = '0';
pub(crate) const CHARACTER_WHITESPACE: char = '_';
pub(crate) const CHARACTER_CONTROL: char = '⍾';
pub(crate) const CHARACTER_FILL: char = '•';
pub(crate) const CHARACTER_UNKNOWN: char = '�';

const COLOR_NULL: Color = Color::DarkGray;
const COLOR_ASCII: Color = Color::Cyan;
const COLOR_UNICODE: Color = Color::LightCyan;
const COLOR_WHITESPACE: Color = Color::Green;
const COLOR_CONTROL: Color = Color::Magenta;
const COLOR_FILL: Color = Color::LightCyan;
const COLOR_UNKNOWN: Color = Color::Yellow;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Type {
    Ascii,
    Unicode(usize),
    Unknown,
}

impl Type {
    pub(crate) fn size(&self) -> usize {
        match self {
            Type::Ascii | Type::Unknown => 1,
            Type::Unicode(size) => *size,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Category {
    Null,
    Ascii,
    Unicode,
    Whitespace,
    Control,
    Fill,
    Unknown,
}

impl From<&char> for Category {
    fn from(character: &char) -> Self {
        if character == &'\0' {
            Category::Null
        } else if character.is_whitespace() {
            Category::Whitespace
        } else if character.is_control() {
            Category::Control
        } else if character.is_ascii() {
            Category::Ascii
        } else {
            Category::Unicode
        }
    }
}

impl Category {
    pub(crate) fn escape(&self, character: char) -> char {
        match self {
            Category::Null => CHARACTER_NULL,
            Category::Ascii | Category::Unicode => character,
            Category::Whitespace if character == ' ' => ' ',
            Category::Whitespace => CHARACTER_WHITESPACE,
            Category::Control => CHARACTER_CONTROL,
            Category::Fill => CHARACTER_FILL,
            Category::Unknown => CHARACTER_UNKNOWN,
        }
    }

    pub(crate) fn color(&self) -> &'static Color {
        match self {
            Category::Null => &COLOR_NULL,
            Category::Ascii => &COLOR_ASCII,
            Category::Unicode => &COLOR_UNICODE,
            Category::Whitespace => &COLOR_WHITESPACE,
            Category::Control => &COLOR_CONTROL,
            Category::Fill => &COLOR_FILL,
            Category::Unknown => &COLOR_UNKNOWN,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RichChar {
    character: char,
    category: Category,
}

impl RichChar {
    pub(crate) fn new(character: char, category: Category) -> Self {
        Self { character, category }
    }

    pub(crate) fn escape(&self) -> char {
        self.category.escape(self.character)
    }

    pub(crate) fn color(&self) -> &'static Color {
        self.category.color()
    }
}

impl Display for RichChar {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.character, f)
    }
}

impl From<RichChar> for char {
    fn from(rich_char: RichChar) -> Self {
        rich_char.character
    }
}

impl From<&RichChar> for char {
    fn from(rich_char: &RichChar) -> Self {
        rich_char.character
    }
}

impl From<RichChar> for String {
    fn from(rich_char: RichChar) -> Self {
        rich_char.character.to_string()
    }
}

impl From<&RichChar> for String {
    fn from(rich_char: &RichChar) -> Self {
        rich_char.character.to_string()
    }
}
