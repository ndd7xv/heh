use ratatui::{
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph},
};

use crate::{app::Data, label::Handler as LabelHandler, screen::Handler as ScreenHandler};

use super::{adjust_offset, KeyHandler, PopupOutput, Window};

/// A window that accepts either a hexadecimal or an ASCII sequence and moves cursor to the next
/// occurrence of this sequence
///
/// This can be opened by pressing `CNTRLf`.
///
/// Each symbol group is either parsed as hexadecimal if it is preceded with "0x", or decimal if
/// not.
///
/// Replace ASCII "0x", with "0x30x", (0x30 is hexadecimal for ascii 0) e.g. to search for "0xFF"
/// in ASCII, search for "0x30xFF" instead.
#[derive(PartialEq, Eq)]
pub(crate) struct Search {
    pub(crate) input: String,
}

impl Search {
    pub(crate) fn new() -> Self {
        Self { input: String::new() }
    }
}

impl KeyHandler for Search {
    fn is_focusing(&self, window_type: super::Window) -> bool {
        window_type == Window::Search
    }
    fn char(&mut self, _: &mut Data, _: &mut ScreenHandler, _: &mut LabelHandler, c: char) {
        self.input.push(c);
    }
    fn get_user_input(&self) -> PopupOutput {
        PopupOutput::Str(&self.input)
    }
    fn backspace(&mut self, _: &mut Data, _: &mut ScreenHandler, _: &mut LabelHandler) {
        self.input.pop();
    }
    fn enter(&mut self, app: &mut Data, display: &mut ScreenHandler, labels: &mut LabelHandler) {
        let byte_sequence_to_search = self.input.as_bytes();
        if byte_sequence_to_search.is_empty() {
            labels.notification = "Empty search query".into();
            return;
        }

        app.search_term.clone_from(&self.input);
        app.reindex_search();

        perform_search(app, display, labels, &SearchDirection::Forward);
    }
    fn dimensions(&self) -> Option<(u16, u16)> {
        Some((50, 3))
    }
    fn widget(&self) -> Paragraph {
        Paragraph::new(Span::styled(&self.input, Style::default().fg(Color::White))).block(
            Block::default()
                .title("Search:")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Yellow)),
        )
    }
}

pub(crate) enum SearchDirection {
    Forward,
    Backward,
}

pub(crate) fn perform_search(
    app: &mut Data,
    display: &mut ScreenHandler,
    labels: &mut LabelHandler,
    search_direction: &SearchDirection,
) {
    if app.search_term.is_empty() {
        return;
    }

    // Cached search data may be invalidated if contents have changed
    if app.dirty {
        app.reindex_search();
    }

    // This check needs to happen after reindexing search
    if app.search_offsets.is_empty() {
        labels.notification = "Query not found".into();
        return;
    }

    let idx = get_next_match_index(&app.search_offsets, app.offset, search_direction);
    let found_position = *app.search_offsets.get(idx).expect("There should be at least one result");

    labels.notification =
        format!("Search: {} [{}/{}]", app.search_term, idx + 1, app.search_offsets.len());

    app.offset = found_position;
    labels.update_all(&app.contents[app.offset..]);
    adjust_offset(app, display, labels);
}

// Find closest index of a match to the current offset, wrapping to the other end of the file if necessary
// This performs a binary search for the current offset in the list of matches. If the current
// offset is not present, the binary search will return the index in the list of matches where
// the current offset would fit, and from that we either pick the index to the left or right
// depending on whether we're searching forwards or backwards from the current offset.
fn get_next_match_index(
    search_offsets: &[usize],
    current_offset: usize,
    search_direction: &SearchDirection,
) -> usize {
    match search_direction {
        SearchDirection::Forward => search_offsets
            .binary_search(&(current_offset + 1))
            .unwrap_or_else(|i| if i >= search_offsets.len() { 0 } else { i }),
        SearchDirection::Backward => search_offsets
            .binary_search(&(current_offset.checked_sub(1).unwrap_or(usize::MAX)))
            .unwrap_or_else(|i| if i == 0 { search_offsets.len() - 1 } else { i - 1 }),
    }
}

#[cfg(test)]
mod tests {
    use super::{get_next_match_index, SearchDirection};

    #[test]
    fn test_search() {
        fn search(
            search_offsets: &[usize],
            current_offset: usize,
            search_direction: &SearchDirection,
        ) -> usize {
            let idx = get_next_match_index(search_offsets, current_offset, search_direction);
            search_offsets[idx]
        }

        let search_offsets = vec![1, 4, 5, 7];
        // positioned in between matches
        assert_eq!(search(&search_offsets, 2, &SearchDirection::Backward), 1);
        assert_eq!(search(&search_offsets, 3, &SearchDirection::Backward), 1);

        // positioned on a match
        assert_eq!(search(&search_offsets, 4, &SearchDirection::Backward), 1);
        assert_eq!(search(&search_offsets, 4, &SearchDirection::Forward), 5);

        // wrap around
        assert_eq!(search(&search_offsets, 7, &SearchDirection::Forward), 1);
        assert_eq!(search(&search_offsets, 1, &SearchDirection::Backward), 7);
        assert_eq!(search(&search_offsets, 0, &SearchDirection::Backward), 7);

        // singular match
        let search_offsets = vec![3];
        assert_eq!(search(&search_offsets, 4, &SearchDirection::Backward), 3);
        assert_eq!(search(&search_offsets, 3, &SearchDirection::Backward), 3);
        assert_eq!(search(&search_offsets, 2, &SearchDirection::Backward), 3);
    }
}
