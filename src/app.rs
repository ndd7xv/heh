use std::{
    cmp,
    error::Error,
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    process,
};

use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind};

use arboard::Clipboard;

use crate::{
    label::{LabelHandler, LABEL_TITLES},
    screen::{
        ClickedComponent::{self, *},
        ScreenHandler,
    },
};

#[derive(PartialEq)]
pub(crate) enum FocusedEditor {
    Ascii,
    Hex,
}

#[derive(PartialEq)]
pub(crate) enum Nibble {
    Beginning,
    End,
}

pub(crate) struct Application {
    file: File,
    contents: Vec<u8>,
    offset: usize,
    cursor: usize,
    display: ScreenHandler,
    labels: LabelHandler,
    last_click: ClickedComponent,
    clipboard: Clipboard,
    focused_editor: FocusedEditor,
    nibble: Nibble,
}

impl Application {
    pub(crate) fn new(mut file: File) -> Result<Application, Box<dyn Error>> {
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .expect("Reading the contents of the file was interrupted.");
        if contents.is_empty() {
            eprintln!("heh does not support editing empty files");
            process::exit(1);
        }
        let labels = LabelHandler::new(&contents);
        Ok(Application {
            file,
            contents,
            offset: 0,
            cursor: 0,
            display: ScreenHandler::new()?,
            labels,
            last_click: Unclickable,
            clipboard: Clipboard::new().unwrap(),
            focused_editor: FocusedEditor::Hex,
            nibble: Nibble::Beginning,
        })
    }
    pub(crate) fn run(&mut self) -> Result<(), Box<dyn Error>> {
        self.display.setup()?;
        loop {
            self.render_display()?;
            if !self.handle_input()? {
                break;
            }
        }
        self.display.teardown()?;
        Ok(())
    }
    fn render_display(&mut self) -> Result<(), Box<dyn Error>> {
        self.display.render(
            &self.contents,
            self.offset,
            self.cursor,
            &self.labels,
            &self.focused_editor,
        )?;
        Ok(())
    }
    fn handle_input(&mut self) -> Result<bool, Box<dyn Error>> {
        let event = event::read()?;
        match event {
            Event::Key(key) => {
                // Meta controls
                if key.code == KeyCode::Char('q') && key.modifiers == KeyModifiers::CONTROL {
                    return Ok(false);
                } else if key.code == KeyCode::Char('s') && key.modifiers == KeyModifiers::CONTROL {
                    self.file.seek(SeekFrom::Start(0))?;
                    self.file.write_all(&self.contents)?;
                    self.file.set_len(self.contents.len() as u64)?;
                    self.labels.notification = String::from("Saved!");
                } else if key.code == KeyCode::Char('=') {
                    self.labels
                        .update_stream_length(cmp::min(self.labels.get_stream_length() + 1, 64));
                    self.labels.update_streams(&self.contents[self.cursor..]);
                } else if key.code == KeyCode::Char('-') {
                    self.labels.update_stream_length(cmp::max(
                        self.labels.get_stream_length().saturating_sub(1),
                        0,
                    ));
                    self.labels.update_streams(&self.contents[self.cursor..]);
                }
                // Navigation controls
                else if key.code == KeyCode::Right {
                    self.cursor = cmp::min(self.cursor.saturating_add(1), self.contents.len() - 1);
                    self.cursor_change_epilogue();
                } else if key.code == KeyCode::Left {
                    self.cursor = self.cursor.saturating_sub(1);
                    self.cursor_change_epilogue();
                } else if key.code == KeyCode::Up {
                    self.cursor = self
                        .cursor
                        .saturating_sub(self.display.comp_layouts.bytes_per_line);
                    self.cursor_change_epilogue();
                } else if key.code == KeyCode::Down {
                    self.cursor = cmp::min(
                        self.cursor
                            .saturating_add(self.display.comp_layouts.bytes_per_line),
                        self.contents.len() - 1,
                    );
                    self.cursor_change_epilogue();
                }
                // Input Controls
                else if key.code == KeyCode::Backspace {
                    if self.cursor > 0 {
                        self.contents.remove(self.cursor - 1);
                        self.cursor = self.cursor.saturating_sub(1);
                        self.cursor_change_epilogue();
                    }
                } else if key.code == KeyCode::Delete {
                    if self.contents.len() > 1 {
                        self.contents.remove(self.cursor);
                        self.cursor = self.cursor.saturating_sub(1);
                        self.cursor_change_epilogue();
                    }
                } else if key.modifiers | KeyModifiers::SHIFT == KeyModifiers::SHIFT {
                    match self.focused_editor {
                        FocusedEditor::Ascii => {
                            if let KeyCode::Char(c) = key.code {
                                self.contents[self.cursor] = c as u8;
                                self.cursor = cmp::min(
                                    self.cursor.saturating_add(1),
                                    self.contents.len() - 1,
                                );
                                self.cursor_change_epilogue();
                            }
                        }
                        FocusedEditor::Hex => {
                            if let KeyCode::Char(c) = key.code {
                                if c.is_ascii_hexdigit() {
                                    // This can probably be optimized...

                                    if self.nibble == Nibble::Beginning {
                                        // Change the first character of the selected byte
                                        let mut src = c.to_string();
                                        src.push(
                                            format!("{:02X}", self.contents[self.cursor])
                                                .chars()
                                                .last()
                                                .unwrap(),
                                        );
                                        let changed = u8::from_str_radix(src.as_str(), 16).unwrap();
                                        self.contents[self.cursor] = changed;
                                        // Move to next nibble
                                        self.nibble = Nibble::End;
                                    } else {
                                        let mut src = format!("{:02X}", self.contents[self.cursor])
                                            .chars()
                                            .take(1)
                                            .collect::<String>();
                                        src.push(c);
                                        let changed = u8::from_str_radix(src.as_str(), 16).unwrap();
                                        self.contents[self.cursor] = changed;

                                        // Move to the next byte
                                        self.cursor = cmp::min(
                                            self.cursor.saturating_add(1),
                                            self.contents.len() - 1,
                                        );
                                        self.cursor_change_epilogue();
                                    }
                                } else {
                                    self.labels.notification = format!("Invalid Hex: {c}");
                                }
                            }
                        }
                    }
                }
            }
            Event::Mouse(mouse) => {
                let component = self
                    .display
                    .identify_clicked_component(mouse.row, mouse.column);
                if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
                    self.last_click = component;
                    match self.last_click {
                        HexTable => {
                            self.focused_editor = FocusedEditor::Hex;
                            self.nibble = Nibble::Beginning;
                        }
                        AsciiTable => {
                            self.focused_editor = FocusedEditor::Ascii;
                        }
                        Label(_) => {}
                        Unclickable => {}
                    }
                } else if mouse.kind == MouseEventKind::Up(MouseButton::Left) {
                    match component {
                        HexTable => {}
                        AsciiTable => {}
                        Label(i) => {
                            if self.last_click == component {
                                // Put string into clipboard
                                self.clipboard
                                    .set_text(self.labels[LABEL_TITLES[i]].clone())
                                    .unwrap();
                                self.labels.notification = format!("{} copied!", LABEL_TITLES[i]);
                            }
                        }
                        Unclickable => {}
                    }
                }
                //self.labels.notification = format!("{:?} at ({}, {})", mouse.kind, mouse.row, mouse.column);
            }
            _ => {}
        }
        Ok(true)
    }
    // Puts the cursor back into the display
    fn adjust_offset(&mut self) {
        let line_adjustment = ((self.cursor as f32 - self.offset as f32)
            / self.display.comp_layouts.bytes_per_line as f32)
            .floor()
            .abs() as usize;
        let bytes_per_screen =
            self.display.comp_layouts.bytes_per_line * self.display.comp_layouts.lines_per_screen;
        if self.cursor < self.offset {
            self.offset = self
                .offset
                .saturating_sub(self.display.comp_layouts.bytes_per_line * line_adjustment);
        } else if self.cursor >= self.offset + (bytes_per_screen)
            && self.offset + self.display.comp_layouts.bytes_per_line < self.contents.len()
        {
            self.offset = self.offset.saturating_add(
                self.display.comp_layouts.bytes_per_line
                    * (line_adjustment + 1 - self.display.comp_layouts.lines_per_screen),
            );
        }
        self.labels.offset = format!("{:#X}", self.cursor);
    }
    fn cursor_change_epilogue(&mut self) {
        self.nibble = Nibble::Beginning;
        self.labels.update_all(&self.contents[self.cursor..]);
        self.adjust_offset();
    }
}
