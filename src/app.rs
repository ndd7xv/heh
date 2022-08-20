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

#[derive(PartialEq, Clone)]
pub(crate) enum FocusedWindow {
    Ascii,
    Hex,
    Popup(PopupData),
}

#[derive(PartialEq, Clone)]
pub(crate) struct PopupData {
    pub(crate) input: String,
}

#[derive(PartialEq)]
pub(crate) enum Nibble {
    Beginning,
    End,
}

impl Nibble {
    fn toggle(&mut self) {
        match self {
            Nibble::Beginning => *self = Nibble::End,
            Nibble::End => *self = Nibble::Beginning,
        }
    }
}

/// Application provides the user interaction interface and renders the terminal screen in response to user actions.
pub(crate) struct Application {
    /// The file under editting.
    file: File,
    /// The file content.
    contents: Vec<u8>,
    /// Offset of the first content byte that is visible on the screen.
    start_address: usize,
    /// Offset of the content byte under cursor.
    offset: usize,
    /// The current component that is currently focused in the terminal.
    focused_window: FocusedWindow,
    /// The nibble that is currently selected in the Hex viewport.
    nibble: Nibble,
    display: ScreenHandler,
    labels: LabelHandler,
    last_click: ClickedComponent,
    /// The most previously focused window in the terminal.
    last_window: FocusedWindow,
    clipboard: Option<Clipboard>,
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
        let mut labels = LabelHandler::new(&contents);
        let clipboard = Clipboard::new().ok();
        if clipboard.is_none() {
            labels.notification = String::from("Can't find clipboard!");
        }
        Ok(Application {
            file,
            contents,
            start_address: 0,
            offset: 0,
            focused_window: FocusedWindow::Hex,
            nibble: Nibble::Beginning,
            display: ScreenHandler::new()?,
            labels,
            last_click: Unclickable,
            last_window: FocusedWindow::Hex,
            clipboard,
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
            self.start_address,
            self.offset,
            &self.labels,
            &self.focused_window,
            &self.nibble,
        )?;
        Ok(())
    }
    fn handle_input(&mut self) -> Result<bool, Box<dyn Error>> {
        let event = event::read()?;
        match event {
            Event::Key(key) => {
                match key.code {
                    // Directional inputs that move the selected offset
                    KeyCode::Left => {
                        if self.nibble == Nibble::Beginning
                            && self.focused_window == FocusedWindow::Hex
                            || self.focused_window == FocusedWindow::Ascii
                        {
                            self.offset = self.offset.saturating_sub(1);
                            self.offset_change_epilogue();
                        }
                        if self.focused_window == FocusedWindow::Hex {
                            self.nibble.toggle();
                        }
                    }
                    KeyCode::Right => {
                        if self.nibble == Nibble::End && self.focused_window == FocusedWindow::Hex
                            || self.focused_window == FocusedWindow::Ascii
                        {
                            self.offset =
                                cmp::min(self.offset.saturating_add(1), self.contents.len() - 1);
                            self.offset_change_epilogue();
                        }
                        if self.focused_window == FocusedWindow::Hex {
                            self.nibble.toggle();
                        }
                    }
                    KeyCode::Up => {
                        if let FocusedWindow::Popup(_) = self.focused_window {
                        } else if let Some(new_offset) = self
                            .offset
                            .checked_sub(self.display.comp_layouts.bytes_per_line)
                        {
                            self.offset = new_offset;
                            self.offset_change_epilogue();
                        }
                    }
                    KeyCode::Down => {
                        if let FocusedWindow::Popup(_) = self.focused_window {
                        } else if let Some(new_offset) = self
                            .offset
                            .checked_add(self.display.comp_layouts.bytes_per_line)
                        {
                            if new_offset < self.contents.len() {
                                self.offset = new_offset;
                                self.offset_change_epilogue();
                            }
                        }
                    }
                    KeyCode::Home => {
                        let bytes_per_line = self.display.comp_layouts.bytes_per_line;
                        self.offset = self.offset / bytes_per_line * bytes_per_line;
                        self.offset_change_epilogue();

                        if self.focused_window == FocusedWindow::Hex {
                            self.nibble = Nibble::Beginning;
                        }
                    }
                    KeyCode::End => {
                        let bytes_per_line = self.display.comp_layouts.bytes_per_line;
                        self.offset = cmp::min(
                            self.offset + (bytes_per_line - 1 - self.offset % bytes_per_line),
                            self.contents.len() - 1,
                        );
                        self.offset_change_epilogue();

                        if self.focused_window == FocusedWindow::Hex {
                            self.nibble = Nibble::End;
                        }
                    }

                    // Input that removes bytes
                    KeyCode::Backspace => {
                        if let FocusedWindow::Popup(popup_data) = &mut self.focused_window {
                            popup_data.input.pop();
                        } else if self.offset > 0 {
                            self.contents.remove(self.offset - 1);
                            self.offset = self.offset.saturating_sub(1);
                            self.offset_change_epilogue();
                        }
                    }
                    KeyCode::Delete => {
                        if self.contents.len() > 1 {
                            self.contents.remove(self.offset);
                            self.offset = self.offset.saturating_sub(1);
                            self.offset_change_epilogue();
                        }
                    }

                    KeyCode::Enter => {
                        if let FocusedWindow::Popup(popup_data) = &mut self.focused_window {
                            let res = if popup_data.input.starts_with("0x") {
                                usize::from_str_radix(&popup_data.input[2..], 16)
                            } else {
                                popup_data.input.parse()
                            };
                            if let Ok(offset) = res {
                                if offset >= self.contents.len() {
                                    self.labels.notification = String::from("Invalid range!");
                                } else {
                                    self.offset = offset;
                                    self.offset_change_epilogue();
                                }
                            } else {
                                self.labels.notification = format!("Error: {:?}", res.unwrap_err());
                            }
                            self.focused_window = self.last_window.clone();
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
                                    if let FocusedWindow::Popup(_) = self.focused_window {
                                        self.focused_window = self.last_window.clone();
                                    } else {
                                        self.last_window = self.focused_window.clone();
                                        self.focused_window = FocusedWindow::Popup(PopupData {
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
                                    self.file.seek(SeekFrom::Start(0))?;
                                    self.file.write_all(&self.contents)?;
                                    self.file.set_len(self.contents.len() as u64)?;
                                    self.labels.notification = String::from("Saved!");
                                }
                            }
                            '=' => {
                                if key.modifiers == KeyModifiers::ALT {
                                    special_command = true;
                                    self.labels.update_stream_length(cmp::min(
                                        self.labels.get_stream_length() + 1,
                                        64,
                                    ));
                                    self.labels.update_streams(&self.contents[self.offset..]);
                                }
                            }
                            '-' => {
                                if key.modifiers == KeyModifiers::ALT {
                                    special_command = true;
                                    self.labels.update_stream_length(cmp::max(
                                        self.labels.get_stream_length().saturating_sub(1),
                                        0,
                                    ));
                                    self.labels.update_streams(&self.contents[self.offset..]);
                                }
                            }
                            _ => {}
                        }
                        if !special_command
                            && key.modifiers | KeyModifiers::SHIFT == KeyModifiers::SHIFT
                        {
                            match &mut self.focused_window {
                                FocusedWindow::Ascii => {
                                    self.contents[self.offset] = char as u8;
                                    self.offset = cmp::min(
                                        self.offset.saturating_add(1),
                                        self.contents.len() - 1,
                                    );
                                    self.offset_change_epilogue();
                                }
                                FocusedWindow::Hex => {
                                    if char.is_ascii_hexdigit() {
                                        // This can probably be optimized...
                                        match self.nibble {
                                            Nibble::Beginning => {
                                                let mut src = char.to_string();
                                                src.push(
                                                    format!("{:02X}", self.contents[self.offset])
                                                        .chars()
                                                        .last()
                                                        .unwrap(),
                                                );
                                                let changed =
                                                    u8::from_str_radix(src.as_str(), 16).unwrap();
                                                self.contents[self.offset] = changed;
                                            }
                                            Nibble::End => {
                                                let mut src =
                                                    format!("{:02X}", self.contents[self.offset])
                                                        .chars()
                                                        .take(1)
                                                        .collect::<String>();
                                                src.push(char);
                                                let changed =
                                                    u8::from_str_radix(src.as_str(), 16).unwrap();
                                                self.contents[self.offset] = changed;

                                                // Move to the next byte
                                                self.offset = cmp::min(
                                                    self.offset.saturating_add(1),
                                                    self.contents.len() - 1,
                                                );
                                                self.offset_change_epilogue();
                                            }
                                        }
                                        self.nibble.toggle()
                                    } else {
                                        self.labels.notification = format!("Invalid Hex: {char}");
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
                let component = self.display.identify_clicked_component(
                    mouse.row,
                    mouse.column,
                    &self.focused_window,
                );
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        self.last_click = component;
                        match self.last_click {
                            HexTable => {
                                self.focused_window = FocusedWindow::Hex;
                            }
                            AsciiTable => {
                                self.focused_window = FocusedWindow::Ascii;
                            }
                            Label(_) => {}
                            Unclickable => {}
                        }
                    }
                    MouseEventKind::Up(MouseButton::Left) => {
                        match component {
                            HexTable => {}
                            AsciiTable => {}
                            Label(i) => {
                                if self.last_click == component {
                                    // Put string into clipboard
                                    if let Some(clipboard) = self.clipboard.as_mut() {
                                        clipboard
                                            .set_text(self.labels[LABEL_TITLES[i]].clone())
                                            .unwrap();
                                        self.labels.notification =
                                            format!("{} copied!", LABEL_TITLES[i]);
                                    } else {
                                        self.labels.notification =
                                            String::from("Can't find clipboard!");
                                    }
                                }
                            }
                            Unclickable => {}
                        }
                    }
                    MouseEventKind::ScrollUp => {
                        let bytes_per_line = self.display.comp_layouts.bytes_per_line;

                        // Scroll up a line in the viewport without changing cursor.
                        self.start_address = self.start_address.saturating_sub(bytes_per_line);
                    }
                    MouseEventKind::ScrollDown => {
                        let bytes_per_line = self.display.comp_layouts.bytes_per_line;
                        let lines_per_screen = self.display.comp_layouts.lines_per_screen;

                        let content_lines = self.contents.len() / bytes_per_line + 1;
                        let start_row = self.start_address / bytes_per_line;

                        // Scroll down a line in the viewport without changing cursor.
                        // Until the viewport contains the last page of content.
                        if start_row + lines_per_screen < content_lines {
                            self.start_address = self.start_address.saturating_add(bytes_per_line);
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
    fn adjust_offset(&mut self) {
        let line_adjustment = ((self.offset as f32 - self.start_address as f32)
            / self.display.comp_layouts.bytes_per_line as f32)
            .floor()
            .abs() as usize;
        let bytes_per_screen =
            self.display.comp_layouts.bytes_per_line * self.display.comp_layouts.lines_per_screen;
        if self.offset < self.start_address {
            self.start_address = self
                .start_address
                .saturating_sub(self.display.comp_layouts.bytes_per_line * line_adjustment);
        } else if self.offset >= self.start_address + (bytes_per_screen)
            && self.start_address + self.display.comp_layouts.bytes_per_line < self.contents.len()
        {
            self.start_address = self.start_address.saturating_add(
                self.display.comp_layouts.bytes_per_line
                    * (line_adjustment + 1 - self.display.comp_layouts.lines_per_screen),
            );
        }
        self.labels.offset = format!("{:#X}", self.offset);
    }
    fn offset_change_epilogue(&mut self) {
        self.labels.update_all(&self.contents[self.offset..]);
        self.adjust_offset();
    }
}
