use tui::{
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph},
};

use crate::{app::AppData, label::LabelHandler, screen::ScreenHandler};

use super::{adjust_offset, KeyHandler, PopupOutput, Window};

/// A window that accepts either a hexadecimal or an ASCII sequence and moves cursor to the next
/// occurrence of this sequence
///
/// This can be opened by pressing `CNTRLf`.
///
/// Each symbol group is either parsed as hexadecimal if it is preceded with "0x", or decimal if
/// not.
///
/// Replace ASCII "0x", with "0x30x", (0x30 is hexadecimal for ascii 0) e.g. to search for "0xFF"
/// in ASCII, search for "0x30xFF" instead.
#[derive(PartialEq, Eq)]
pub(crate) struct Search {
    pub(crate) input: String,
}

impl Search {
    pub(crate) fn new() -> Self {
        Self { input: String::new() }
    }
}

impl KeyHandler for Search {
    fn is_focusing(&self, window_type: super::Window) -> bool {
        window_type == Window::Search
    }
    fn char(&mut self, _: &mut AppData, _: &mut ScreenHandler, _: &mut LabelHandler, c: char) {
        self.input.push(c);
    }
    fn get_user_input(&self) -> PopupOutput {
        PopupOutput::Str(&self.input)
    }
    fn backspace(&mut self, _: &mut AppData, _: &mut ScreenHandler, _: &mut LabelHandler) {
        self.input.pop();
    }
    fn enter(&mut self, app: &mut AppData, display: &mut ScreenHandler, labels: &mut LabelHandler) {
        let byte_sequence_to_search = match parse_input(&self.input) {
            Ok(s) => s,
            Err(e) => {
                labels.notification = format!("Error: {e:?}");
                return;
            }
        };
        if byte_sequence_to_search.is_empty() {
            labels.notification = "Empty search query".into();
            return;
        }
        let needle_offset_delta = if let Some(p) = app.contents[app.offset..]
            .windows(byte_sequence_to_search.len())
            .position(|w| w == byte_sequence_to_search)
        {
            p
        } else {
            labels.notification = "Query not found".into();
            return;
        };

        app.offset += needle_offset_delta;
        labels.update_all(&app.contents[app.offset..]);
        adjust_offset(app, display, labels);
    }
    fn dimensions(&self) -> Option<(u16, u16)> {
        Some((50, 3))
    }
    fn widget(&self) -> Paragraph {
        Paragraph::new(Span::styled(&self.input, Style::default().fg(Color::White))).block(
            Block::default()
                .title("Search:")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Yellow)),
        )
    }
}

fn parse_input(input: &str) -> Result<Vec<u8>, String> {
    if !input.is_ascii() {
        return Err("Expect ASCII search string".into());
    }
    let mut input = input.as_bytes();
    let mut result = vec![];

    loop {
        match input {
            [] => return Ok(result),
            // [0x30, 0x78] are hex for '0x'
            [0x30, 0x78, h1, h2, ..] => {
                let bytes = [*h1, *h2];
                let hex = std::str::from_utf8(&bytes).expect("input string to contain ascii");
                let byte = u8::from_str_radix(hex, 16)
                    .map_err(|e| format!("Parsing {:?} {:?}: {e}", *h1 as char, *h2 as char))?;
                result.push(byte);
                input = &input[4..];
            }
            [ascii_symbol, ..] => {
                result.push(*ascii_symbol);
                input = &input[1..];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_input;

    #[test]
    fn test_parse() {
        assert_eq!(parse_input(""), Ok(vec![]));
        assert_eq!(parse_input("0x"), Ok(vec![0x30, 0x78]));
        assert_eq!(parse_input("asdf"), Ok(b"asdf".to_vec()));
        assert_eq!(parse_input("0x30"), Ok(b"0".to_vec()));
        assert_eq!(parse_input("0x30x"), Ok(b"0x".to_vec()));
        assert_eq!(parse_input("abc0x64e"), Ok(b"abcde".to_vec()));
    }
}
