//! The terminal hex editor in its entirety.
//!
//! The application holds the main components of the other modules, like the [`ScreenHandler`],
//! [`LabelHandler`], and input handling, as well as the state data that each of them need.

use std::{error::Error, fs::File, io::Read, process};

use crossterm::event::{self, Event};

use arboard::Clipboard;

use crate::{
    input,
    label::LabelHandler,
    screen::{
        ClickedComponent::{self, Unclickable},
        ScreenHandler,
    },
    windows::{Editor, KeyHandler},
};

/// Enum that represent grouping of 4 bits in a byte.
///
/// For example, the first nibble in 0XF4 is 1111, or the F in hexadecimal. This is specified by
/// [`Nibble::Beginning`]. The last four bits (or 4 in hex) would be specified by [`Nibble::End`].
#[derive(PartialEq)]
pub(crate) enum Nibble {
    Beginning,
    End,
}

impl Nibble {
    pub(crate) fn toggle(&mut self) {
        match self {
            Self::Beginning => *self = Self::End,
            Self::End => *self = Self::Beginning,
        }
    }
}

/// State Information needed by the [`ScreenHandler`] and [`KeyHandler`].
pub(crate) struct AppData {
    /// The file under editting.
    pub(crate) file: File,

    /// The file content.
    pub(crate) contents: Vec<u8>,

    /// Offset of the first content byte that is visible on the screen.
    pub(crate) start_address: usize,

    /// Offset of the content byte under cursor.
    pub(crate) offset: usize,

    /// The nibble that is currently selected in the Hex viewport.
    pub(crate) nibble: Nibble,

    /// The last clicked (key down AND key up) label/window.
    pub(crate) last_click: ClickedComponent,

    /// Copies label data to your clipboard.
    pub(crate) clipboard: Option<Clipboard>,

    /// The editor that is currently selected. This editor will be refocused upon a popup closing.
    pub(crate) editor: Editor,
}

/// Application provides the user interaction interface and renders the terminal screen in response
/// to user actions.
pub(crate) struct Application {
    /// The application's state and data.
    pub(crate) data: AppData,

    /// Renders and displays objects to the terminal.
    pub(crate) display: ScreenHandler,

    /// The labels at the bottom of the UI that provide information
    /// based on the current offset.
    pub(crate) labels: LabelHandler,

    /// The window that handles keyboard input. This is usually in the form of the Hex/ASCII editor
    /// or popups.
    pub(crate) key_handler: Box<dyn KeyHandler>,
}

impl Application {
    /// Creates a new application, focusing the Hex editor and starting with an offset of 0 by
    /// default. This is called once at the beginning of the program.
    ///
    /// This errors out if the file specified is empty.
    pub(crate) fn new(mut file: File) -> Result<Self, Box<dyn Error>> {
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).expect("Reading the contents of the file was interrupted.");
        if contents.is_empty() {
            eprintln!("heh does not support editing empty files");
            process::exit(1);
        }
        let mut labels = LabelHandler::new(&contents);
        let clipboard = Clipboard::new().ok();
        if clipboard.is_none() {
            labels.notification = String::from("Can't find clipboard!");
        }
        Ok(Self {
            data: AppData {
                file,
                contents,
                start_address: 0,
                offset: 0,
                nibble: Nibble::Beginning,
                last_click: Unclickable,
                clipboard,
                editor: Editor::Hex,
            },
            display: ScreenHandler::new()?,
            labels,
            key_handler: Box::from(Editor::Hex),
        })
    }

    /// A loop that repeatedly renders the terminal and modifies state based on input. Is stopped
    /// when input handling receives CNTRLq, the command to stop.
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

    /// Renders the display. This is a wrapper around [`ScreenHandler`'s
    /// render](ScreenHandler::render) method.
    fn render_display(&mut self) -> Result<(), Box<dyn Error>> {
        self.display.render(&self.data, &self.labels, self.key_handler.as_ref())?;
        Ok(())
    }

    /// Handles all forms of user input. This calls out to code in [input], which uses
    /// [Application's `key_handler` method](Application::key_handler) to determine what to do for
    /// key input.
    fn handle_input(&mut self) -> Result<bool, Box<dyn Error>> {
        let event = event::read()?;
        match event {
            Event::Key(key) => {
                return input::handle_key_input(self, key);
            }
            Event::Mouse(mouse) => {
                input::handle_mouse_input(self, mouse);
            }
            Event::Resize(_, _) => {}
        }
        Ok(true)
    }
}
