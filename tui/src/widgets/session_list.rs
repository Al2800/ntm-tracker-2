use crate::rpc::types::{PaneView, SessionView};
use crate::theme;
use ftui::core::geometry::Rect;
use ftui::render::frame::Frame;
use ftui::Style;
use ftui::widgets::list::{List, ListItem, ListState};
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::{StatefulWidget, Widget};

/// Maps a visual row in the session list to its logical item.
#[derive(Debug, Clone, PartialEq)]
pub enum RowKind {
    Session(usize),
    Pane { session_idx: usize, pane_idx: usize },
}

/// State for session list selection.
pub struct SessionListState {
    pub list_state: ListState,
    pub expanded_index: Option<usize>,
    pub row_map: Vec<RowKind>,
}

impl SessionListState {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            list_state,
            expanded_index: None,
            row_map: Vec::new(),
        }
    }

    pub fn selected(&self) -> Option<usize> {
        self.list_state.selected()
    }

    /// Returns the session index for the currently selected row.
    /// Pane rows return their parent session index.
    pub fn selected_session_index(&self) -> Option<usize> {
        let row = self.list_state.selected()?;
        match self.row_map.get(row) {
            Some(RowKind::Session(i)) => Some(*i),
            Some(RowKind::Pane { session_idx, .. }) => Some(*session_idx),
            None => None,
        }
    }

    /// Returns (session_idx, pane_idx) if cursor is on a pane row.
    pub fn selected_pane_in_session(&self) -> Option<(usize, usize)> {
        let row = self.list_state.selected()?;
        match self.row_map.get(row) {
            Some(RowKind::Pane { session_idx, pane_idx }) => Some((*session_idx, *pane_idx)),
            _ => None,
        }
    }

    /// Navigate to the next session row, skipping pane rows.
    pub fn select_next_session(&mut self) {
        if self.row_map.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0);
        // Find the next Session row after current
        for (i, kind) in self.row_map.iter().enumerate().skip(current + 1) {
            if matches!(kind, RowKind::Session(_)) {
                self.list_state.select(Some(i));
                return;
            }
        }
        // No next session found — stay put
    }

    /// Navigate to the previous session row, skipping pane rows.
    pub fn select_prev_session(&mut self) {
        if self.row_map.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0);
        if current == 0 {
            return;
        }
        // Search backward from current-1
        for i in (0..current).rev() {
            if matches!(self.row_map.get(i), Some(RowKind::Session(_))) {
                self.list_state.select(Some(i));
                return;
            }
        }
    }

    /// Jump to the first session row.
    pub fn select_first_session(&mut self) {
        if !self.row_map.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    /// Jump to the last session row.
    pub fn select_last_session(&mut self) {
        for (i, kind) in self.row_map.iter().enumerate().rev() {
            if matches!(kind, RowKind::Session(_)) {
                self.list_state.select(Some(i));
                return;
            }
        }
    }

    /// Select the row corresponding to a given session index.
    pub fn select_session_by_index(&mut self, session_idx: usize) {
        for (i, kind) in self.row_map.iter().enumerate() {
            if matches!(kind, RowKind::Session(s) if *s == session_idx) {
                self.list_state.select(Some(i));
                return;
            }
        }
    }

    /// Total visual rows (sessions + expanded panes).
    pub fn total_rows(&self) -> usize {
        self.row_map.len()
    }

    // Legacy methods kept for backward compat with existing callers
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
        // Use row_map to find the session index at cursor
        let session_idx = self.selected_session_index();
        if self.expanded_index == session_idx {
            self.expanded_index = None;
        } else {
            self.expanded_index = session_idx;
        }
    }

    /// Build row_map from session/pane data without rendering.
    /// Each session gets a Session row; if expanded, its panes follow.
    pub fn build_row_map(&mut self, sessions: &[SessionView], panes: &[PaneView]) {
        self.row_map.clear();
        for (i, session) in sessions.iter().enumerate() {
            self.row_map.push(RowKind::Session(i));
            if self.expanded_index == Some(i) {
                let session_panes: Vec<_> = panes
                    .iter()
                    .filter(|p| p.session_id == session.session_id)
                    .collect();
                for (pi, _) in session_panes.iter().enumerate() {
                    self.row_map.push(RowKind::Pane { session_idx: i, pane_idx: pi });
                }
            }
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
    connected: bool,
    spinner_frame: usize,
) {
    let block = theme::panel_block(" Sessions ", focused);

    if sessions.is_empty() {
        let text = if connected {
            let spinner = theme::SPINNER_FRAMES[spinner_frame % theme::SPINNER_FRAMES.len()];
            format!("  {spinner} Waiting for sessions...")
        } else {
            "  No sessions".to_string()
        };
        let empty = Paragraph::new(text)
            .style(theme::muted_style())
            .block(block);
        empty.render(area, frame);
        state.row_map.clear();
        return;
    }

    let mut items: Vec<ListItem> = Vec::new();
    state.row_map.clear();

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
        state.row_map.push(RowKind::Session(i));

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
                state.row_map.push(RowKind::Pane { session_idx: i, pane_idx: pi });
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

    /// Build a row_map for testing without needing a Frame.
    fn build_test_row_map(session_count: usize, expanded_idx: Option<usize>, pane_counts: &[usize]) -> Vec<RowKind> {
        let mut map = Vec::new();
        for i in 0..session_count {
            map.push(RowKind::Session(i));
            if expanded_idx == Some(i) {
                let pc = pane_counts.get(i).copied().unwrap_or(0);
                for pi in 0..pc {
                    map.push(RowKind::Pane { session_idx: i, pane_idx: pi });
                }
            }
        }
        map
    }

    fn state_with_row_map(row_map: Vec<RowKind>, selected: usize) -> SessionListState {
        let mut state = SessionListState::new();
        state.row_map = row_map;
        state.list_state.select(Some(selected));
        state
    }

    // === SessionListState basic tests ===

    #[test]
    fn test_new_starts_at_zero() {
        let state = SessionListState::new();
        assert_eq!(state.selected(), Some(0));
        assert!(state.row_map.is_empty());
    }

    #[test]
    fn test_new_not_expanded() {
        let state = SessionListState::new();
        assert_eq!(state.expanded_index, None);
    }

    // === Legacy navigation tests ===

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
        state.select_next(0);
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
        state.select_last(0);
    }

    // === RowKind + row_map tests ===

    #[test]
    fn test_row_map_empty_sessions() {
        let map = build_test_row_map(0, None, &[]);
        assert!(map.is_empty());
    }

    #[test]
    fn test_row_map_no_expansion() {
        let map = build_test_row_map(3, None, &[2, 3, 1]);
        assert_eq!(map.len(), 3);
        assert_eq!(map[0], RowKind::Session(0));
        assert_eq!(map[1], RowKind::Session(1));
        assert_eq!(map[2], RowKind::Session(2));
    }

    #[test]
    fn test_row_map_with_expansion() {
        // 3 sessions, session 0 expanded with 3 panes
        let map = build_test_row_map(3, Some(0), &[3, 2, 1]);
        assert_eq!(map.len(), 6); // 3 sessions + 3 panes
        assert_eq!(map[0], RowKind::Session(0));
        assert_eq!(map[1], RowKind::Pane { session_idx: 0, pane_idx: 0 });
        assert_eq!(map[2], RowKind::Pane { session_idx: 0, pane_idx: 1 });
        assert_eq!(map[3], RowKind::Pane { session_idx: 0, pane_idx: 2 });
        assert_eq!(map[4], RowKind::Session(1));
        assert_eq!(map[5], RowKind::Session(2));
    }

    // === selected_session_index tests ===

    #[test]
    fn test_selected_session_index_on_session_row() {
        let map = build_test_row_map(3, Some(0), &[3, 2, 1]);
        let state = state_with_row_map(map, 0); // on session 0
        assert_eq!(state.selected_session_index(), Some(0));
    }

    #[test]
    fn test_selected_session_index_on_pane_row() {
        let map = build_test_row_map(3, Some(0), &[3, 2, 1]);
        let state = state_with_row_map(map, 2); // on pane row (session 0, pane 1)
        assert_eq!(state.selected_session_index(), Some(0));
    }

    #[test]
    fn test_selected_session_index_on_second_session() {
        let map = build_test_row_map(3, Some(0), &[3, 2, 1]);
        let state = state_with_row_map(map, 4); // on session 1 (row 4)
        assert_eq!(state.selected_session_index(), Some(1));
    }

    #[test]
    fn test_selected_session_index_empty() {
        let state = SessionListState::new(); // empty row_map
        assert_eq!(state.selected_session_index(), None);
    }

    // === selected_pane_in_session tests ===

    #[test]
    fn test_selected_pane_on_session_row() {
        let map = build_test_row_map(3, Some(0), &[3, 2, 1]);
        let state = state_with_row_map(map, 0);
        assert_eq!(state.selected_pane_in_session(), None);
    }

    #[test]
    fn test_selected_pane_on_pane_row() {
        let map = build_test_row_map(3, Some(0), &[3, 2, 1]);
        let state = state_with_row_map(map, 2);
        assert_eq!(state.selected_pane_in_session(), Some((0, 1)));
    }

    // === Session-aware navigation tests ===

    #[test]
    fn test_next_session_skips_panes() {
        // Session 0 expanded with 3 panes: rows [S0, P0, P1, P2, S1, S2]
        let map = build_test_row_map(3, Some(0), &[3, 2, 1]);
        let mut state = state_with_row_map(map, 0); // on session 0
        state.select_next_session();
        assert_eq!(state.selected(), Some(4)); // jumped to session 1
    }

    #[test]
    fn test_next_session_from_pane_row() {
        let map = build_test_row_map(3, Some(0), &[3, 2, 1]);
        let mut state = state_with_row_map(map, 2); // on pane row within session 0
        state.select_next_session();
        assert_eq!(state.selected(), Some(4)); // jumped to session 1
    }

    #[test]
    fn test_next_session_at_last_is_noop() {
        let map = build_test_row_map(3, None, &[]);
        let mut state = state_with_row_map(map, 2); // on session 2 (last)
        state.select_next_session();
        assert_eq!(state.selected(), Some(2)); // stays
    }

    #[test]
    fn test_prev_session_skips_panes() {
        let map = build_test_row_map(3, Some(0), &[3, 2, 1]);
        let mut state = state_with_row_map(map, 4); // on session 1
        state.select_prev_session();
        assert_eq!(state.selected(), Some(0)); // jumped to session 0
    }

    #[test]
    fn test_prev_session_at_first_is_noop() {
        let map = build_test_row_map(3, None, &[]);
        let mut state = state_with_row_map(map, 0);
        state.select_prev_session();
        assert_eq!(state.selected(), Some(0)); // stays
    }

    #[test]
    fn test_first_session() {
        let map = build_test_row_map(3, Some(0), &[3, 2, 1]);
        let mut state = state_with_row_map(map, 5); // on session 2
        state.select_first_session();
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn test_last_session() {
        let map = build_test_row_map(3, Some(0), &[3, 2, 1]);
        let mut state = state_with_row_map(map, 0);
        state.select_last_session();
        assert_eq!(state.selected(), Some(5)); // session 2 is at row 5
    }

    #[test]
    fn test_navigation_empty_row_map() {
        let mut state = SessionListState::new();
        state.select_next_session(); // no panic
        state.select_prev_session(); // no panic
        state.select_first_session(); // no panic
        state.select_last_session(); // no panic
    }

    #[test]
    fn test_navigation_no_expansion() {
        // Simple case: 3 sessions, none expanded
        let map = build_test_row_map(3, None, &[]);
        let mut state = state_with_row_map(map, 0);
        state.select_next_session();
        assert_eq!(state.selected(), Some(1));
        state.select_next_session();
        assert_eq!(state.selected(), Some(2));
        state.select_prev_session();
        assert_eq!(state.selected(), Some(1));
    }

    #[test]
    fn test_navigation_last_session_expanded() {
        // Session 2 expanded with 2 panes: [S0, S1, S2, P0, P1]
        let map = build_test_row_map(3, Some(2), &[0, 0, 2]);
        let mut state = state_with_row_map(map, 1); // on session 1
        state.select_next_session();
        assert_eq!(state.selected(), Some(2)); // session 2
        state.select_next_session();
        assert_eq!(state.selected(), Some(2)); // stays (no more sessions)
    }

    #[test]
    fn test_select_session_by_index() {
        let map = build_test_row_map(3, Some(0), &[3, 2, 1]);
        let mut state = state_with_row_map(map, 0);
        state.select_session_by_index(1);
        assert_eq!(state.selected(), Some(4)); // session 1 is at row 4
        state.select_session_by_index(2);
        assert_eq!(state.selected(), Some(5)); // session 2 is at row 5
    }

    #[test]
    fn test_total_rows() {
        let map = build_test_row_map(3, Some(0), &[3, 2, 1]);
        let state = state_with_row_map(map, 0);
        assert_eq!(state.total_rows(), 6);
    }

    // === Toggle expand with row_map ===

    #[test]
    fn test_toggle_expand_on() {
        let map = build_test_row_map(3, None, &[]);
        let mut state = state_with_row_map(map, 0);
        assert_eq!(state.expanded_index, None);
        state.toggle_expand();
        assert_eq!(state.expanded_index, Some(0));
    }

    #[test]
    fn test_toggle_expand_off() {
        let map = build_test_row_map(3, None, &[]);
        let mut state = state_with_row_map(map, 0);
        state.toggle_expand();
        state.toggle_expand();
        assert_eq!(state.expanded_index, None);
    }

    #[test]
    fn test_toggle_expand_from_pane_row() {
        // Cursor on pane row of session 0 — toggle should collapse session 0
        let map = build_test_row_map(3, Some(0), &[3, 2, 1]);
        let mut state = state_with_row_map(map, 2); // pane row
        state.expanded_index = Some(0);
        state.toggle_expand(); // should collapse (same session)
        assert_eq!(state.expanded_index, None);
    }

    #[test]
    fn test_toggle_expand_switch() {
        let map = build_test_row_map(3, None, &[]);
        let mut state = state_with_row_map(map, 0);
        state.toggle_expand();
        assert_eq!(state.expanded_index, Some(0));
        state.list_state.select(Some(2));
        state.toggle_expand();
        assert_eq!(state.expanded_index, Some(2));
    }

    // === String truncation tests ===

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
        assert!(result.len() <= 8);
        assert!(result.contains('…'));
    }

    #[test]
    fn test_truncate_empty_string() {
        assert_eq!(truncate("", 5), "");
    }

    // === Render tests ===

    use crate::rpc::types::{PaneView, SessionView};
    use crate::test_helpers::*;
    use ftui::core::geometry::Rect;

    fn make_session(id: &str, name: &str, status: &str) -> SessionView {
        SessionView {
            session_id: id.to_string(),
            name: name.to_string(),
            status: status.to_string(),
            pane_count: 2,
            last_seen_at: chrono::Utc::now().timestamp(),
            ..Default::default()
        }
    }

    fn make_pane(pane_id: &str, session_id: &str, status: &str) -> PaneView {
        PaneView {
            pane_id: pane_id.to_string(),
            session_id: session_id.to_string(),
            status: status.to_string(),
            pane_index: 0,
            ..Default::default()
        }
    }

    #[test]
    fn test_render_empty_disconnected_shows_no_sessions() {
        test_frame!(pool, frame, 60, 8);
        let area = Rect::new(0, 0, 60, 8);
        let mut state = SessionListState::new();
        render(&mut frame, area, &[], &[], &mut state, false, false, 0);
        assert_text_present(&frame.buffer, "No sessions");
    }

    #[test]
    fn test_render_empty_connected_shows_waiting() {
        test_frame!(pool, frame, 60, 8);
        let area = Rect::new(0, 0, 60, 8);
        let mut state = SessionListState::new();
        render(&mut frame, area, &[], &[], &mut state, false, true, 0);
        assert_text_present(&frame.buffer, "Waiting for sessions");
    }

    #[test]
    fn test_render_shows_session_names() {
        test_frame!(pool, frame, 80, 10);
        let area = Rect::new(0, 0, 80, 10);
        let sessions = vec![
            make_session("s1", "my-project", "active"),
            make_session("s2", "backend", "idle"),
        ];
        let mut state = SessionListState::new();
        render(&mut frame, area, &sessions, &[], &mut state, false, true, 0);
        assert_text_present(&frame.buffer, "my-project");
        assert_text_present(&frame.buffer, "backend");
    }

    #[test]
    fn test_render_shows_title() {
        test_frame!(pool, frame, 60, 8);
        let area = Rect::new(0, 0, 60, 8);
        let mut state = SessionListState::new();
        render(&mut frame, area, &[], &[], &mut state, false, false, 0);
        assert_text_present(&frame.buffer, "Sessions");
    }

    #[test]
    fn test_render_expanded_shows_pane_sub_rows() {
        test_frame!(pool, frame, 80, 12);
        let area = Rect::new(0, 0, 80, 12);
        let sessions = vec![make_session("s1", "my-proj", "active")];
        let panes = vec![
            make_pane("p1", "s1", "active"),
            make_pane("p2", "s1", "idle"),
        ];
        let mut state = SessionListState::new();
        state.expanded_index = Some(0);
        render(&mut frame, area, &sessions, &panes, &mut state, true, true, 0);
        // Should show tree guide chars
        let lines = buf_to_lines(&frame.buffer);
        let has_tree = lines.iter().any(|l| l.contains("├") || l.contains("└"));
        assert!(has_tree, "Missing tree guides in: {lines:?}");
    }
}
