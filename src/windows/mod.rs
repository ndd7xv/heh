//! The components that implement [`KeyHandler`], which allow them to uniquely react to user input.
//! Example of a component include the Hex/ASCII editors and the Unsaved Changes warning.

pub(crate) mod editor;
pub(crate) mod jump_to_byte;
pub(crate) mod search;
pub(crate) mod unsaved_changes;

use ratatui::widgets::Paragraph;

use crate::{app::Data, label::Handler as LabelHandler, screen::Handler as ScreenHandler};

/// An enumeration of all the potential components that can be clicked. Used to identify which
/// component has been most recently clicked, and is also used to detmine which window is
/// focused in the `Application`'s input field.
#[derive(PartialEq, Eq, Copy, Clone)]
pub enum Window {
    Ascii,
    Hex,
    JumpToByte,
    Search,
    UnsavedChanges,
    Label(usize),
    Unhandled,
}

/// Represents the possible output of a variety of different popups.
#[derive(PartialEq, Eq)]
pub enum PopupOutput<'a> {
    Str(&'a str),
    Boolean(bool),
    NoOutput,
}

/// A trait for objects which handle input.
///
/// Depending on what is currently focused, user input can be handled in different ways. For
/// example, pressing enter should not modify the opened file in any form, but doing so while the
/// "Jump To Byte" popup is focused should attempt to move the cursor to the inputted byte.
pub trait KeyHandler {
    /// Checks if the current [`KeyHandler`] is a certain [`Window`].
    fn is_focusing(&self, window_type: Window) -> bool;

    // Methods that handle their respective keypresses.
    fn left(&mut self, _: &mut Data, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn right(&mut self, _: &mut Data, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn up(&mut self, _: &mut Data, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn down(&mut self, _: &mut Data, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn home(&mut self, _: &mut Data, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn end(&mut self, _: &mut Data, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn page_up(&mut self, _: &mut Data, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn page_down(&mut self, _: &mut Data, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn backspace(&mut self, _: &mut Data, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn delete(&mut self, _: &mut Data, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn enter(&mut self, _: &mut Data, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn char(&mut self, _: &mut Data, _: &mut ScreenHandler, _: &mut LabelHandler, _: char) {}

    /// Returns user input. Is currently used to get information from popups.
    fn get_user_input(&self) -> PopupOutput {
        PopupOutput::NoOutput
    }

    /// Returns the dimensions of to be used in displaying a popup. Returns None if an editor.
    fn dimensions(&self) -> Option<(u16, u16)> {
        None
    }

    /// Returns the contents to display on the screen
    fn widget(&self) -> Paragraph {
        Paragraph::new("")
    }
}

/// Moves the starting address of the editor viewports (Hex and ASCII) to include the cursor.
/// This is helpful because some window actions (search, jump to byte) move the cursor, and we
/// want to move the screen along with it.
///
/// If the cursor's location is before the viewports start, the viewports will move so that the
/// cursor is included in the first row.
///
/// If the cursor's location is past the end of the viewports, the viewports will move so that
/// the cursor is included in the final row.
pub(crate) fn adjust_offset(
    app: &mut Data,
    display: &mut ScreenHandler,
    labels: &mut LabelHandler,
) {
    let bytes_per_line = display.comp_layouts.bytes_per_line;
    let bytes_per_screen = bytes_per_line * display.comp_layouts.lines_per_screen;

    if app.offset < app.start_address {
        app.start_address = (app.offset / bytes_per_line) * bytes_per_line;
    } else if app.offset >= app.start_address + (bytes_per_screen) {
        app.start_address =
            (app.offset / bytes_per_line) * bytes_per_line - bytes_per_screen + bytes_per_line;
    }

    labels.offset = format!("{:#X}", app.offset);
}
