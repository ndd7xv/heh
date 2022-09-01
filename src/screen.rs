//! In charge of calculating dimensions and displaying everything.

use std::{
    cmp,
    error::Error,
    io::{self, Stdout},
};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Clear, Paragraph},
    Terminal,
};

use crate::{
    app::{AppData, Nibble},
    label::{LabelHandler, LABEL_TITLES},
    windows::{editor::Editor, KeyHandler, Window},
};

use crate::byte::{as_str, get_color};

const COLOR_NULL: Color = Color::DarkGray;

pub(crate) struct ScreenHandler {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    pub(crate) terminal_size: Rect,
    pub(crate) comp_layouts: ComponentLayouts,
}

pub(crate) struct ComponentLayouts {
    line_numbers: Rect,
    hex: Rect,
    ascii: Rect,
    labels: Vec<Rect>,
    pub(crate) popup: Rect,
    pub(crate) bytes_per_line: usize,
    pub(crate) lines_per_screen: usize,
}

impl ScreenHandler {
    pub(crate) fn new() -> Result<Self, Box<dyn Error>> {
        let terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
        let terminal_size = terminal.size()?;
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
    fn calculate_dimensions(frame: Rect, window: &dyn KeyHandler) -> ComponentLayouts {
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
                Constraint::Length((frame.width - 10) * 3 / 4 - 1),
                Constraint::Length((frame.width - 10) / 4),
            ])
            .split(sections[0]);
        let mut labels = Vec::with_capacity(12);
        let label_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
            ])
            .split(sections[1]);
        for label in label_cols {
            labels.append(
                &mut Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Ratio(1, 4),
                        Constraint::Ratio(1, 4),
                        Constraint::Ratio(1, 4),
                        Constraint::Ratio(1, 4),
                    ])
                    .split(label),
            );
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
            labels,
        }
    }

    /// Calculates the dimensions of the popup that is being focused. Currently used in
    /// [`calculate_dimensions`](Self::calculate_dimensions) and
    /// [`set_focused_window`](crate::app::Application::set_focused_window)
    /// since the dimensions are constant and are only changed when the popup changes.
    pub(crate) fn calculate_popup_dimensions(frame: Rect, window: &dyn KeyHandler) -> Rect {
        window.dimensions().map_or_else(Rect::default, |dimensions| popup_rect(dimensions, frame))
    }

    fn generate_text<'a>(
        contents: &'a [u8],
        start_address: usize,
        offset: usize,
        bytes_per_line: usize,
        lines_per_screen: usize,
        nibble: &Nibble,
    ) -> (Text<'a>, Text<'a>, Text<'a>) {
        let content_lines = contents.len() / bytes_per_line + 1;
        let start_row = start_address / bytes_per_line;

        // Generate address lines
        let mut address_text = (0..cmp::min(lines_per_screen, content_lines - start_row))
            .map(|i| Spans::from(format!("{:08X?}\n", (start_address + i * bytes_per_line))))
            .collect::<Vec<Spans>>();

        // Display hex - chunks the bytes into rows and formats them into hex
        let mut hex_text = contents[start_address..]
            .chunks(bytes_per_line)
            .take(lines_per_screen)
            .map(|chunk| {
                Spans::from(
                    chunk
                        .iter()
                        .map(|&byte| {
                            Span::styled(
                                format!("{byte:02X?} "),
                                Style::default().fg(*get_color(byte)),
                            )
                        })
                        .collect::<Vec<Span>>(),
                )
            })
            .collect::<Vec<Spans>>();

        // Display ASCII bytes
        let mut ascii_text = contents[start_address..]
            .chunks(bytes_per_line)
            .take(lines_per_screen)
            .map(|chunk| {
                Spans::from(
                    chunk
                        .iter()
                        .map(|&byte| {
                            Span::styled(as_str(byte), Style::default().fg(*get_color(byte)))
                        })
                        .collect::<Vec<Span>>(),
                )
            })
            .collect::<Vec<Spans>>();

        let cursor_row = offset / bytes_per_line;
        let cursor_col = offset % bytes_per_line;

        // Style the selected byte that the cursor is on
        let cursor_byte = contents[offset];
        if cursor_row >= start_row && cursor_row < start_row + lines_per_screen {
            // Highlight the address row that the cursor is in for visibility
            address_text[cursor_row - start_row].0[0].style =
                Style::default().bg(Color::White).fg(Color::Black);

            // Highlight the selected nibble in the Hex table
            let byte = format!("{:02X?}", cursor_byte);
            let mut byte = byte.chars();
            hex_text[cursor_row - start_row].0[cursor_col] = Span::styled(
                byte.next().unwrap().to_string(),
                if nibble == &Nibble::Beginning {
                    Style::default().fg(*get_color(cursor_byte)).bg(COLOR_NULL)
                } else {
                    Style::default().fg(*get_color(cursor_byte))
                },
            );
            hex_text[cursor_row - start_row].0.insert(
                cursor_col + 1,
                Span::styled(
                    byte.next().unwrap().to_string(),
                    if nibble == &Nibble::End {
                        Style::default().fg(*get_color(cursor_byte)).bg(COLOR_NULL)
                    } else {
                        Style::default().fg(*get_color(cursor_byte))
                    },
                ),
            );
            hex_text[cursor_row - start_row]
                .0
                .insert(cursor_col + 2, Span::styled(" ", Style::default()));

            // Highlight the selected byte in the ASCII table
            ascii_text[cursor_row - start_row].0[cursor_col] = Span::styled(
                as_str(cursor_byte),
                Style::default().fg(*get_color(cursor_byte)).bg(COLOR_NULL),
            );
        }

        (address_text.into(), hex_text.into(), ascii_text.into())
    }

    /// Display the addresses, editors, labels, and popups based off of the specifications of
    /// [`ComponentLayouts`], defined by
    /// [`calculate_dimensions`](Self::calculate_dimensions).
    pub(crate) fn render(
        &mut self,
        app_info: &mut AppData,
        labels: &LabelHandler,
        window: &dyn KeyHandler,
    ) -> Result<(), Box<dyn Error>> {
        self.terminal.draw(|f| {
            // We check if we need to recompute the terminal size in the case that the saved off
            // variable differs from the current frame, which can occur when a terminal is resized
            // between an event handling and a rendering.
            let size = f.size();
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

            // Check if terminal is large enough
            if self.terminal_size.width < 50 || self.terminal_size.height < 15 {
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
                    .split(self.terminal_size);
                f.render_widget(dimension_notification, vertical_center[1]);
                return;
            }

            let (address_text, hex_text, ascii_text) = Self::generate_text(
                &app_info.contents,
                app_info.start_address,
                app_info.offset,
                self.comp_layouts.bytes_per_line,
                self.comp_layouts.lines_per_screen,
                &app_info.nibble,
            );

            // Render Line Numbers
            f.render_widget(
                Paragraph::new(address_text)
                    .block(Block::default().borders(Borders::ALL).title("Address")),
                self.comp_layouts.line_numbers,
            );

            // Render Hex
            f.render_widget(
                Paragraph::new(hex_text).block(
                    Block::default().borders(Borders::ALL).title("Hex").style(
                        if window.is_focusing(Window::Hex) {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        },
                    ),
                ),
                self.comp_layouts.hex,
            );

            // Render ASCII
            f.render_widget(
                Paragraph::new(ascii_text).block(
                    Block::default().borders(Borders::ALL).title("ASCII").style(
                        if window.is_focusing(Window::Ascii) {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        },
                    ),
                ),
                self.comp_layouts.ascii,
            );

            // Render Info
            for (i, label) in self.comp_layouts.labels.iter().enumerate() {
                f.render_widget(
                    Paragraph::new(labels[LABEL_TITLES[i]].clone())
                        .block(Block::default().borders(Borders::ALL).title(LABEL_TITLES[i])),
                    *label,
                );
            }

            // Render Popup
            if !window.is_focusing(Window::Hex) && !window.is_focusing(Window::Ascii) {
                f.render_widget(Clear, self.comp_layouts.popup);
                f.render_widget(window.widget(), self.comp_layouts.popup);
            }
        })?;
        Ok(())
    }
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
