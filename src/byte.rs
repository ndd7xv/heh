//! Determines how a byte is colored and displayed.

use tui::style::Color;
pub(crate) enum ByteCategory {
    Null,
    AsciiPrintable,
    AsciiWhitespace,
    AsciiOther,
    NonAscii,
}

const COLOR_NULL: Color = Color::DarkGray;
const COLOR_ASCII_PRINTABLE: Color = Color::Cyan;
const COLOR_ASCII_WHITESPACE: Color = Color::Green;
const COLOR_ASCII_OTHER: Color = Color::Magenta;
const COLOR_NONASCII: Color = Color::Yellow;

pub(crate) fn category(byte: u8) -> ByteCategory {
    if byte == 0x00 {
        ByteCategory::Null
    } else if byte.is_ascii_graphic() {
        ByteCategory::AsciiPrintable
    } else if byte.is_ascii_whitespace() {
        ByteCategory::AsciiWhitespace
    } else if byte.is_ascii() {
        ByteCategory::AsciiOther
    } else {
        ByteCategory::NonAscii
    }
}

pub(crate) fn as_str(byte: u8) -> String {
    match category(byte) {
        ByteCategory::Null => "0".to_string(),
        ByteCategory::AsciiPrintable => (byte as char).to_string(),
        ByteCategory::AsciiWhitespace if byte == 0x20 => " ".to_string(),
        ByteCategory::AsciiWhitespace => "_".to_string(),
        ByteCategory::AsciiOther => "•".to_string(),
        ByteCategory::NonAscii => "×".to_string(),
    }
}

pub(crate) fn get_color(byte: u8) -> &'static Color {
    match category(byte) {
        ByteCategory::Null => &COLOR_NULL,
        ByteCategory::AsciiPrintable => &COLOR_ASCII_PRINTABLE,
        ByteCategory::AsciiWhitespace => &COLOR_ASCII_WHITESPACE,
        ByteCategory::AsciiOther => &COLOR_ASCII_OTHER,
        ByteCategory::NonAscii => &COLOR_NONASCII,
    }
}
