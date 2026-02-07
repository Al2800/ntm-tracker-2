use crate::rpc::types::SessionView;
use crate::theme;
use ftui::core::geometry::Rect;
use ftui::render::frame::Frame;
use ftui::Style;
use ftui::widgets::block::Block;
use ftui::widgets::borders::Borders;
use ftui::widgets::list::{List, ListItem, ListState};
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::{StatefulWidget, Widget};

/// State for session list selection.
pub struct SessionListState {
    pub list_state: ListState,
    pub expanded_index: Option<usize>,
}

impl SessionListState {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            list_state,
            expanded_index: None,
        }
    }

    pub fn selected(&self) -> Option<usize> {
        self.list_state.selected()
    }

    pub fn select_next(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        let i = self.list_state.selected().unwrap_or(0);
        self.list_state.select(Some((i + 1).min(len - 1)));
    }

    pub fn select_prev(&mut self) {
        let i = self.list_state.selected().unwrap_or(0);
        self.list_state.select(Some(i.saturating_sub(1)));
    }

    pub fn select_first(&mut self) {
        self.list_state.select(Some(0));
    }

    pub fn select_last(&mut self, len: usize) {
        if len > 0 {
            self.list_state.select(Some(len - 1));
        }
    }

    pub fn toggle_expand(&mut self) {
        let sel = self.list_state.selected();
        if self.expanded_index == sel {
            self.expanded_index = None;
        } else {
            self.expanded_index = sel;
        }
    }
}

/// Render the session list.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    sessions: &[SessionView],
    state: &mut SessionListState,
    focused: bool,
) {
    let border_color = if focused {
        theme::INFO
    } else {
        theme::BG_SURFACE
    };

    let block = Block::new()
        .title(" Sessions ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(border_color))
        .style(theme::raised_style());

    if sessions.is_empty() {
        let empty = Paragraph::new("  No sessions")
            .style(theme::muted_style())
            .block(block);
        empty.render(area, frame);
        return;
    }

    let items: Vec<ListItem> = sessions
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let marker = if state.expanded_index == Some(i) {
                "▾"
            } else {
                "▸"
            };
            let badge = theme::status_badge(&s.status);
            let color = theme::status_color(&s.status);
            let line = format!(
                " {marker} {name:<16} {badge} {status:<8} {count}",
                name = truncate(&s.name, 16),
                status = s.status,
                count = s.pane_count
            );
            ListItem::new(line).style(Style::new().fg(color))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(theme::highlight_style())
        .highlight_symbol(">> ");

    StatefulWidget::render(&list, area, frame, &mut state.list_state);
}

pub(crate) fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === SessionListState navigation tests (bd-1g4) ===

    #[test]
    fn test_new_starts_at_zero() {
        let state = SessionListState::new();
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn test_new_not_expanded() {
        let state = SessionListState::new();
        assert_eq!(state.expanded_index, None);
    }

    #[test]
    fn test_select_next_increments() {
        let mut state = SessionListState::new();
        state.select_next(5);
        assert_eq!(state.selected(), Some(1));
        state.select_next(5);
        assert_eq!(state.selected(), Some(2));
    }

    #[test]
    fn test_select_next_clamps_at_end() {
        let mut state = SessionListState::new();
        state.list_state.select(Some(4));
        state.select_next(5);
        assert_eq!(state.selected(), Some(4));
    }

    #[test]
    fn test_select_next_empty_list() {
        let mut state = SessionListState::new();
        state.select_next(0); // should not panic
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn test_select_prev_decrements() {
        let mut state = SessionListState::new();
        state.list_state.select(Some(3));
        state.select_prev();
        assert_eq!(state.selected(), Some(2));
    }

    #[test]
    fn test_select_prev_clamps_at_zero() {
        let mut state = SessionListState::new();
        state.select_prev();
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn test_select_first() {
        let mut state = SessionListState::new();
        state.list_state.select(Some(5));
        state.select_first();
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn test_select_last() {
        let mut state = SessionListState::new();
        state.select_last(10);
        assert_eq!(state.selected(), Some(9));
    }

    #[test]
    fn test_select_last_empty() {
        let mut state = SessionListState::new();
        state.select_last(0); // should not panic
    }

    #[test]
    fn test_toggle_expand_on() {
        let mut state = SessionListState::new();
        assert_eq!(state.expanded_index, None);
        state.toggle_expand();
        assert_eq!(state.expanded_index, Some(0));
    }

    #[test]
    fn test_toggle_expand_off() {
        let mut state = SessionListState::new();
        state.toggle_expand(); // on
        state.toggle_expand(); // off
        assert_eq!(state.expanded_index, None);
    }

    #[test]
    fn test_toggle_expand_switch() {
        let mut state = SessionListState::new();
        state.toggle_expand(); // expand index 0
        assert_eq!(state.expanded_index, Some(0));
        state.list_state.select(Some(2));
        state.toggle_expand(); // expand index 2 (different)
        assert_eq!(state.expanded_index, Some(2));
    }

    // === String truncation tests (bd-2x0m partial) ===

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        let result = truncate("hello world", 5);
        assert!(result.len() <= 8); // 4 chars + ellipsis (multi-byte)
        assert!(result.contains('…'));
    }

    #[test]
    fn test_truncate_empty_string() {
        assert_eq!(truncate("", 5), "");
    }
}
