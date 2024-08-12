//! Handles user input.
//!
//! This is where mouse actions are programmed. It's also a wrapper around calls to a dynamic
//! [`KeyHandler`](crate::windows::KeyHandler), which handles keyboared input.

use std::{
    cmp,
    error::Error,
    io::{Seek, Write},
};

use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use crate::{
    app::{Action, Application, Nibble},
    label::LABEL_TITLES,
    windows::{
        adjust_offset,
        search::{perform_search, SearchDirection},
        PopupOutput, Window,
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
        KeyCode::PageUp => {
            app.key_handler.page_up(&mut app.data, &mut app.display, &mut app.labels);
        }
        KeyCode::PageDown => {
            app.key_handler.page_down(&mut app.data, &mut app.display, &mut app.labels);
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
        return handle_control_options(char, app);
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
                    if !app.data.dirty {
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
            '/' if is_hex => {
                app.set_focused_window(Window::Search);
            }
            _ => {
                app.key_handler.char(&mut app.data, &mut app.display, &mut app.labels, char);
            }
        }
    }
    Ok(true)
}

fn handle_control_options(char: char, app: &mut Application) -> Result<bool, Box<dyn Error>> {
    match char {
        'j' => {
            if app.key_handler.is_focusing(Window::JumpToByte) {
                app.focus_editor();
            } else {
                app.set_focused_window(Window::JumpToByte);
            }
        }
        'f' => {
            if app.key_handler.is_focusing(Window::Search) {
                app.focus_editor();
            } else {
                app.set_focused_window(Window::Search);
            }
        }
        'q' => {
            if !app.key_handler.is_focusing(Window::UnsavedChanges) {
                if !app.data.dirty {
                    return Ok(false);
                }
                app.set_focused_window(Window::UnsavedChanges);
            }
        }
        's' => {
            app.data.contents.block();
            app.data.file.rewind()?;
            app.data.file.write_all(&app.data.contents)?;
            app.data.file.set_len(app.data.contents.len() as u64)?;

            app.data.dirty = false;

            app.labels.notification = String::from("Saved!");
        }
        'e' => {
            app.labels.switch_endianness();
            app.labels.update_all(&app.data.contents[app.data.offset..]);

            app.labels.notification = app.labels.endianness.to_string();
        }
        'd' => {
            app.key_handler.page_down(&mut app.data, &mut app.display, &mut app.labels);
        }
        'u' => {
            app.key_handler.page_up(&mut app.data, &mut app.display, &mut app.labels);
        }
        'n' => {
            perform_search(
                &mut app.data,
                &mut app.display,
                &mut app.labels,
                &SearchDirection::Forward,
            );
        }
        'p' => {
            perform_search(
                &mut app.data,
                &mut app.display,
                &mut app.labels,
                &SearchDirection::Backward,
            );
        }
        'z' => {
            if let Some(action) = app.data.actions.pop() {
                match action {
                    Action::CharacterInput(offset, byte, nibble) => {
                        app.data.offset = offset;
                        if let Some(nibble) = nibble {
                            app.data.nibble = nibble;
                        }
                        app.data.contents[offset] = byte;
                    }
                    Action::Delete(offset, byte) => {
                        app.data.contents.insert(offset, byte);
                        app.data.offset = offset;
                    }
                }
            }
        }
        _ => {}
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
                Window::Ascii => {
                    if let Some((cursor_pos, _)) = handle_editor_click(Window::Ascii, app, mouse) {
                        app.data.offset = cursor_pos;
                    }
                }
                Window::Hex => {
                    if let Some((cursor_pos, nibble)) = handle_editor_click(Window::Hex, app, mouse)
                    {
                        app.data.offset = cursor_pos;
                        app.data.nibble = nibble.expect("Clicking on Hex should return a nibble!");
                    }
                }
                Window::Label(_)
                | Window::Unhandled
                | Window::JumpToByte
                | Window::Search
                | Window::UnsavedChanges => {}
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if app.data.drag_enabled {
                match app.data.last_click {
                    Window::Ascii => {
                        if let Some((cursor_pos, _)) = handle_editor_drag(Window::Ascii, app, mouse)
                        {
                            if app.data.last_drag.is_none() {
                                app.data.last_drag = Some(app.data.offset);
                            }
                            app.data.offset = cursor_pos;
                            app.labels.update_all(&app.data.contents[app.data.offset..]);
                            adjust_offset(&mut app.data, &mut app.display, &mut app.labels);
                        }
                    }
                    Window::Hex => {
                        if let Some((cursor_pos, nibble)) =
                            handle_editor_drag(Window::Hex, app, mouse)
                        {
                            if app.data.last_drag.is_none() {
                                app.data.last_drag = Some(app.data.offset);
                                app.data.drag_nibble = Some(app.data.nibble);
                            }
                            app.data.offset = cursor_pos;
                            app.data.nibble = nibble.unwrap();
                            app.labels.update_all(&app.data.contents[app.data.offset..]);
                            adjust_offset(&mut app.data, &mut app.display, &mut app.labels);
                        }
                    }
                    Window::Label(_)
                    | Window::Unhandled
                    | Window::JumpToByte
                    | Window::Search
                    | Window::UnsavedChanges => {}
                }
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
                | Window::Search
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

/// A wrapper around [`handle_editor_cursor_action`] that does the additional things that come with a click.
#[allow(clippy::cast_possible_truncation)]
fn handle_editor_click(
    window: Window,
    app: &mut Application,
    mut mouse: MouseEvent,
) -> Option<(usize, Option<Nibble>)> {
    app.set_focused_window(window);

    let (editor, word_size) = match window {
        Window::Ascii => (&app.display.comp_layouts.ascii, 1),
        Window::Hex => (&app.display.comp_layouts.hex, 3),
        _ => {
            panic!("Trying to move cursor on unhandled window!")
        }
    };

    // In the hex editor, a cursor click in between two bytes will select the first nibble of the
    // latter one. In the case that we're at the end of the row, this is just a tweak so that the
    // cursor is selected as the last nibble of the first byte.
    let end_of_row = editor.x + app.display.comp_layouts.bytes_per_line as u16 * word_size;
    if mouse.column == end_of_row {
        mouse.column = end_of_row;
    }
    let res = handle_editor_cursor_action(window, app, mouse);
    if res.is_some() {
        // Reset the dragged highlighting.
        app.data.last_drag = None;
        app.data.drag_nibble = None;
        app.data.drag_enabled = true;
    } else {
        app.data.drag_enabled = false;
    }
    res
}

/// A wrapper around [`handle_editor_cursor_action`] that does the additional things that come with a drag.
#[allow(clippy::cast_possible_truncation)]
fn handle_editor_drag(
    window: Window,
    app: &mut Application,
    mut mouse: MouseEvent,
) -> Option<(usize, Option<Nibble>)> {
    let (editor, word_size) = match window {
        Window::Ascii => (&app.display.comp_layouts.ascii, 1),
        Window::Hex => (&app.display.comp_layouts.hex, 3),
        _ => {
            panic!("Trying to move cursor on unhandled window!")
        }
    };

    let click_past_contents = app.display.comp_layouts.bytes_per_line
        * app.display.comp_layouts.lines_per_screen
        + app.data.start_address
        > app.data.contents.len();

    let mut editor_last_col = app.display.comp_layouts.bytes_per_line as u16;
    let mut end_of_row = 1 + editor.x + (editor_last_col * word_size);

    // Allows cursor x position to be tracked outside of the initially selected viewport when
    // dragged. Quickly dragging to the right will select everything to the end of the row.
    if mouse.column <= editor.left() {
        mouse.column = editor.x + 1;
    } else if mouse.column >= end_of_row {
        mouse.column = end_of_row;
    }

    // Allows the view port to be moved up and down depending on the cursor has been dragged way
    // above or below it.
    let editor_bottom_row = editor.top()
        + 1
        + cmp::min(
            app.display.comp_layouts.lines_per_screen,
            (app.data.contents.len() - app.data.start_address)
                / app.display.comp_layouts.bytes_per_line,
        ) as u16;
    if mouse.row == 0 {
        mouse.row = 1;
        if let Some(mut result) = handle_editor_cursor_action(window, app, mouse) {
            if let Some(new_y) = result.0.checked_sub(app.display.comp_layouts.bytes_per_line) {
                result.0 = new_y;
                return Some(result);
            }
            return Some(result);
        }
        None
    } else if mouse.row > editor_bottom_row {
        // When the mouse is dragged past the end of the contents, we need to update drag, but not
        // change the start address/scroll.
        if click_past_contents {
            editor_last_col = ((app.data.contents.len() - app.data.start_address)
                % app.display.comp_layouts.bytes_per_line) as u16;
            end_of_row = 1 + editor.x + (editor_last_col * word_size);
            if mouse.column >= end_of_row {
                mouse.column = end_of_row;
            }
        }
        mouse.row = editor_bottom_row - u16::from(!click_past_contents);
        if let Some(mut result) = handle_editor_cursor_action(window, app, mouse) {
            if let Some(new_y) = result.0.checked_add(app.display.comp_layouts.bytes_per_line) {
                if new_y < app.data.contents.len() {
                    result.0 = new_y;
                    return Some(result);
                }
            }
            return Some(result);
        }
        None
    } else {
        handle_editor_cursor_action(window, app, mouse)
    }
}

/// Determines if the relative cursor/drag position should be updated.
#[allow(clippy::cast_possible_truncation)]
fn handle_editor_cursor_action(
    window: Window,
    app: &mut Application,
    mouse: MouseEvent,
) -> Option<(usize, Option<Nibble>)> {
    let (editor, word_size) = match window {
        Window::Ascii => (&app.display.comp_layouts.ascii, 1),
        Window::Hex => (&app.display.comp_layouts.hex, 3),
        _ => {
            panic!("Trying to move cursor on unhandled window!")
        }
    };
    // Identify the byte that was clicked on based on the relative position.
    let (mut rel_x, mut rel_y) =
        (mouse.column.saturating_sub(editor.x), mouse.row.saturating_sub(editor.y));

    // Do not consider a click to the space after the last byte of a full viewport to be a click.
    // The space after the last byte of every row is generally considered a click for the first
    // byte on the next row for dragging purposes.
    if rel_y == editor.height - 2
        && rel_x
            > app.display.comp_layouts.bytes_per_line as u16 * word_size
                - u16::from(window == Window::Hex)
    {
        return None;
    }

    // Account for the border of the viewport and only allow clicks to the end of the last
    // character.
    if rel_y > 0 && rel_x > 0 && editor.height - 1 > rel_y && editor.width - 1 > rel_x {
        match window {
            Window::Ascii => {
                (rel_x, rel_y) = (rel_x - 1, rel_y - 1);
                let content_pos = app.data.start_address
                    + (rel_y as usize * app.display.comp_layouts.bytes_per_line)
                    + (rel_x as usize);
                if content_pos < app.data.contents.len() {
                    return Some((content_pos, None));
                }
            }
            Window::Hex => {
                (rel_x, rel_y) = (rel_x, rel_y - 1);
                let content_pos = app.data.start_address
                    + (rel_y as usize * app.display.comp_layouts.bytes_per_line)
                    + (rel_x as usize / 3);
                if content_pos < app.data.contents.len() {
                    if rel_x % 3 < 2 {
                        return Some((content_pos, Some(Nibble::Beginning)));
                    }
                    return Some((content_pos, Some(Nibble::End)));
                }
            }
            _ => {
                panic!()
            }
        }
    }
    None
}
