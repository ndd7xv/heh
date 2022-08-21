use std::{
    any::Any,
    cmp,
    error::Error,
    io::{Seek, SeekFrom, Write},
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::{
    app::{AppData, Application, Nibble},
    label::{LabelHandler, LABEL_TITLES},
    screen::{
        ClickedComponent::{AsciiTable, HexTable, Label, Unclickable},
        ScreenHandler,
    },
};

const DEFAULT_INPUT: &str = "";

/// A trait for objects which handle input.
///
/// Depending on what is currently focused, user input can be handled in different ways. For
/// example, pressing enter should not modify the opened file in any form, but doing so while the
/// "Jump To Byte" popup is focused should attempt to move the cursor to the inputted byte.
pub(crate) trait InputHandler {
    /// Downcasts a dynamic `InputHandler` into a specific one.
    fn as_any(&self) -> &dyn Any;

    /// Checks if the current `InputHandler` is a certain `FocusedWindow`.
    fn is_focusing(&self, window_type: FocusedWindow) -> bool;

    // Methods that handle their respective keypresses.
    fn left(&mut self, _: &mut AppData, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn right(&mut self, _: &mut AppData, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn up(&mut self, _: &mut AppData, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn down(&mut self, _: &mut AppData, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn home(&mut self, _: &mut AppData, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn end(&mut self, _: &mut AppData, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn backspace(&mut self, _: &mut AppData, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn delete(&mut self, _: &mut AppData, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn enter(&mut self, _: &mut AppData, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn char(&mut self, _: &mut AppData, _: &mut ScreenHandler, _: &mut LabelHandler, _: char) {}

    /// Returns user input. Is currently just used to get the contents of popups.
    fn get_user_input(&self) -> &str {
        DEFAULT_INPUT
    }
}

/// An enumeration of all the potential components that could be focused. Used to identify which
/// component is currently focused in the `Application`'s input field.
#[derive(PartialEq)]
pub(crate) enum FocusedWindow {
    Ascii,
    Hex,
    JumpToByte,
}

/// The main windows that allow users to edit HEX and ASCII.
#[derive(PartialEq, Clone, Copy)]
pub(crate) enum Editor {
    Ascii,
    Hex,
}

/// A window that can accept input and attempt to move the cursor to the inputted byte.
///
/// The input is either parsed as hexadecimal if it is preceded with "0x", or decimal if not.
#[derive(PartialEq)]
pub(crate) struct JumpToByte {
    pub(crate) input: String,
}

impl InputHandler for Editor {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn is_focusing(&self, window_type: FocusedWindow) -> bool {
        match self {
            Editor::Ascii => window_type == FocusedWindow::Ascii,
            Editor::Hex => window_type == FocusedWindow::Hex,
        }
    }
    fn left(&mut self, app: &mut AppData, display: &mut ScreenHandler, labels: &mut LabelHandler) {
        match self {
            Editor::Ascii => {
                app.offset = app.offset.saturating_sub(1);
                labels.update_all(&app.contents[app.offset..]);
                adjust_offset(app, display, labels);
            }
            Editor::Hex => {
                if app.nibble == Nibble::Beginning {
                    app.offset = app.offset.saturating_sub(1);
                    labels.update_all(&app.contents[app.offset..]);
                    adjust_offset(app, display, labels);
                }
                app.nibble.toggle();
            }
        }
    }
    fn right(&mut self, app: &mut AppData, display: &mut ScreenHandler, labels: &mut LabelHandler) {
        match self {
            Editor::Ascii => {
                app.offset = cmp::min(app.offset.saturating_add(1), app.contents.len() - 1);
                labels.update_all(&app.contents[app.offset..]);
                adjust_offset(app, display, labels);
            }
            Editor::Hex => {
                if app.nibble == Nibble::End {
                    app.offset = cmp::min(app.offset.saturating_add(1), app.contents.len() - 1);
                    labels.update_all(&app.contents[app.offset..]);
                    adjust_offset(app, display, labels);
                }
                app.nibble.toggle();
            }
        }
    }
    fn up(&mut self, app: &mut AppData, display: &mut ScreenHandler, labels: &mut LabelHandler) {
        if let Some(new_offset) = app.offset.checked_sub(display.comp_layouts.bytes_per_line) {
            app.offset = new_offset;
            labels.update_all(&app.contents[app.offset..]);
            adjust_offset(app, display, labels);
        }
    }
    fn down(&mut self, app: &mut AppData, display: &mut ScreenHandler, labels: &mut LabelHandler) {
        if let Some(new_offset) = app.offset.checked_add(display.comp_layouts.bytes_per_line) {
            if new_offset < app.contents.len() {
                app.offset = new_offset;
                labels.update_all(&app.contents[app.offset..]);
                adjust_offset(app, display, labels);
            }
        }
    }
    fn home(&mut self, app: &mut AppData, display: &mut ScreenHandler, labels: &mut LabelHandler) {
        let bytes_per_line = display.comp_layouts.bytes_per_line;
        app.offset = app.offset / bytes_per_line * bytes_per_line;
        labels.update_all(&app.contents[app.offset..]);
        adjust_offset(app, display, labels);

        if self.is_focusing(FocusedWindow::Hex) {
            app.nibble = Nibble::Beginning;
        }
    }
    fn end(&mut self, app: &mut AppData, display: &mut ScreenHandler, labels: &mut LabelHandler) {
        let bytes_per_line = display.comp_layouts.bytes_per_line;
        app.offset = cmp::min(
            app.offset + (bytes_per_line - 1 - app.offset % bytes_per_line),
            app.contents.len() - 1,
        );
        labels.update_all(&app.contents[app.offset..]);
        adjust_offset(app, display, labels);

        if self.is_focusing(FocusedWindow::Hex) {
            app.nibble = Nibble::End;
        }
    }
    fn backspace(
        &mut self,
        app: &mut AppData,
        display: &mut ScreenHandler,
        labels: &mut LabelHandler,
    ) {
        if app.offset > 0 {
            app.contents.remove(app.offset - 1);
            app.offset = app.offset.saturating_sub(1);
            labels.update_all(&app.contents[app.offset..]);
            adjust_offset(app, display, labels);
        }
    }
    fn delete(
        &mut self,
        app: &mut AppData,
        display: &mut ScreenHandler,
        labels: &mut LabelHandler,
    ) {
        if app.contents.len() > 1 {
            app.contents.remove(app.offset);
            labels.update_all(&app.contents[app.offset..]);
            adjust_offset(app, display, labels);
        }
    }
    fn char(
        &mut self,
        app: &mut AppData,
        display: &mut ScreenHandler,
        labels: &mut LabelHandler,
        c: char,
    ) {
        match *self {
            Editor::Ascii => {
                app.contents[app.offset] = c as u8;
                app.offset = cmp::min(app.offset.saturating_add(1), app.contents.len() - 1);
                labels.update_all(&app.contents[app.offset..]);
                adjust_offset(app, display, labels);
            }
            Editor::Hex => {
                if c.is_ascii_hexdigit() {
                    // This can probably be optimized...
                    match app.nibble {
                        Nibble::Beginning => {
                            let mut src = c.to_string();
                            src.push(
                                format!("{:02X}", app.contents[app.offset]).chars().last().unwrap(),
                            );
                            let changed = u8::from_str_radix(src.as_str(), 16).unwrap();
                            app.contents[app.offset] = changed;
                        }
                        Nibble::End => {
                            let mut src = format!("{:02X}", app.contents[app.offset])
                                .chars()
                                .take(1)
                                .collect::<String>();
                            src.push(c);
                            let changed = u8::from_str_radix(src.as_str(), 16).unwrap();
                            app.contents[app.offset] = changed;

                            // Move to the next byte
                            app.offset =
                                cmp::min(app.offset.saturating_add(1), app.contents.len() - 1);
                            labels.update_all(&app.contents[app.offset..]);
                            adjust_offset(app, display, labels);
                        }
                    }
                    app.nibble.toggle();
                } else {
                    labels.notification = format!("Invalid Hex: {c}");
                }
            }
        }
    }
}

impl InputHandler for JumpToByte {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn is_focusing(&self, window_type: FocusedWindow) -> bool {
        window_type == FocusedWindow::JumpToByte
    }
    fn char(&mut self, _: &mut AppData, _: &mut ScreenHandler, _: &mut LabelHandler, c: char) {
        self.input.push(c);
    }
    fn get_user_input(&self) -> &str {
        &self.input
    }
    fn backspace(&mut self, _: &mut AppData, _: &mut ScreenHandler, _: &mut LabelHandler) {
        self.input.pop();
    }
    fn enter(&mut self, app: &mut AppData, display: &mut ScreenHandler, labels: &mut LabelHandler) {
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
}

impl JumpToByte {
    pub(crate) fn new() -> Self {
        Self { input: String::new() }
    }
}

/// Moves the starting address of the editor viewports (Hex and ASCII) to include the cursor.
///
/// If the cursor's location is before the viewports start, the viewports will move so that the
/// cursor is included in the first row.
///
/// If the cursor's location is past the end of the viewports, the vierports will move so that
/// the cursor is included in the final row.
fn adjust_offset(app: &mut AppData, display: &mut ScreenHandler, labels: &mut LabelHandler) {
    let line_adjustment = if app.offset <= app.start_address {
        app.start_address - app.offset + display.comp_layouts.bytes_per_line - 1
    } else {
        app.offset - app.start_address
    } / display.comp_layouts.bytes_per_line;

    let bytes_per_screen =
        display.comp_layouts.bytes_per_line * display.comp_layouts.lines_per_screen;

    if app.offset < app.start_address {
        app.start_address =
            app.start_address.saturating_sub(display.comp_layouts.bytes_per_line * line_adjustment);
    } else if app.offset >= app.start_address + (bytes_per_screen)
        && app.start_address + display.comp_layouts.bytes_per_line < app.contents.len()
    {
        app.start_address = app.start_address.saturating_add(
            display.comp_layouts.bytes_per_line
                * (line_adjustment + 1 - display.comp_layouts.lines_per_screen),
        );
    }

    labels.offset = format!("{:#X}", app.offset);
}

pub(crate) fn handle_key_input(
    app: &mut Application,
    key: KeyEvent,
) -> Result<bool, Box<dyn Error>> {
    match key.code {
        KeyCode::Left => {
            app.input_handler.left(&mut app.data, &mut app.display, &mut app.labels);
        }
        KeyCode::Right => {
            app.input_handler.right(&mut app.data, &mut app.display, &mut app.labels);
        }
        KeyCode::Up => {
            app.input_handler.up(&mut app.data, &mut app.display, &mut app.labels);
        }
        KeyCode::Down => {
            app.input_handler.down(&mut app.data, &mut app.display, &mut app.labels);
        }

        KeyCode::Home => {
            app.input_handler.home(&mut app.data, &mut app.display, &mut app.labels);
        }
        KeyCode::End => {
            app.input_handler.end(&mut app.data, &mut app.display, &mut app.labels);
        }

        KeyCode::Backspace => {
            app.input_handler.backspace(&mut app.data, &mut app.display, &mut app.labels);
        }
        KeyCode::Delete => {
            app.input_handler.delete(&mut app.data, &mut app.display, &mut app.labels);
        }

        KeyCode::Enter => {
            app.input_handler.enter(&mut app.data, &mut app.display, &mut app.labels);
            app.input_handler = Box::from(app.last_input_handler);
        }

        KeyCode::Char(char) => {
            // Because CNTRLq is the signal to quit, we propogate the message
            // if this handling method returns false
            return handle_character_input(app, char, key.modifiers);
        }
        _ => {}
    }
    Ok(true)
}
pub(crate) fn handle_character_input(
    app: &mut Application,
    char: char,
    modifiers: KeyModifiers,
) -> Result<bool, Box<dyn Error>> {
    if modifiers == KeyModifiers::CONTROL {
        match char {
            'j' => {
                if app.input_handler.is_focusing(FocusedWindow::JumpToByte) {
                    app.input_handler = Box::from(app.last_input_handler);
                } else {
                    app.last_input_handler = *app
                        .input_handler
                        .as_any()
                        .downcast_ref()
                        .expect("The current window wasn't an editor");
                    app.input_handler = Box::from(JumpToByte::new());
                }
            }
            'q' => return Ok(false),
            's' => {
                app.data.file.seek(SeekFrom::Start(0))?;
                app.data.file.write_all(&app.data.contents)?;
                app.data.file.set_len(app.data.contents.len() as u64)?;
                app.labels.notification = String::from("Saved!");
            }
            _ => {}
        }
    } else if modifiers == KeyModifiers::ALT {
        match char {
            '=' => {
                app.labels.update_stream_length(cmp::min(app.labels.get_stream_length() + 1, 64));
                app.labels.update_streams(&app.data.contents[app.data.offset..]);
            }
            '-' => {
                app.labels.update_stream_length(cmp::max(
                    app.labels.get_stream_length().saturating_sub(1),
                    0,
                ));
                app.labels.update_streams(&app.data.contents[app.data.offset..]);
            }
            _ => {}
        }
    } else if modifiers | KeyModifiers::NONE | KeyModifiers::SHIFT
        == KeyModifiers::NONE | KeyModifiers::SHIFT
    {
        app.input_handler.char(&mut app.data, &mut app.display, &mut app.labels, char);
    }
    Ok(true)
}
pub(crate) fn handle_mouse_input(app: &mut Application, mouse: MouseEvent) {
    let component =
        app.display.identify_clicked_component(mouse.row, mouse.column, app.input_handler.as_ref());
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            app.data.last_click = component;
            match app.data.last_click {
                HexTable => {
                    app.input_handler = Box::from(Editor::Hex);
                }
                AsciiTable => {
                    app.input_handler = Box::from(Editor::Ascii);
                }
                Label(_) | Unclickable => {}
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            match component {
                HexTable | AsciiTable | Unclickable => {}
                Label(i) => {
                    if app.data.last_click == component {
                        // Put string into clipboard
                        if let Some(clipboard) = app.data.clipboard.as_mut() {
                            clipboard.set_text(app.labels[LABEL_TITLES[i]].clone()).unwrap();
                            app.labels.notification = format!("{} copied!", LABEL_TITLES[i]);
                        } else {
                            app.labels.notification = String::from("Can't find clipboard!");
                        }
                    }
                }
            }
        }
        MouseEventKind::ScrollUp => {
            let bytes_per_line = app.display.comp_layouts.bytes_per_line;

            // Scroll up a line in the viewport without changing cursor.
            app.data.start_address = app.data.start_address.saturating_sub(bytes_per_line);
        }
        MouseEventKind::ScrollDown => {
            let bytes_per_line = app.display.comp_layouts.bytes_per_line;
            let lines_per_screen = app.display.comp_layouts.lines_per_screen;

            let content_lines = app.data.contents.len() / bytes_per_line + 1;
            let start_row = app.data.start_address / bytes_per_line;

            // Scroll down a line in the viewport without changing cursor.
            // Until the viewport contains the last page of content.
            if start_row + lines_per_screen < content_lines {
                app.data.start_address = app.data.start_address.saturating_add(bytes_per_line);
            }
        }
        _ => {}
    }
}
