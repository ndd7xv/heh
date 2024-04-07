use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::{app::Data, label::Handler as LabelHandler, screen::Handler as ScreenHandler};

use super::{KeyHandler, PopupOutput, Window};

pub(crate) struct UnsavedChanges {
    pub(crate) should_quit: bool,
}

impl KeyHandler for UnsavedChanges {
    fn is_focusing(&self, window_type: Window) -> bool {
        window_type == Window::UnsavedChanges
    }
    fn left(&mut self, _: &mut Data, _: &mut ScreenHandler, _: &mut LabelHandler) {
        if !self.should_quit {
            self.should_quit = true;
        }
    }
    fn right(&mut self, _: &mut Data, _: &mut ScreenHandler, _: &mut LabelHandler) {
        if self.should_quit {
            self.should_quit = false;
        }
    }
    fn get_user_input(&self) -> PopupOutput {
        PopupOutput::Boolean(self.should_quit)
    }
    fn dimensions(&self) -> Option<(u16, u16)> {
        Some((50, 5))
    }
    fn widget(&self) -> Paragraph {
        let message = vec![
            Line::from(Span::styled(
                "Are you sure you want to quit?",
                Style::default().fg(Color::White),
            )),
            Line::from(Span::from("")),
            Line::from(vec![
                Span::styled(
                    "    Yes    ",
                    if self.should_quit {
                        Style::default()
                    } else {
                        Style::default().fg(Color::White)
                    },
                ),
                Span::styled(
                    "    No    ",
                    if self.should_quit {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default()
                    },
                ),
            ]),
        ];
        Paragraph::new(message).alignment(Alignment::Center).block(
            Block::default()
                .title(Span::styled(
                    "You Have Unsaved Changes.",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Yellow)),
        )
    }
}

impl UnsavedChanges {
    pub(crate) fn new() -> Self {
        UnsavedChanges { should_quit: false }
    }
}
