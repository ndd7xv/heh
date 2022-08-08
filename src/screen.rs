use std::{
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
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

use crate::{
    app::FocusedEditor,
    label::{LabelHandler, LABEL_TITLES},
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
    pub(crate) bytes_per_line: usize,
    pub(crate) lines_per_screen: usize,
}

#[derive(PartialEq)]
pub(crate) enum ClickedComponent {
    HexTable,
    AsciiTable,
    Label(usize),
    Unclickable,
}

impl ScreenHandler {
    pub(crate) fn new() -> Result<Self, Box<dyn Error>> {
        let terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
        let terminal_size = terminal.size()?;
        Ok(ScreenHandler {
            terminal,
            terminal_size,
            comp_layouts: Self::calculate_dimensions(terminal_size),
        })
    }
    pub(crate) fn setup(&self) -> Result<(), Box<dyn Error>> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
        Ok(())
    }
    pub(crate) fn teardown(&mut self) -> Result<(), Box<dyn Error>> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }
    pub(crate) fn identify_clicked_component(&self, row: u16, col: u16) -> ClickedComponent {
        let click = Rect::new(col, row, 0, 0);
        if self.comp_layouts.hex.union(click) == self.comp_layouts.hex {
            return ClickedComponent::HexTable;
        } else if self.comp_layouts.ascii.union(click) == self.comp_layouts.ascii {
            return ClickedComponent::AsciiTable;
        }
        for (i, &label) in self.comp_layouts.labels.iter().enumerate() {
            if label.union(click) == label {
                return ClickedComponent::Label(i);
            }
        }
        ClickedComponent::Unclickable
    }
    fn calculate_dimensions(frame: Rect) -> ComponentLayouts {
        // Establish Constraints
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(12)])
            .split(frame);
        let editors = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(10),
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
            )
        }

        // Calculate bytes per line
        let bytes_per_line = ((editors[1].width - 2) / 3) as usize;
        let lines_per_screen = (editors[1].height - 2) as usize;

        ComponentLayouts {
            line_numbers: editors[0],
            hex: editors[1],
            ascii: editors[2],
            bytes_per_line,
            lines_per_screen,
            labels,
        }
    }
    fn generate_text(
        contents: &[u8],
        start_address: usize,
        offset: usize,
        bytes_per_line: usize,
        lines_per_screen: usize,
    ) -> (Text<'_>, Text<'_>, Text<'_>) {
        let address_text = (0..lines_per_screen)
            .map(|i| format!("{:08X?}", (start_address + i * bytes_per_line)))
            .fold(String::new(), |mut a, b| {
                a.push_str(&b);
                a.push('\n');
                a
            });
        let mut hex_text = contents[start_address..]
            .chunks(bytes_per_line)
            .take(lines_per_screen)
            .map(|chunk| {
                Spans::from(
                    chunk
                        .iter()
                        .map(|byte| {
                            Span::styled(
                                format!("{byte:02X?} "),
                                Style::default().fg(*get_color(byte)),
                            )
                        })
                        .collect::<Vec<Span>>(),
                )
            })
            .collect::<Vec<Spans>>();

        let mut ascii_text = contents[start_address..]
            .chunks(bytes_per_line)
            .take(lines_per_screen)
            .map(|chunk| {
                Spans::from(
                    chunk
                        .iter()
                        .map(|byte| {
                            Span::styled(as_str(byte), Style::default().fg(*get_color(byte)))
                        })
                        .collect::<Vec<Span>>(),
                )
            })
            .collect::<Vec<Spans>>();

        let offset_byte = contents[offset];
        let offset_row = (offset - start_address) / bytes_per_line;
        let offset_col = (offset - start_address) % bytes_per_line;
        if offset_row < lines_per_screen {
            hex_text[offset_row].0[offset_col] = Span::styled(
                format!("{:02X?}", offset_byte),
                Style::default().fg(*get_color(&offset_byte)).bg(COLOR_NULL),
            );
            hex_text[offset_row]
                .0
                .insert(offset_col + 1, Span::styled(" ", Style::default()));
            ascii_text[offset_row].0[offset_col] = Span::styled(
                as_str(&offset_byte),
                Style::default().fg(*get_color(&offset_byte)).bg(COLOR_NULL),
            );
        }

        (address_text.into(), hex_text.into(), ascii_text.into())
    }
    pub(crate) fn render(
        &mut self,
        contents: &[u8],
        start_address: usize,
        offset: usize,
        labels: &LabelHandler,
        focused_editor: &FocusedEditor,
    ) -> Result<(), Box<dyn Error>> {
        self.terminal.draw(|f| {
            // We check if we need to recompute the terminal size in the case that the saved off variable
            // differs from the current frame, which can occur when a terminal is resized between an event
            // handling and a rendering.
            if f.size() != self.terminal_size {
                self.terminal_size = f.size();
                self.comp_layouts = ScreenHandler::calculate_dimensions(self.terminal_size);
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
                contents,
                start_address,
                offset,
                self.comp_layouts.bytes_per_line,
                self.comp_layouts.lines_per_screen,
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
                        if *focused_editor == FocusedEditor::Hex {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        },
                    ),
                ),
                self.comp_layouts.hex,
            );

            // Render Normal
            f.render_widget(
                Paragraph::new(ascii_text).block(
                    Block::default().borders(Borders::ALL).title("ASCII").style(
                        if *focused_editor == FocusedEditor::Ascii {
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
                    Paragraph::new(labels[LABEL_TITLES[i]].clone()).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(LABEL_TITLES[i]),
                    ),
                    *label,
                );
            }
        })?;
        Ok(())
    }
}
