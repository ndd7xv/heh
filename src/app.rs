//! The terminal hex editor in its entirety.
//!
//! The application holds the main components of the other modules, like the [`ScreenHandler`],
//! [`LabelHandler`], and input handling, as well as the state data that each of them need.

use std::{
    collections::hash_map::DefaultHasher, error::Error, fs::File, hash::Hasher, io::Read, process,
};

use arboard::Clipboard;
use crossterm::event::{self, Event};

use crate::decoder::Encoding;
use crate::windows::search::Search;
use crate::{
    input,
    label::LabelHandler,
    screen::ScreenHandler,
    windows::{
        editor::Editor, jump_to_byte::JumpToByte, unsaved_changes::UnsavedChanges, KeyHandler,
        Window,
    },
};

/// Enum that represent grouping of 4 bits in a byte.
///
/// For example, the first nibble in 0XF4 is 1111, or the F in hexadecimal. This is specified by
/// [`Nibble::Beginning`]. The last four bits (or 4 in hex) would be specified by [`Nibble::End`].
#[derive(PartialEq, Copy, Clone, Debug)]
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

/// An instance of a user action, used to implement the undo feature.
///
/// These actions record the previous state - deleting the first byte (x00) correlates to
/// Delete(0, x00).
pub(crate) enum Action {
    /// Tracks a user keypress to modify the contents of the file.
    CharacterInput(usize, u8, Option<Nibble>),

    /// Tracks when a user deletes the byte before the cursor.
    Backspace(usize, u8),

    /// Tracks when a user deletes the byte at the current cursor.
    Delete(usize, u8),
}

/// State Information needed by the [`ScreenHandler`] and [`KeyHandler`].
pub(crate) struct AppData {
    /// The file under editing.
    pub(crate) file: File,

    /// The file content.
    pub(crate) contents: Vec<u8>,

    /// The decoding used for the editor.
    pub(crate) encoding: Encoding,

    /// The hashed content, used for checking if anything has been changed.
    pub(crate) hashed_contents: u64,

    /// Offset of the first content byte that is visible on the screen.
    pub(crate) start_address: usize,

    /// Offset of the content byte under cursor.
    pub(crate) offset: usize,

    /// The nibble that is currently selected in the Hex viewport.
    pub(crate) nibble: Nibble,

    /// The last clicked (key down AND key up) label/window.
    pub(crate) last_click: Window,

    /// A flag to enable dragging, only when a click is first valid.
    pub(crate) drag_enabled: bool,

    /// The most recent cursor location where a drag occurred
    pub(crate) last_drag: Option<usize>,

    /// The nibble that was last hovered from the drag.
    pub(crate) drag_nibble: Option<Nibble>,

    /// Copies label data to your clipboard.
    pub(crate) clipboard: Option<Clipboard>,

    /// The editor that is currently selected. This editor will be refocused upon a popup closing.
    pub(crate) editor: Editor,

    /// A series of actions that keep track of what the user does.
    pub(crate) actions: Vec<Action>,

    /// Term the user is searching for.
    pub(crate) search_term: String,

    /// List of all offsets that the search term was found at.
    pub(crate) search_offsets: Vec<usize>,
}

impl AppData {
    /// Reindexes contents to find locations of the user's search term.
    pub(crate) fn reindex_search(&mut self) {
        self.search_offsets = self
            .contents
            .windows(self.search_term.len())
            .enumerate()
            .filter_map(|(idx, w)| (w == self.search_term.as_bytes()).then_some(idx))
            .collect();
    }

    /// Hashes the contents of a file and is used to check if there are any changes.
    pub(crate) fn hash_contents(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        hasher.write(&self.contents);
        hasher.finish()
    }
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
    pub(crate) fn new(
        mut file: File,
        encoding: Encoding,
        offset: usize,
    ) -> Result<Self, Box<dyn Error>> {
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).expect("Reading the contents of the file was interrupted.");
        if contents.is_empty() {
            eprintln!("heh does not support editing empty files");
            process::exit(1);
        } else if offset >= contents.len() {
            eprintln!(
                "The specified offset ({offset}) is too large! (must be less than {})",
                contents.len()
            );
            process::exit(1);
        }

        let mut labels = LabelHandler::new(&contents, offset);
        let clipboard = Clipboard::new().ok();
        if clipboard.is_none() {
            labels.notification = String::from("Can't find clipboard!");
        }

        let display = ScreenHandler::new()?;

        let mut app = Self {
            data: AppData {
                file,
                contents,
                encoding,
                hashed_contents: 0,
                start_address: (offset / display.comp_layouts.bytes_per_line)
                    * display.comp_layouts.bytes_per_line,
                offset,
                nibble: Nibble::Beginning,
                last_click: Window::Unhandled,
                drag_enabled: false,
                last_drag: None,
                drag_nibble: None,
                clipboard,
                editor: Editor::Hex,
                actions: vec![],
                search_term: String::new(),
                search_offsets: Vec::new(),
            },
            display,
            labels,
            key_handler: Box::from(Editor::Hex),
        };

        app.data.hashed_contents = app.data.hash_contents();

        Ok(app)
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
        self.display.render(&mut self.data, &self.labels, self.key_handler.as_ref())?;
        Ok(())
    }

    /// Handles all forms of user input. This calls out to code in [input], which uses
    /// [Application's `key_handler` method](Application::key_handler) to determine what to do for
    /// key input.
    fn handle_input(&mut self) -> Result<bool, Box<dyn Error>> {
        let event = event::read()?;
        match event {
            Event::Key(key) => {
                self.labels.notification.clear();
                return input::handle_key_input(self, key);
            }
            Event::Mouse(mouse) => {
                self.labels.notification.clear();
                input::handle_mouse_input(self, mouse);
            }
            Event::Resize(_, _) | Event::FocusGained | Event::FocusLost | Event::Paste(_) => {}
        }
        Ok(true)
    }

    /// Sets the current [`KeyHandler`]. This should be used when trying to focus another window.
    /// Setting the [`KeyHandler`] directly could cause errors.
    ///
    /// Popup dimensions are also changed here and are safe to do so because there are currently
    /// no popups that have dimensions based off of the size of the terminal frame.
    pub(crate) fn set_focused_window(&mut self, window: Window) {
        match window {
            Window::Hex => {
                self.key_handler = Box::from(Editor::Hex);
                self.data.editor = Editor::Hex;
            }
            Window::Ascii => {
                self.key_handler = Box::from(Editor::Ascii);
                self.data.editor = Editor::Ascii;
            }
            Window::JumpToByte => {
                self.key_handler = Box::from(JumpToByte::new());
                self.display.comp_layouts.popup = ScreenHandler::calculate_popup_dimensions(
                    self.display.terminal_size,
                    self.key_handler.as_ref(),
                );
            }
            Window::Search => {
                self.key_handler = Box::from(Search::new());
                self.display.comp_layouts.popup = ScreenHandler::calculate_popup_dimensions(
                    self.display.terminal_size,
                    self.key_handler.as_ref(),
                );
            }
            Window::UnsavedChanges => {
                self.key_handler = Box::from(UnsavedChanges::new());
                self.display.comp_layouts.popup = ScreenHandler::calculate_popup_dimensions(
                    self.display.terminal_size,
                    self.key_handler.as_ref(),
                );
            }
            // We should never try and focus these windows to accept input.
            Window::Unhandled | Window::Label(_) => {
                panic!()
            }
        }
    }

    /// Focuses the previously selected editor and is usually invoked after closing a popup.
    pub(crate) fn focus_editor(&mut self) {
        self.key_handler = Box::from(self.data.editor);
    }
}
