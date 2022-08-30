//! The components that implement [`KeyHandler`], which allow them to uniquely react to user input.
//! Example of a component include the Hex/ASCII editors and the Unsaved Changes warning.

pub(crate) mod editor;
pub(crate) mod jump_to_byte;
pub(crate) mod unsaved_changes;

use tui::widgets::Paragraph;

use crate::{app::AppData, label::LabelHandler, screen::ScreenHandler};

/// An enumeration of all the potential components that could be focused. Used to identify which
/// component is currently focused in the `Application`'s input field.
#[derive(PartialEq, Eq)]
pub enum FocusedWindow {
    Ascii,
    Hex,
    JumpToByte,
    UnsavedChanges,
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
pub(crate) trait KeyHandler {
    /// Checks if the current [`KeyHandler`] is a certain [`FocusedWindow`].
    fn is_focusing(&self, window_type: FocusedWindow) -> bool;

    // Methods that handle their respective keypresses.
    fn left(&mut self, _: &mut AppData, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn right(&mut self, _: &mut AppData, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn up(&mut self, _: &mut AppData, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn down(&mut self, _: &mut AppData, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn home(&mut self, _: &mut AppData, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn end(&mut self, _: &mut AppData, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn backspace(&mut self, _: &mut AppData, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn delete(&mut self, _: &mut AppData, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn enter(&mut self, _: &mut AppData, _: &mut ScreenHandler, _: &mut LabelHandler) {}
    fn char(&mut self, _: &mut AppData, _: &mut ScreenHandler, _: &mut LabelHandler, _: char) {}

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
///
/// If the cursor's location is before the viewports start, the viewports will move so that the
/// cursor is included in the first row.
///
/// If the cursor's location is past the end of the viewports, the vierports will move so that
/// the cursor is included in the final row.
pub(crate) fn adjust_offset(
    app: &mut AppData,
    display: &mut ScreenHandler,
    labels: &mut LabelHandler,
) {
    let line_adjustment = if app.offset <= app.start_address {
        app.start_address - app.offset + display.comp_layouts.bytes_per_line - 1
    } else {
        app.offset - app.start_address
    } / display.comp_layouts.bytes_per_line;

    let bytes_per_screen =
        display.comp_layouts.bytes_per_line * display.comp_layouts.lines_per_screen;

    if app.offset < app.start_address {
        app.start_address =
            app.start_address.saturating_sub(display.comp_layouts.bytes_per_line * line_adjustment);
    } else if app.offset >= app.start_address + (bytes_per_screen)
        && app.start_address + display.comp_layouts.bytes_per_line < app.contents.len()
    {
        app.start_address = app.start_address.saturating_add(
            display.comp_layouts.bytes_per_line
                * (line_adjustment + 1 - display.comp_layouts.lines_per_screen),
        );
    }

    labels.offset = format!("{:#X}", app.offset);
}
