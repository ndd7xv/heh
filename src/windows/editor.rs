use std::cmp;

use crate::{
    app::{Action, Data, Nibble},
    label::Handler as LabelHandler,
    screen::Handler as ScreenHandler,
};

use super::{
    adjust_offset,
    search::{perform_search, SearchDirection},
    KeyHandler, Window,
};

/// The main windows that allow users to edit HEX and ASCII.
#[derive(PartialEq, Eq, Clone, Copy)]
pub(crate) enum Editor {
    Ascii,
    Hex,
}

impl KeyHandler for Editor {
    fn is_focusing(&self, window_type: Window) -> bool {
        match self {
            Self::Ascii => window_type == Window::Ascii,
            Self::Hex => window_type == Window::Hex,
        }
    }
    fn left(&mut self, app: &mut Data, display: &mut ScreenHandler, labels: &mut LabelHandler) {
        app.last_drag = None;
        app.drag_nibble = None;
        match self {
            Self::Ascii => {
                app.offset = app.offset.saturating_sub(1);
                labels.update_all(&app.contents[app.offset..]);
                adjust_offset(app, display, labels);
            }
            Self::Hex => {
                if app.nibble == Nibble::Beginning {
                    app.offset = app.offset.saturating_sub(1);
                    labels.update_all(&app.contents[app.offset..]);
                    adjust_offset(app, display, labels);
                }
                app.nibble.toggle();
            }
        }
    }
    fn right(&mut self, app: &mut Data, display: &mut ScreenHandler, labels: &mut LabelHandler) {
        app.last_drag = None;
        app.drag_nibble = None;
        match self {
            Self::Ascii => {
                app.offset = cmp::min(app.offset.saturating_add(1), app.contents.len() - 1);
                labels.update_all(&app.contents[app.offset..]);
                adjust_offset(app, display, labels);
            }
            Self::Hex => {
                if app.nibble == Nibble::End {
                    app.offset = cmp::min(app.offset.saturating_add(1), app.contents.len() - 1);
                    labels.update_all(&app.contents[app.offset..]);
                    adjust_offset(app, display, labels);
                }
                app.nibble.toggle();
            }
        }
    }
    fn up(&mut self, app: &mut Data, display: &mut ScreenHandler, labels: &mut LabelHandler) {
        app.last_drag = None;
        app.drag_nibble = None;
        if let Some(new_offset) = app.offset.checked_sub(display.comp_layouts.bytes_per_line) {
            app.offset = new_offset;
            labels.update_all(&app.contents[app.offset..]);
            adjust_offset(app, display, labels);
        }
    }
    fn down(&mut self, app: &mut Data, display: &mut ScreenHandler, labels: &mut LabelHandler) {
        app.last_drag = None;
        app.drag_nibble = None;
        if let Some(new_offset) = app.offset.checked_add(display.comp_layouts.bytes_per_line) {
            if new_offset < app.contents.len() {
                app.offset = new_offset;
                labels.update_all(&app.contents[app.offset..]);
                adjust_offset(app, display, labels);
            }
        }
    }
    fn home(&mut self, app: &mut Data, display: &mut ScreenHandler, labels: &mut LabelHandler) {
        app.last_drag = None;
        app.drag_nibble = None;
        let bytes_per_line = display.comp_layouts.bytes_per_line;
        app.offset = app.offset / bytes_per_line * bytes_per_line;
        labels.update_all(&app.contents[app.offset..]);
        adjust_offset(app, display, labels);

        if self.is_focusing(Window::Hex) {
            app.nibble = Nibble::Beginning;
        }
    }
    fn end(&mut self, app: &mut Data, display: &mut ScreenHandler, labels: &mut LabelHandler) {
        app.last_drag = None;
        app.drag_nibble = None;
        let bytes_per_line = display.comp_layouts.bytes_per_line;
        app.offset = cmp::min(
            app.offset + (bytes_per_line - 1 - app.offset % bytes_per_line),
            app.contents.len() - 1,
        );
        labels.update_all(&app.contents[app.offset..]);
        adjust_offset(app, display, labels);

        if self.is_focusing(Window::Hex) {
            app.nibble = Nibble::End;
        }
    }
    fn page_up(&mut self, app: &mut Data, display: &mut ScreenHandler, labels: &mut LabelHandler) {
        app.last_drag = None;
        app.drag_nibble = None;
        app.offset = app.offset.saturating_sub(
            display.comp_layouts.bytes_per_line * display.comp_layouts.lines_per_screen,
        );
        labels.update_all(&app.contents[app.offset..]);
        adjust_offset(app, display, labels);
    }
    fn page_down(
        &mut self,
        app: &mut Data,
        display: &mut ScreenHandler,
        labels: &mut LabelHandler,
    ) {
        app.last_drag = None;
        app.drag_nibble = None;
        app.offset = cmp::min(
            app.offset.saturating_add(
                display.comp_layouts.bytes_per_line * display.comp_layouts.lines_per_screen,
            ),
            app.contents.len() - 1,
        );
        labels.update_all(&app.contents[app.offset..]);
        adjust_offset(app, display, labels);
    }
    fn backspace(
        &mut self,
        app: &mut Data,
        display: &mut ScreenHandler,
        labels: &mut LabelHandler,
    ) {
        if app.offset > 0 {
            app.actions.push(Action::Delete(
                app.offset.saturating_sub(1),
                app.contents.remove(app.offset - 1),
            ));
            app.offset = app.offset.saturating_sub(1);
            labels.update_all(&app.contents[app.offset..]);
            adjust_offset(app, display, labels);
            app.dirty = true;
        }
    }
    fn delete(&mut self, app: &mut Data, display: &mut ScreenHandler, labels: &mut LabelHandler) {
        if app.contents.len() > 1 {
            app.actions.push(Action::Delete(app.offset, app.contents.remove(app.offset)));
            labels.update_all(&app.contents[app.offset..]);
            adjust_offset(app, display, labels);
            app.dirty = true;
        }
    }
    fn char(
        &mut self,
        app: &mut Data,
        display: &mut ScreenHandler,
        labels: &mut LabelHandler,
        c: char,
    ) {
        app.last_drag = None;
        app.drag_nibble = None;
        match *self {
            Self::Ascii => {
                app.actions.push(Action::CharacterInput(
                    app.offset,
                    app.contents[app.offset],
                    None,
                ));
                app.contents[app.offset] = c as u8;
                app.dirty = true;
                app.offset = cmp::min(app.offset.saturating_add(1), app.contents.len() - 1);
                labels.update_all(&app.contents[app.offset..]);
                adjust_offset(app, display, labels);
            }
            Self::Hex => {
                app.actions.push(Action::CharacterInput(
                    app.offset,
                    app.contents[app.offset],
                    Some(app.nibble),
                ));
                if c.is_ascii_hexdigit() {
                    // This can probably be optimized...
                    match app.nibble {
                        Nibble::Beginning => {
                            let mut src = c.to_string();
                            src.push(
                                format!("{:02X}", app.contents[app.offset]).chars().last().unwrap(),
                            );
                            let changed = u8::from_str_radix(src.as_str(), 16).unwrap();
                            app.contents[app.offset] = changed;
                        }
                        Nibble::End => {
                            let mut src = format!("{:02X}", app.contents[app.offset])
                                .chars()
                                .take(1)
                                .collect::<String>();
                            src.push(c);
                            let changed = u8::from_str_radix(src.as_str(), 16).unwrap();
                            app.contents[app.offset] = changed;

                            // Move to the next byte
                            app.offset =
                                cmp::min(app.offset.saturating_add(1), app.contents.len() - 1);
                            labels.update_all(&app.contents[app.offset..]);
                            adjust_offset(app, display, labels);
                        }
                    }
                    app.nibble.toggle();
                    app.dirty = true;
                } else {
                    labels.notification = format!("Invalid Hex: {c}");
                }
            }
        }
    }

    fn enter(&mut self, data: &mut Data, display: &mut ScreenHandler, labels: &mut LabelHandler) {
        perform_search(data, display, labels, &SearchDirection::Forward);
    }
}
