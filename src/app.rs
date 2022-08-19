use std::{error::Error, fs::File, io::Read, process};

use arboard::Clipboard;

use crate::{
    input,
    label::LabelHandler,
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
    pub fn toggle(&mut self) {
        match self {
            Nibble::Beginning => *self = Nibble::End,
            Nibble::End => *self = Nibble::Beginning,
        }
    }
}

/// Application provides the user interaction interface and renders the terminal screen in response to user actions.
pub(crate) struct Application {
    /// The file under editting.
    pub(crate) file: File,
    /// The file content.
    pub(crate) contents: Vec<u8>,
    /// Offset of the first content byte that is visible on the screen.
    pub(crate) start_address: usize,
    /// Offset of the content byte under cursor.
    pub(crate) offset: usize,
    /// The current component that is currently focused in the terminal.
    pub(crate) focused_window: FocusedWindow,
    /// The nibble that is currently selected in the Hex viewport.
    pub(crate) nibble: Nibble,
    pub(crate) display: ScreenHandler,
    pub(crate) labels: LabelHandler,
    pub(crate) last_click: ClickedComponent,
    /// The most previously focused window in the terminal.
    pub(crate) last_window: FocusedWindow,
    pub(crate) clipboard: Option<Clipboard>,
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
            if !input::handle_input(self)? {
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
}
