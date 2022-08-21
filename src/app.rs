use std::{error::Error, fs::File, io::Read, process};

use crossterm::event::{self, Event};

use arboard::Clipboard;

use crate::{
    input::{self, Editor, InputHandler},
    label::LabelHandler,
    screen::{
        ClickedComponent::{self, Unclickable},
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

    /// The window that handles user input. This is usually in the form of the Hex/ASCII editor
    /// or popups.
    pub(crate) input_handler: Box<dyn InputHandler>,

    /// The input that was most previously selected.
    pub(crate) last_input_handler: Editor,
}

impl Application {
    pub(crate) fn new(mut file: File) -> Result<Application, Box<dyn Error>> {
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
        self.display.render(&self.data, &self.labels, self.input_handler.as_ref())?;
        Ok(())
    }
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
