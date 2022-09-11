//! Handles user input.
//!
//! This is where mouse actions are programmed. It's also a wrapper around calls to a dynamic
//! [`KeyHandler`](crate::windows::KeyHandler), which handles keyboared input.

use std::{
    cmp,
    error::Error,
    io::{Seek, SeekFrom, Write},
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::{
    app::{Application, Nibble},
    label::LABEL_TITLES,
    windows::{PopupOutput, Window},
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
            app.focus_editor();
        }

        KeyCode::Enter => {
            if app.key_handler.is_focusing(Window::UnsavedChanges)
                && app.key_handler.get_user_input() == PopupOutput::Boolean(true)
            {
                return Ok(false);
            }
            app.key_handler.enter(&mut app.data, &mut app.display, &mut app.labels);
            app.focus_editor();
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

/// Handles a character key press. While used predominantly to edit a file, it also checks for
/// any shortcut commands being used.
pub(crate) fn handle_character_input(
    app: &mut Application,
    char: char,
    modifiers: KeyModifiers,
) -> Result<bool, Box<dyn Error>> {
    if modifiers == KeyModifiers::CONTROL {
        match char {
            'j' => {
                if app.key_handler.is_focusing(Window::JumpToByte) {
                    app.focus_editor();
                } else {
                    app.set_focused_window(Window::JumpToByte);
                }
            }
            'q' => {
                if !app.key_handler.is_focusing(Window::UnsavedChanges) {
                    if app.hash_contents() == app.data.hashed_contents {
                        return Ok(false);
                    }
                    app.set_focused_window(Window::UnsavedChanges);
                }
            }
            's' => {
                app.data.file.seek(SeekFrom::Start(0))?;
                app.data.file.write_all(&app.data.contents)?;
                app.data.file.set_len(app.data.contents.len() as u64)?;

                app.data.hashed_contents = app.hash_contents();

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
        let is_hex = app.key_handler.is_focusing(Window::Hex);

        match char {
            'q' if is_hex => {
                if !app.key_handler.is_focusing(Window::UnsavedChanges) {
                    if app.hash_contents() == app.data.hashed_contents {
                        return Ok(false);
                    }
                    app.set_focused_window(Window::UnsavedChanges);
                }
            }
            'h' if is_hex => {
                app.key_handler.left(&mut app.data, &mut app.display, &mut app.labels);
            }
            'l' if is_hex => {
                app.key_handler.right(&mut app.data, &mut app.display, &mut app.labels);
            }
            'k' if is_hex => {
                app.key_handler.up(&mut app.data, &mut app.display, &mut app.labels);
            }
            'j' if is_hex => {
                app.key_handler.down(&mut app.data, &mut app.display, &mut app.labels);
            }
            '^' if is_hex => {
                app.key_handler.home(&mut app.data, &mut app.display, &mut app.labels);
            }
            '$' if is_hex => {
                app.key_handler.end(&mut app.data, &mut app.display, &mut app.labels);
            }
            _ => {
                app.key_handler.char(&mut app.data, &mut app.display, &mut app.labels, char);
            }
        }
    }
    Ok(true)
}

/// Handles the mouse input, which consists of things like scrolling and focusing components
/// based on a left and right click.
pub(crate) fn handle_mouse_input(app: &mut Application, mouse: MouseEvent) {
    let component =
        app.display.identify_clicked_component(mouse.row, mouse.column, app.key_handler.as_ref());
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            app.data.last_click = component;
            match app.data.last_click {
                Window::Hex => {
                    app.set_focused_window(Window::Hex);
                    let hex = &app.display.comp_layouts.hex;
                    // Identify the byte that was clicked on based on the relative position.
                    let (mut rel_x, mut rel_y) = (mouse.column - hex.x, mouse.row - hex.y);
                    // Account for the border of the viewport.
                    if rel_y > 0 && rel_x > 0 && hex.height - 1 > rel_y && hex.width - 1 > rel_x {
                        (rel_x, rel_y) = (rel_x, rel_y - 1);
                        if rel_x % 3 != 0 {
                            let new_pos = app.data.start_address
                                + (rel_y as usize * app.display.comp_layouts.bytes_per_line)
                                + (rel_x as usize / 3);
                            if new_pos < app.data.contents.len() {
                                app.data.offset = new_pos;
                                if rel_x % 3 == 1 {
                                    app.data.nibble = Nibble::Beginning;
                                } else if rel_x % 3 == 2 {
                                    app.data.nibble = Nibble::End;
                                }
                            }
                        }
                    }
                }
                Window::Ascii => {
                    app.set_focused_window(Window::Ascii);
                    let ascii = &app.display.comp_layouts.ascii;
                    let (mut rel_x, mut rel_y) = (mouse.column - ascii.x, mouse.row - ascii.y);
                    if rel_y > 0 && rel_x > 0 && ascii.height - 1 > rel_y && ascii.width - 1 > rel_x
                    {
                        (rel_x, rel_y) = (rel_x - 1, rel_y - 1);
                        let new_pos = app.data.start_address
                            + (rel_y as usize * app.display.comp_layouts.bytes_per_line)
                            + (rel_x as usize);
                        if new_pos < app.data.contents.len() {
                            app.data.offset = new_pos;
                        }
                    }
                }
                Window::Label(_)
                | Window::Unhandled
                | Window::JumpToByte
                | Window::UnsavedChanges => {}
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            match component {
                Window::Label(i) => {
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
                Window::Hex
                | Window::Ascii
                | Window::Unhandled
                | Window::JumpToByte
                | Window::UnsavedChanges => {}
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
