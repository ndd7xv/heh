use std::{
    cmp,
    error::Error,
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    process,
};

use crossterm::event::{
    self, Event, KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use arboard::Clipboard;

use crate::{
    input::{Editor, FocusedWindow, InputHandler, JumpToByte},
    label::{LabelHandler, LABEL_TITLES},
    screen::{
        ClickedComponent::{self, AsciiTable, HexTable, Label, Unclickable},
        ScreenHandler,
    },
};

#[derive(PartialEq)]
pub(crate) enum Nibble {
    Beginning,
    End,
}

impl Nibble {
    pub(crate) fn toggle(&mut self) {
        match self {
            Nibble::Beginning => *self = Nibble::End,
            Nibble::End => *self = Nibble::Beginning,
        }
    }
}

/// State Information needed by the `ScreenHandler` and `InputHandler`.
pub(crate) struct AppData {
    /// The file under editting.
    file: File,
    /// The file content.
    pub(crate) contents: Vec<u8>,
    /// Offset of the first content byte that is visible on the screen.
    pub(crate) start_address: usize,
    /// Offset of the content byte under cursor.
    pub(crate) offset: usize,
    /// The nibble that is currently selected in the Hex viewport.
    pub(crate) nibble: Nibble,
    /// The last clicked (key down AND key up) label/window.
    last_click: ClickedComponent,
    /// Copies label data to your clipboard.
    clipboard: Option<Clipboard>,
}

/// Application provides the user interaction interface and renders the terminal screen in response
/// to user actions.
pub(crate) struct Application {
    /// The application's state and data.
    data: AppData,

    /// Renders and displays objects to the terminal.
    display: ScreenHandler,

    /// The labels at the bottom of the UI that provide information
    /// based on the current offset.
    pub(crate) labels: LabelHandler,

    /// The window that handles user input. This is usually in the form of the Hex/ASCII editor
    /// or popups.
    input_handler: Box<dyn InputHandler>,

    /// The input that was most previously selected.
    last_input_handler: Editor,
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
            data: AppData {
                file,
                contents,
                start_address: 0,
                offset: 0,
                nibble: Nibble::Beginning,
                last_click: Unclickable,
                clipboard,
            },
            display: ScreenHandler::new()?,
            labels,
            last_input_handler: Editor::Hex,
            input_handler: Box::from(Editor::Hex),
        })
    }
    pub(crate) fn run(&mut self) -> Result<(), Box<dyn Error>> {
        ScreenHandler::setup()?;
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
        self.display
            .render(&self.data, &self.labels, self.input_handler.as_ref())?;
        Ok(())
    }
    fn handle_input(&mut self) -> Result<bool, Box<dyn Error>> {
        let event = event::read()?;
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {
                    self.handle_arrow_key_input(key.code);
                }

                KeyCode::Home => {
                    self.input_handler
                        .home(&mut self.data, &mut self.display, &mut self.labels);
                }
                KeyCode::End => {
                    self.input_handler
                        .end(&mut self.data, &mut self.display, &mut self.labels);
                }

                KeyCode::Backspace => {
                    self.input_handler.backspace(
                        &mut self.data,
                        &mut self.display,
                        &mut self.labels,
                    );
                }
                KeyCode::Delete => {
                    self.input_handler
                        .delete(&mut self.data, &mut self.display, &mut self.labels);
                }

                KeyCode::Enter => {
                    self.input_handler
                        .enter(&mut self.data, &mut self.display, &mut self.labels);
                    self.input_handler = Box::from(self.last_input_handler);
                }

                KeyCode::Char(char) => {
                    // Because CNTRLq is the signal to quit, we propogate the message
                    // if this handling method returns false
                    if !self.handle_character_input(char, key.modifiers)? {
                        return Ok(false);
                    };
                }
                _ => {}
            },
            Event::Mouse(mouse) => {
                self.handle_mouse_input(mouse);
            }
            Event::Resize(_, _) => {}
        }
        Ok(true)
    }
    fn handle_arrow_key_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Left => {
                self.input_handler
                    .left(&mut self.data, &mut self.display, &mut self.labels);
            }
            KeyCode::Right => {
                self.input_handler
                    .right(&mut self.data, &mut self.display, &mut self.labels);
            }
            KeyCode::Up => {
                self.input_handler
                    .up(&mut self.data, &mut self.display, &mut self.labels);
            }
            KeyCode::Down => {
                self.input_handler
                    .down(&mut self.data, &mut self.display, &mut self.labels);
            }
            _ => unreachable!(),
        }
    }
    fn handle_character_input(
        &mut self,
        char: char,
        modifiers: KeyModifiers,
    ) -> Result<bool, Box<dyn Error>> {
        if modifiers == KeyModifiers::CONTROL {
            match char {
                'j' => {
                    if self.input_handler.is_focusing(FocusedWindow::JumpToByte) {
                        self.input_handler = Box::from(self.last_input_handler);
                    } else {
                        self.last_input_handler = *self
                            .input_handler
                            .as_any()
                            .downcast_ref()
                            .expect("The current window wasn't an editor");
                        self.input_handler = Box::from(JumpToByte::new());
                    }
                }
                'q' => return Ok(false),
                's' => {
                    self.data.file.seek(SeekFrom::Start(0))?;
                    self.data.file.write_all(&self.data.contents)?;
                    self.data.file.set_len(self.data.contents.len() as u64)?;
                    self.labels.notification = String::from("Saved!");
                }
                _ => {}
            }
        } else if modifiers == KeyModifiers::ALT {
            match char {
                '=' => {
                    self.labels
                        .update_stream_length(cmp::min(self.labels.get_stream_length() + 1, 64));
                    self.labels
                        .update_streams(&self.data.contents[self.data.offset..]);
                }
                '-' => {
                    self.labels.update_stream_length(cmp::max(
                        self.labels.get_stream_length().saturating_sub(1),
                        0,
                    ));
                    self.labels
                        .update_streams(&self.data.contents[self.data.offset..]);
                }
                _ => {}
            }
        } else if modifiers | KeyModifiers::NONE | KeyModifiers::SHIFT
            == KeyModifiers::NONE | KeyModifiers::SHIFT
        {
            self.input_handler
                .char(&mut self.data, &mut self.display, &mut self.labels, char);
        }
        Ok(true)
    }
    fn handle_mouse_input(&mut self, mouse: MouseEvent) {
        let component = self.display.identify_clicked_component(
            mouse.row,
            mouse.column,
            self.input_handler.as_ref(),
        );
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.data.last_click = component;
                match self.data.last_click {
                    HexTable => {
                        self.input_handler = Box::from(Editor::Hex);
                    }
                    AsciiTable => {
                        self.input_handler = Box::from(Editor::Ascii);
                    }
                    Label(_) | Unclickable => {}
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                match component {
                    HexTable | AsciiTable | Unclickable => {}
                    Label(i) => {
                        if self.data.last_click == component {
                            // Put string into clipboard
                            if let Some(clipboard) = self.data.clipboard.as_mut() {
                                clipboard
                                    .set_text(self.labels[LABEL_TITLES[i]].clone())
                                    .unwrap();
                                self.labels.notification = format!("{} copied!", LABEL_TITLES[i]);
                            } else {
                                self.labels.notification = String::from("Can't find clipboard!");
                            }
                        }
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                let bytes_per_line = self.display.comp_layouts.bytes_per_line;

                // Scroll up a line in the viewport without changing cursor.
                self.data.start_address = self.data.start_address.saturating_sub(bytes_per_line);
            }
            MouseEventKind::ScrollDown => {
                let bytes_per_line = self.display.comp_layouts.bytes_per_line;
                let lines_per_screen = self.display.comp_layouts.lines_per_screen;

                let content_lines = self.data.contents.len() / bytes_per_line + 1;
                let start_row = self.data.start_address / bytes_per_line;

                // Scroll down a line in the viewport without changing cursor.
                // Until the viewport contains the last page of content.
                if start_row + lines_per_screen < content_lines {
                    self.data.start_address =
                        self.data.start_address.saturating_add(bytes_per_line);
                }
            }
            _ => {}
        }
    }
}
