use ratatui::{
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph},
};

use crate::{app::Data, label::Handler as LabelHandler, screen::Handler as ScreenHandler};

use super::{adjust_offset, KeyHandler, PopupOutput, Window};

/// A window that can accept input and attempt to move the cursor to the inputted byte.
///
/// This can be opened by pressing `CNTRLj`.
///
/// The input is either parsed as hexadecimal if it is preceded with "0x", or decimal if not.
#[derive(PartialEq, Eq)]
pub(crate) struct JumpToByte {
    pub(crate) input: String,
}

impl KeyHandler for JumpToByte {
    fn is_focusing(&self, window_type: Window) -> bool {
        window_type == Window::JumpToByte
    }
    fn char(&mut self, _: &mut Data, _: &mut ScreenHandler, _: &mut LabelHandler, c: char) {
        self.input.push(c);
    }
    fn get_user_input(&self) -> PopupOutput {
        PopupOutput::Str(&self.input)
    }
    fn backspace(&mut self, _: &mut Data, _: &mut ScreenHandler, _: &mut LabelHandler) {
        self.input.pop();
    }
    fn enter(&mut self, app: &mut Data, display: &mut ScreenHandler, labels: &mut LabelHandler) {
        let new_offset = if self.input.starts_with("0x") {
            usize::from_str_radix(&self.input[2..], 16)
        } else {
            self.input.parse()
        };
        if let Ok(new_offset) = new_offset {
            if new_offset >= app.contents.len() {
                labels.notification = String::from("Invalid range!");
            } else {
                app.offset = new_offset;
                labels.update_all(&app.contents[app.offset..]);
                adjust_offset(app, display, labels);
            }
        } else {
            labels.notification = format!("Error: {:?}", new_offset.unwrap_err());
        }
    }
    fn dimensions(&self) -> Option<(u16, u16)> {
        Some((50, 3))
    }
    fn widget(&self) -> Paragraph {
        Paragraph::new(Span::styled(&self.input, Style::default().fg(Color::White))).block(
            Block::default()
                .title("Jump to Byte:")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Yellow)),
        )
    }
}

impl JumpToByte {
    pub(crate) fn new() -> Self {
        Self { input: String::new() }
    }
}
