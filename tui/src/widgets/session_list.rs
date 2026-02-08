use crate::rpc::types::{PaneView, SessionView};
use crate::theme;
use ftui::core::geometry::Rect;
use ftui::render::frame::Frame;
use ftui::Style;
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

/// Render the session list with tree-like guides, badges, and relative times.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    sessions: &[SessionView],
    panes: &[PaneView],
    state: &mut SessionListState,
    focused: bool,
) {
    let block = theme::panel_block(" Sessions ", focused);

    if sessions.is_empty() {
        let empty = Paragraph::new("  No sessions")
            .style(theme::muted_style())
            .block(block);
        empty.render(area, frame);
        return;
    }

    let mut items: Vec<ListItem> = Vec::new();

    for (i, s) in sessions.iter().enumerate() {
        let is_expanded = state.expanded_index == Some(i);
        let marker = if is_expanded { "▾" } else { "▸" };
        let badge = theme::status_badge(&s.status);
        let color = theme::status_color(&s.status);
        let rel_time = theme::relative_time(s.last_seen_at);

        let line = format!(
            " {marker} {name:<16} {badge} {status:<8} {count}p  {rel_time}",
            name = truncate(&s.name, 16),
            status = s.status,
            count = s.pane_count,
        );
        items.push(ListItem::new(line).style(Style::new().fg(color)));

        // If expanded, show inline pane summaries with tree guide chars
        if is_expanded {
            let session_panes: Vec<&PaneView> = panes
                .iter()
                .filter(|p| p.session_id == s.session_id)
                .collect();

            for (pi, pane) in session_panes.iter().enumerate() {
                let guide = if pi == session_panes.len() - 1 {
                    theme::TREE_LAST
                } else {
                    theme::TREE_BRANCH
                };
                let agent = theme::agent_label(pane.agent_type.as_deref().unwrap_or("--"));
                let p_badge = theme::status_badge(&pane.status);
                let p_color = theme::status_color(&pane.status);
                let cmd = match pane.status.as_str() {
                    "waiting" | "paused" => {
                        pane.status_reason.as_deref().unwrap_or("waiting...")
                    }
                    _ => pane.current_command.as_deref().unwrap_or("--"),
                };
                let pane_line = format!(
                    "   {guide}#{idx} {agent} {p_badge} {status:<8} {cmd}",
                    idx = pane.pane_index,
                    status = pane.status,
                    cmd = truncate(cmd, 20),
                );
                items.push(ListItem::new(pane_line).style(Style::new().fg(p_color)));
            }
        }
    }

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
