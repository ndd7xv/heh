use std::{
    cmp,
    error::Error,
    io::{Seek, SeekFrom, Write},
};

use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind};

use crate::{
    app::{Application, FocusedWindow, Nibble, PopupData},
    label::LABEL_TITLES,
    screen::ClickedComponent,
};

/// Handles user input.
pub(crate) fn handle_input(app: &mut Application) -> Result<bool, Box<dyn Error>> {
    let event = event::read()?;
    match event {
        Event::Key(key) => {
            match key.code {
                // Directional inputs that move the selected offset
                KeyCode::Left => {
                    if app.nibble == Nibble::Beginning && app.focused_window == FocusedWindow::Hex
                        || app.focused_window == FocusedWindow::Ascii
                    {
                        app.offset = app.offset.saturating_sub(1);
                        offset_change_epilogue(app);
                    }
                    if app.focused_window == FocusedWindow::Hex {
                        app.nibble.toggle();
                    }
                }
                KeyCode::Right => {
                    if app.nibble == Nibble::End && app.focused_window == FocusedWindow::Hex
                        || app.focused_window == FocusedWindow::Ascii
                    {
                        app.offset = cmp::min(app.offset.saturating_add(1), app.contents.len() - 1);
                        offset_change_epilogue(app);
                    }
                    if app.focused_window == FocusedWindow::Hex {
                        app.nibble.toggle();
                    }
                }
                KeyCode::Up => {
                    if let FocusedWindow::Popup(_) = app.focused_window {
                    } else if let Some(new_offset) = app
                        .offset
                        .checked_sub(app.display.comp_layouts.bytes_per_line)
                    {
                        app.offset = new_offset;
                        offset_change_epilogue(app);
                    }
                }
                KeyCode::Down => {
                    if let FocusedWindow::Popup(_) = app.focused_window {
                    } else if let Some(new_offset) = app
                        .offset
                        .checked_add(app.display.comp_layouts.bytes_per_line)
                    {
                        if new_offset < app.contents.len() {
                            app.offset = new_offset;
                            offset_change_epilogue(app);
                        }
                    }
                }

                // Input that removes bytes
                KeyCode::Backspace => {
                    if let FocusedWindow::Popup(popup_data) = &mut app.focused_window {
                        popup_data.input.pop();
                    } else if app.offset > 0 {
                        app.contents.remove(app.offset - 1);
                        app.offset = app.offset.saturating_sub(1);
                        offset_change_epilogue(app);
                    }
                }
                KeyCode::Delete => {
                    if app.contents.len() > 1 {
                        app.contents.remove(app.offset);
                        app.offset = app.offset.saturating_sub(1);
                        offset_change_epilogue(app);
                    }
                }

                KeyCode::Enter => {
                    if let FocusedWindow::Popup(popup_data) = &mut app.focused_window {
                        let res = if popup_data.input.starts_with("0x") {
                            usize::from_str_radix(&popup_data.input[2..], 16)
                        } else {
                            popup_data.input.parse()
                        };
                        if let Ok(offset) = res {
                            if offset >= app.contents.len() {
                                app.labels.notification = String::from("Invalid range!");
                            } else {
                                app.offset = offset;
                                offset_change_epilogue(app);
                            }
                        } else {
                            app.labels.notification = format!("Error: {:?}", res.unwrap_err());
                        }
                        app.focused_window = app.last_window.clone();
                    }
                }
                // Character Input and Shortcuts
                KeyCode::Char(char) => {
                    // A flag to indicate if a user wasn't trying to modify a byte
                    // i.e., CNTRLs shouldn't save and then modify the file
                    let mut special_command = false;
                    match char {
                        'j' => {
                            if key.modifiers == KeyModifiers::CONTROL {
                                special_command = true;
                                if let FocusedWindow::Popup(_) = app.focused_window {
                                    app.focused_window = app.last_window.clone();
                                } else {
                                    app.last_window = app.focused_window.clone();
                                    app.focused_window = FocusedWindow::Popup(PopupData {
                                        input: String::new(),
                                    });
                                }
                            }
                        }
                        'q' => {
                            if key.modifiers == KeyModifiers::CONTROL {
                                return Ok(false);
                            }
                        }
                        's' => {
                            if key.modifiers == KeyModifiers::CONTROL {
                                special_command = true;
                                app.file.seek(SeekFrom::Start(0))?;
                                app.file.write_all(&app.contents)?;
                                app.file.set_len(app.contents.len() as u64)?;
                                app.labels.notification = String::from("Saved!");
                            }
                        }
                        '=' => {
                            if key.modifiers == KeyModifiers::ALT {
                                special_command = true;
                                app.labels.update_stream_length(cmp::min(
                                    app.labels.get_stream_length() + 1,
                                    64,
                                ));
                                app.labels.update_streams(&app.contents[app.offset..]);
                            }
                        }
                        '-' => {
                            if key.modifiers == KeyModifiers::ALT {
                                special_command = true;
                                app.labels.update_stream_length(cmp::max(
                                    app.labels.get_stream_length().saturating_sub(1),
                                    0,
                                ));
                                app.labels.update_streams(&app.contents[app.offset..]);
                            }
                        }
                        _ => {}
                    }
                    if !special_command
                        && key.modifiers | KeyModifiers::SHIFT == KeyModifiers::SHIFT
                    {
                        match &mut app.focused_window {
                            FocusedWindow::Ascii => {
                                app.contents[app.offset] = char as u8;
                                app.offset =
                                    cmp::min(app.offset.saturating_add(1), app.contents.len() - 1);
                                offset_change_epilogue(app);
                            }
                            FocusedWindow::Hex => {
                                if char.is_ascii_hexdigit() {
                                    // This can probably be optimized...
                                    match app.nibble {
                                        Nibble::Beginning => {
                                            let mut src = char.to_string();
                                            src.push(
                                                format!("{:02X}", app.contents[app.offset])
                                                    .chars()
                                                    .last()
                                                    .unwrap(),
                                            );
                                            let changed =
                                                u8::from_str_radix(src.as_str(), 16).unwrap();
                                            app.contents[app.offset] = changed;
                                        }
                                        Nibble::End => {
                                            let mut src =
                                                format!("{:02X}", app.contents[app.offset])
                                                    .chars()
                                                    .take(1)
                                                    .collect::<String>();
                                            src.push(char);
                                            let changed =
                                                u8::from_str_radix(src.as_str(), 16).unwrap();
                                            app.contents[app.offset] = changed;

                                            // Move to the next byte
                                            app.offset = cmp::min(
                                                app.offset.saturating_add(1),
                                                app.contents.len() - 1,
                                            );
                                            offset_change_epilogue(app);
                                        }
                                    }
                                    app.nibble.toggle()
                                } else {
                                    app.labels.notification = format!("Invalid Hex: {char}");
                                }
                            }
                            FocusedWindow::Popup(popup_data) => {
                                popup_data.input.push(char);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Event::Mouse(mouse) => {
            let component = app.display.identify_clicked_component(
                mouse.row,
                mouse.column,
                &app.focused_window,
            );
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    app.last_click = component;
                    match &app.last_click {
                        ClickedComponent::HexTable => {
                            app.focused_window = FocusedWindow::Hex;
                        }
                        ClickedComponent::AsciiTable => {
                            app.focused_window = FocusedWindow::Ascii;
                        }
                        ClickedComponent::Label(_) => {}
                        ClickedComponent::Unclickable => {}
                    }
                }
                MouseEventKind::Up(MouseButton::Left) => {
                    match component {
                        ClickedComponent::HexTable => {}
                        ClickedComponent::AsciiTable => {}
                        ClickedComponent::Label(i) => {
                            if app.last_click == component {
                                // Put string into clipboard
                                if let Some(clipboard) = app.clipboard.as_mut() {
                                    clipboard
                                        .set_text(app.labels[LABEL_TITLES[i]].clone())
                                        .unwrap();
                                    app.labels.notification =
                                        format!("{} copied!", LABEL_TITLES[i]);
                                } else {
                                    app.labels.notification = String::from("Can't find clipboard!");
                                }
                            }
                        }
                        ClickedComponent::Unclickable => {}
                    }
                }
                MouseEventKind::ScrollUp => {
                    let bytes_per_line = app.display.comp_layouts.bytes_per_line;

                    // Scroll up a line in the viewport without changing cursor.
                    app.start_address = app.start_address.saturating_sub(bytes_per_line);
                }
                MouseEventKind::ScrollDown => {
                    let bytes_per_line = app.display.comp_layouts.bytes_per_line;
                    let lines_per_screen = app.display.comp_layouts.lines_per_screen;

                    let content_lines = app.contents.len() / bytes_per_line + 1;
                    let start_row = app.start_address / bytes_per_line;

                    // Scroll down a line in the viewport without changing cursor.
                    // Until the viewport contains the last page of content.
                    if start_row + lines_per_screen < content_lines {
                        app.start_address = app.start_address.saturating_add(bytes_per_line);
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    Ok(true)
}

// Puts the offset back into the display
fn adjust_offset(app: &mut Application) {
    let line_adjustment = ((app.offset as f32 - app.start_address as f32)
        / app.display.comp_layouts.bytes_per_line as f32)
        .floor()
        .abs() as usize;
    let bytes_per_screen =
        app.display.comp_layouts.bytes_per_line * app.display.comp_layouts.lines_per_screen;
    if app.offset < app.start_address {
        app.start_address = app
            .start_address
            .saturating_sub(app.display.comp_layouts.bytes_per_line * line_adjustment);
    } else if app.offset >= app.start_address + (bytes_per_screen)
        && app.start_address + app.display.comp_layouts.bytes_per_line < app.contents.len()
    {
        app.start_address = app.start_address.saturating_add(
            app.display.comp_layouts.bytes_per_line
                * (line_adjustment + 1 - app.display.comp_layouts.lines_per_screen),
        );
    }
    app.labels.offset = format!("{:#X}", app.offset);
}
fn offset_change_epilogue(app: &mut Application) {
    app.labels.update_all(&app.contents[app.offset..]);
    adjust_offset(app);
}
