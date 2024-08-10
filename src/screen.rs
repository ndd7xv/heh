//! In charge of calculating dimensions and displaying everything.

use std::{
    cmp,
    error::Error,
    io::{self, Stdout},
    rc::Rc,
};

use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame, Terminal,
};

use crate::chunk::OverlappingChunks;
use crate::{
    app::{Data, Nibble},
    decoder::ByteAlignedDecoder,
    label::{Handler as LabelHandler, LABEL_TITLES},
    windows::{editor::Editor, KeyHandler, Window},
};

const COLOR_NULL: Color = Color::DarkGray;

pub struct Handler {
    pub terminal: Terminal<CrosstermBackend<Stdout>>,
    pub terminal_size: Rect,
    pub comp_layouts: ComponentLayouts,
}

pub struct ComponentLayouts {
    line_numbers: Rect,
    pub(crate) hex: Rect,
    pub(crate) ascii: Rect,
    labels: Rc<Vec<Rect>>,
    pub(crate) popup: Rect,
    pub(crate) bytes_per_line: usize,
    pub(crate) lines_per_screen: usize,
}

impl Handler {
    /// Creates a new screen handler.
    ///
    /// # Errors
    ///
    /// This errors when constructing the terminal or retrieving the terminal size fails.
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
        let size = terminal.size()?;
        let terminal_size = Rect::new(0, 0, size.width, size.height);
        Ok(Self {
            terminal,
            terminal_size,
            comp_layouts: Self::calculate_dimensions(terminal_size, &Editor::Hex),
        })
    }
    pub(crate) fn setup() -> Result<(), Box<dyn Error>> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
        Ok(())
    }
    pub(crate) fn teardown(&mut self) -> Result<(), Box<dyn Error>> {
        disable_raw_mode()?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
        self.terminal.show_cursor()?;
        Ok(())
    }
    pub(crate) fn identify_clicked_component(
        &self,
        row: u16,
        col: u16,
        window: &dyn KeyHandler,
    ) -> Window {
        let click = Rect::new(col, row, 0, 0);
        let popup_enabled = !(window.is_focusing(Window::Hex) || window.is_focusing(Window::Ascii));
        if popup_enabled && self.comp_layouts.popup.union(click) == self.comp_layouts.popup {
            return Window::Unhandled;
        } else if self.comp_layouts.hex.union(click) == self.comp_layouts.hex {
            return Window::Hex;
        } else if self.comp_layouts.ascii.union(click) == self.comp_layouts.ascii {
            return Window::Ascii;
        }
        for (i, &label) in self.comp_layouts.labels.iter().enumerate() {
            if label.union(click) == label {
                return Window::Label(i);
            }
        }
        Window::Unhandled
    }

    /// Calculates the dimensions of the components that will be continually displayed.
    ///
    /// This includes the editors, labels, and address table.
    pub fn calculate_dimensions(frame: Rect, window: &dyn KeyHandler) -> ComponentLayouts {
        // Establish Constraints
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(12)])
            .split(frame);
        let editors = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(10),
                // The address table is Length(10) as specified above. Because the hex editor takes
                // 3 graphemes for every 1 that ASCII takes (each nibble plus a space), we multiply
                // the editors by those ratios.
                Constraint::Length((frame.width - 10) * 3 / 4),
                Constraint::Length((frame.width - 10) / 4 + 1),
            ])
            .split(sections[0]);
        let mut labels = Rc::new(Vec::with_capacity(12));
        let label_columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
            ])
            .split(sections[1]);
        for label in &*label_columns {
            let column_layout = &mut Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Ratio(1, 4),
                    Constraint::Ratio(1, 4),
                    Constraint::Ratio(1, 4),
                    Constraint::Ratio(1, 4),
                ])
                .split(*label);

            if let Some(labels) = Rc::get_mut(&mut labels) {
                labels.extend_from_slice(column_layout);
            }
        }

        // Calculate popup dimensions
        let popup = Self::calculate_popup_dimensions(frame, window);

        // Calculate bytes per line
        let bytes_per_line = ((editors[1].width - 2) / 3) as usize;
        let lines_per_screen = (editors[1].height - 2) as usize;

        ComponentLayouts {
            line_numbers: editors[0],
            hex: editors[1],
            ascii: editors[2],
            popup,
            bytes_per_line,
            lines_per_screen,
            labels: labels.to_vec().into(),
        }
    }

    /// Calculates the dimensions of the popup that is being focused. Currently used in
    /// [`calculate_dimensions`](Self::calculate_dimensions) and
    /// [`set_focused_window`](crate::app::Application::set_focused_window)
    /// since the dimensions are constant and are only changed when the popup changes.
    ///
    /// In the case that window is a an editor and not a popup, returns the default Rect,
    /// which is essentially not displayed at all.
    pub(crate) fn calculate_popup_dimensions(frame: Rect, window: &dyn KeyHandler) -> Rect {
        window.dimensions().map_or_else(Rect::default, |dimensions| popup_rect(dimensions, frame))
    }

    /// Generates all the visuals of the file contents to be displayed to user by calling
    /// [`generate_hex`] and [`generate_decoded`].
    fn generate_text(
        app_info: &mut Data,
        bytes_per_line: usize,
        lines_per_screen: usize,
    ) -> (Text, Text, Text) {
        let content_lines = app_info.contents.len() / bytes_per_line + 1;
        let start_row = app_info.start_address / bytes_per_line;

        // Generate address lines
        let address_text = (0..cmp::min(lines_per_screen, content_lines - start_row))
            .map(|i| {
                let row_address = app_info.start_address + i * bytes_per_line;
                let mut span = Span::from(format!("{row_address:08X?}\n"));
                // Highlight the address row that the cursor is in for visibility
                if (row_address..row_address + bytes_per_line).contains(&app_info.offset) {
                    span.style = span.style.fg(Color::Black).bg(Color::White);
                }
                Line::from(span)
            })
            .collect::<Vec<Line>>();

        let hex_text = generate_hex(app_info, bytes_per_line, lines_per_screen);
        let decoded_text = generate_decoded(app_info, bytes_per_line, lines_per_screen);

        (address_text.into(), hex_text.into(), decoded_text.into())
    }

    /// Display the addresses, editors, labels, and popups based off of the specifications of
    /// [`ComponentLayouts`], defined by
    /// [`calculate_dimensions`](Self::calculate_dimensions).
    pub(crate) fn render(
        &mut self,
        app_info: &mut Data,
        labels: &LabelHandler,
        window: &dyn KeyHandler,
    ) -> Result<(), Box<dyn Error>> {
        app_info.contents.compute_new_window(app_info.offset);

        self.terminal.draw(|frame| {
            // We check if we need to recompute the terminal size in the case that the saved off
            // variable differs from the current frame, which can occur when a terminal is resized
            // between an event handling and a rendering.
            let size = frame.area();
            if size != self.terminal_size {
                self.terminal_size = size;
                self.comp_layouts = Self::calculate_dimensions(self.terminal_size, window);

                // We change the start_address here to ensure that 0 is ALWAYS the first start
                // address. We round to preventing constant resizing always moving to 0.
                app_info.start_address = (app_info.start_address
                    + (self.comp_layouts.bytes_per_line / 2))
                    / self.comp_layouts.bytes_per_line
                    * self.comp_layouts.bytes_per_line;
            }

            Self::render_frame(
                frame,
                self.terminal_size,
                app_info,
                labels,
                window,
                &self.comp_layouts,
            );
        })?;
        Ok(())
    }

    /// Display the addresses, editors, labels, and popups based off of the specifications of
    /// [`ComponentLayouts`], defined by
    /// [`calculate_dimensions`](Self::calculate_dimensions).
    pub(crate) fn render_frame(
        frame: &mut Frame,
        area: Rect,
        app_info: &mut Data,
        labels: &LabelHandler,
        window: &dyn KeyHandler,
        comp_layouts: &ComponentLayouts,
    ) {
        // Check if terminal is large enough
        if area.width < 50 || area.height < 15 {
            let dimension_notification = Paragraph::new("Terminal dimensions must be larger!")
                .block(Block::default())
                .alignment(Alignment::Center);
            let vertical_center = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(40),
                    Constraint::Percentage(20),
                    Constraint::Percentage(40),
                ])
                .split(area);
            frame.render_widget(dimension_notification, vertical_center[1]);
            return;
        }

        let (address_text, hex_text, ascii_text) = Self::generate_text(
            app_info,
            comp_layouts.bytes_per_line,
            comp_layouts.lines_per_screen,
        );

        // Render Line Numbers
        frame.render_widget(
            Paragraph::new(address_text)
                .block(Block::default().borders(Borders::ALL).title("Address")),
            comp_layouts.line_numbers,
        );

        // Render Hex
        frame.render_widget(
            Paragraph::new(hex_text).block(
                Block::default().borders(Borders::ALL).title("Hex").style(
                    if window.is_focusing(Window::Hex) {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    },
                ),
            ),
            comp_layouts.hex,
        );

        // Render ASCII
        frame.render_widget(
            Paragraph::new(ascii_text).block(
                Block::default().borders(Borders::ALL).title("ASCII").style(
                    if window.is_focusing(Window::Ascii) {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    },
                ),
            ),
            comp_layouts.ascii,
        );

        // Render Info
        for (i, label) in comp_layouts.labels.iter().enumerate() {
            frame.render_widget(
                Paragraph::new(labels[LABEL_TITLES[i]].clone())
                    .block(Block::default().borders(Borders::ALL).title(LABEL_TITLES[i])),
                *label,
            );
        }

        // Render Popup
        if !window.is_focusing(Window::Hex) && !window.is_focusing(Window::Ascii) {
            frame.render_widget(Clear, comp_layouts.popup);
            frame.render_widget(window.widget(), comp_layouts.popup);
        }
    }
}

/// Display hex bytes with correct highlighting and colors by chunking the bytes into rows and
/// formatting them into hex.
///
/// NOTE: In UTF-8, a character takes up to 4 bytes and thus the encoding can break at the ends of a
/// chunk. Increasing the chunk size by 3 bytes at both ends before decoding and cropping them of
/// afterwards solves the issue for the visible parts.
fn generate_hex(app_info: &Data, bytes_per_line: usize, lines_per_screen: usize) -> Vec<Line> {
    let initial_offset = app_info.start_address.min(3);
    OverlappingChunks::new(
        &app_info.contents[(app_info.start_address - initial_offset)..],
        bytes_per_line,
        6,
    )
    .take(lines_per_screen)
    .enumerate()
    .map(|(row, chunk)| {
        let spans = chunk
            .iter()
            .zip(ByteAlignedDecoder::new(chunk, app_info.encoding))
            .skip(initial_offset)
            .take(bytes_per_line)
            .enumerate()
            .flat_map(|(col, (&byte, character))| {
                // We don't want an extra space at the end of each row.
                if col < bytes_per_line - 1 {
                    format!("{byte:02X?} ")
                } else {
                    format!("{byte:02X?}")
                }
                .chars()
                .enumerate()
                .map(|(nibble_pos, c)| {
                    let byte_pos = app_info.start_address + (row * bytes_per_line) + col;
                    let mut span =
                        Span::styled(c.to_string(), Style::default().fg(*character.color()));
                    let is_cursor = byte_pos == app_info.offset
                        && ((nibble_pos == 0 && app_info.nibble == Nibble::Beginning)
                            || (nibble_pos == 1 && app_info.nibble == Nibble::End));

                    // Determine if the specified nibble (or space) should have a
                    // lighter foreground because it is in the user's dragged range.
                    // The logic is more complicated for hex because users can select
                    // a single nibble from a byte.
                    let mut in_drag = false;
                    if let Some(drag) = app_info.last_drag {
                        let drag_nibble = app_info.drag_nibble.unwrap_or(Nibble::End);
                        if !(drag == app_info.offset && app_info.nibble == drag_nibble) {
                            let mut start = drag;
                            let mut end = app_info.offset;
                            let mut start_nibble = drag_nibble;
                            let mut end_nibble = app_info.nibble;

                            if app_info.offset < drag {
                                start = app_info.offset;
                                end = drag;
                                start_nibble = app_info.nibble;
                                end_nibble = drag_nibble;
                            }

                            // The only time the starting byte would not entirely be in
                            // drag range is when the first nibble is not highlighted.
                            // Similarly, the last nibble is only partially highlighted
                            // when the second (and last) nibble is not selected.
                            if byte_pos == start {
                                in_drag = !(nibble_pos == 0 && start_nibble == Nibble::End);
                            }
                            if byte_pos == end {
                                in_drag |= !(nibble_pos == 1 && end_nibble == Nibble::Beginning)
                                    && nibble_pos != 2;
                            }
                            if start == end && nibble_pos == 2 {
                                in_drag = false;
                            } else if end - start > 1 {
                                in_drag |= (start + 1..end).contains(&byte_pos);
                            }
                        }
                    }
                    if is_cursor || in_drag {
                        span.style = span.style.bg(COLOR_NULL);
                    }
                    span
                })
                .collect::<Vec<Span>>()
            })
            .collect::<Vec<Span>>();
        Line::from(spans)
    })
    .collect::<Vec<Line>>()
}

/// Display decoded bytes with correct highlighting and colors.
///
/// NOTE: In UTF-8, a character takes up to 4 bytes and thus the encoding can break at the ends of a
/// chunk. Increasing the chunk size by 3 bytes at both ends before decoding and cropping them of
/// afterwards solves the issue for the visible parts.
fn generate_decoded(app_info: &Data, bytes_per_line: usize, lines_per_screen: usize) -> Vec<Line> {
    let initial_offset = app_info.start_address.min(3);
    OverlappingChunks::new(
        &app_info.contents[(app_info.start_address - initial_offset)..],
        bytes_per_line,
        6,
    )
    .take(lines_per_screen)
    .enumerate()
    .map(|(row, chunk)| {
        Line::from(
            ByteAlignedDecoder::new(chunk, app_info.encoding)
                .skip(initial_offset)
                .take(bytes_per_line)
                .enumerate()
                .map(|(col, character)| {
                    let byte_pos = app_info.start_address + (row * bytes_per_line) + col;
                    let mut span = Span::styled(
                        character.escape().to_string(),
                        Style::default().fg(*character.color()),
                    );
                    // Highlight the selected byte in the ASCII table
                    let last_drag = app_info.last_drag.unwrap_or(app_info.offset);
                    if byte_pos == app_info.offset
                        || (app_info.offset..=last_drag).contains(&byte_pos)
                        || (last_drag..=app_info.offset).contains(&byte_pos)
                    {
                        span.style = span.style.bg(COLOR_NULL);
                    }
                    span
                })
                .collect::<Vec<Span>>(),
        )
    })
    .collect::<Vec<Line>>()
}

/// Generates the dimensions of an x by y popup that is centered in Rect r.
fn popup_rect((x, y): (u16, u16), r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(r.height.saturating_sub(y) / 2),
                Constraint::Length(y),
                Constraint::Min(r.height.saturating_sub(y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Min(r.width.saturating_sub(x) / 2),
                Constraint::Length(x),
                Constraint::Min(r.width.saturating_sub(x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_dimensions_no_popup() {
        let width = 100;
        let height = 100;

        // Given a terminal size of 100 x 100, when dimensions are calculated
        let key_handler: Box<dyn KeyHandler> = Box::from(Editor::Ascii);
        let layout = Handler::calculate_dimensions(Rect::new(0, 0, width, height), &*key_handler);

        // The "editors" section, which consists of the line number column, Hex input box, and
        // ASCII input box should have a size of height - 12 (there are 4 labels per column and
        // each label takes 3 lines; each takes the vertical space alongside these components).
        assert_eq!(layout.line_numbers.height, height - 12);
        assert_eq!(layout.hex.height, height - 12);
        assert_eq!(layout.ascii.height, height - 12);

        // The width of the line numbers column is hard coded to 10,
        assert_eq!(layout.line_numbers.width, 10);
        // The Hex editor takes up 3/4ths of the remaining horizontal space (rounded down as to not
        // overflow)...
        assert_eq!(layout.hex.width, (width - 10) * 3 / 4);
        // And the ASCII editor takes up the remaining 1/4th. In some instances, the dimensions
        // are larger than the layout, so instead of asserting (width - 10) / 4 we assert the
        // remaining space.
        assert_eq!(layout.ascii.width, width - (10 + ((width - 10) * 3 / 4)));

        // The remaining space should consist of the labels in a 4 by 4 grid. Since the height
        // of each label column is hard set to 12, 4 labels in a column should have a width of 3.
        for label in &*layout.labels {
            assert_eq!(label.width, width / 4);
            assert_eq!(label.height, 3);
        }
    }

    // TODO: Create a test for asserting the dimension of each popup
}
