//! Handles user input.
//!
//! This is where mouse actions are programmed. It's also a wrapper around calls to a dynamic
//! [`KeyHandler`](crate::windows::KeyHandler), which handles keyboared input.

use std::{
    cmp,
    collections::hash_map::DefaultHasher,
    error::Error,
    hash::Hasher,
    io::{Seek, SeekFrom, Write},
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::{
    app::Application,
    label::LABEL_TITLES,
    screen::ClickedComponent::{AsciiTable, HexTable, Label, Unclickable},
    windows::{
        editor::Editor, jump_to_byte::JumpToByte, unsaved_changes::UnsavedChanges, FocusedWindow,
        PopupOutput,
    },
};

/// Wrapper function that calls the corresponding [`KeyHandler`](crate::windows::KeyHandler) methods of
/// [the application's `key_handler`.](Application::key_handler)
pub(crate) fn handle_key_input(
    app: &mut Application,
    key: KeyEvent,
) -> Result<bool, Box<dyn Error>> {
    match key.code {
        // Arrow key input
        KeyCode::Left => {
            app.key_handler.left(&mut app.data, &mut app.display, &mut app.labels);
        }
        KeyCode::Right => {
            app.key_handler.right(&mut app.data, &mut app.display, &mut app.labels);
        }
        KeyCode::Up => {
            app.key_handler.up(&mut app.data, &mut app.display, &mut app.labels);
        }
        KeyCode::Down => {
            app.key_handler.down(&mut app.data, &mut app.display, &mut app.labels);
        }

        // Cursor shortcuts
        KeyCode::Home => {
            app.key_handler.home(&mut app.data, &mut app.display, &mut app.labels);
        }
        KeyCode::End => {
            app.key_handler.end(&mut app.data, &mut app.display, &mut app.labels);
        }

        // Removals
        KeyCode::Backspace => {
            app.key_handler.backspace(&mut app.data, &mut app.display, &mut app.labels);
        }
        KeyCode::Delete => {
            app.key_handler.delete(&mut app.data, &mut app.display, &mut app.labels);
        }
        KeyCode::Esc => {
            app.key_handler = Box::from(app.data.editor);
        }

        KeyCode::Enter => {
            if app.key_handler.is_focusing(FocusedWindow::UnsavedChanges)
                && app.key_handler.get_user_input() == PopupOutput::Boolean(true)
            {
                return Ok(false);
            }
            app.key_handler.enter(&mut app.data, &mut app.display, &mut app.labels);
            app.key_handler = Box::from(app.data.editor);
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
                if app.key_handler.is_focusing(FocusedWindow::JumpToByte) {
                    app.key_handler = Box::from(app.data.editor);
                } else {
                    app.key_handler = Box::from(JumpToByte::new());
                }
            }
            'q' => {
                if !app.key_handler.is_focusing(FocusedWindow::UnsavedChanges) {
                    let mut hasher = DefaultHasher::new();
                    hasher.write(&app.data.contents);
                    if hasher.finish() == app.data.hashed_contents {
                        return Ok(false);
                    }
                    app.key_handler = Box::from(UnsavedChanges::new());
                }
            }
            's' => {
                app.data.file.seek(SeekFrom::Start(0))?;
                app.data.file.write_all(&app.data.contents)?;
                app.data.file.set_len(app.data.contents.len() as u64)?;

                let mut hasher = DefaultHasher::new();
                hasher.write(&app.data.contents);
                app.data.hashed_contents = hasher.finish();

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
        app.key_handler.char(&mut app.data, &mut app.display, &mut app.labels, char);
    }
    Ok(true)
}
pub(crate) fn handle_mouse_input(app: &mut Application, mouse: MouseEvent) {
    let component =
        app.display.identify_clicked_component(mouse.row, mouse.column, app.key_handler.as_ref());
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            app.data.last_click = component;
            match app.data.last_click {
                HexTable => {
                    app.key_handler = Box::from(Editor::Hex);
                    app.data.editor = Editor::Hex;
                }
                AsciiTable => {
                    app.key_handler = Box::from(Editor::Ascii);
                    app.data.editor = Editor::Ascii;
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
