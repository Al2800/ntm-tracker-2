use crate::rpc::types::EventView;
use crate::theme;
use ftui::core::geometry::Rect;
use ftui::render::frame::Frame;
use ftui::Style;
use ftui::widgets::block::Block;
use ftui::widgets::borders::Borders;
use ftui::widgets::list::{List, ListItem, ListState};
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::{StatefulWidget, Widget};

pub struct EscalationInboxState {
    pub list_state: ListState,
}

impl EscalationInboxState {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
        }
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
}

/// Render escalation alerts.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    escalations: &[EventView],
    state: &mut EscalationInboxState,
    focused: bool,
) {
    let border_color = if focused {
        theme::INFO
    } else {
        theme::BG_SURFACE
    };

    let count = escalations.len();
    let title_str: &str = if count > 0 {
        // Leak a small string for the title — these are ephemeral per frame
        Box::leak(format!(" Escalations ({count}) ").into_boxed_str())
    } else {
        " Escalations "
    };

    let block = Block::new()
        .title(title_str)
        .borders(Borders::ALL)
        .border_style(Style::new().fg(if count > 0 {
            theme::ERROR
        } else {
            border_color
        }))
        .style(theme::raised_style());

    if escalations.is_empty() {
        let empty = Paragraph::new("  No escalations")
            .style(theme::muted_style())
            .block(block);
        empty.render(area, frame);
        return;
    }

    let items: Vec<ListItem> = escalations
        .iter()
        .map(|e| {
            let time = format_relative(e.detected_at);
            let line = format!(
                " ! {sess}:{pane}  {time}   [d]ismiss [f]ocus",
                sess = truncate(&e.session_id, 10),
                pane = truncate(&e.pane_id, 6),
            );
            ListItem::new(line).style(Style::new().fg(theme::ERROR))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(theme::highlight_style())
        .highlight_symbol(">> ");

    StatefulWidget::render(&list, area, frame, &mut state.list_state);
}

pub(crate) fn format_relative(ts: i64) -> String {
    let now = chrono::Utc::now().timestamp();
    let delta = now - ts;
    if delta < 60 {
        format!("{delta}s ago")
    } else if delta < 3600 {
        format!("{}m ago", delta / 60)
    } else {
        format!("{}h ago", delta / 3600)
    }
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

    // === EscalationInboxState navigation tests (bd-2qbg) ===

    #[test]
    fn test_new_default_selection() {
        let state = EscalationInboxState::new();
        assert_eq!(state.list_state.selected(), None);
    }

    #[test]
    fn test_select_next_increments() {
        let mut state = EscalationInboxState::new();
        state.list_state.select(Some(0));
        state.select_next(5);
        assert_eq!(state.list_state.selected(), Some(1));
    }

    #[test]
    fn test_select_next_clamps() {
        let mut state = EscalationInboxState::new();
        state.list_state.select(Some(4));
        state.select_next(5);
        assert_eq!(state.list_state.selected(), Some(4));
    }

    #[test]
    fn test_select_next_empty() {
        let mut state = EscalationInboxState::new();
        state.select_next(0);
    }

    #[test]
    fn test_select_prev_decrements() {
        let mut state = EscalationInboxState::new();
        state.list_state.select(Some(3));
        state.select_prev();
        assert_eq!(state.list_state.selected(), Some(2));
    }

    #[test]
    fn test_select_prev_saturates() {
        let mut state = EscalationInboxState::new();
        state.list_state.select(Some(0));
        state.select_prev();
        assert_eq!(state.list_state.selected(), Some(0));
    }

    // === Relative time formatting tests (bd-zd7b partial) ===

    #[test]
    fn test_format_relative_seconds() {
        let now = chrono::Utc::now().timestamp();
        let result = format_relative(now - 30);
        assert!(result.contains("s ago"), "Expected seconds, got: {result}");
    }

    #[test]
    fn test_format_relative_minutes() {
        let now = chrono::Utc::now().timestamp();
        let result = format_relative(now - 120);
        assert!(result.contains("m ago"), "Expected minutes, got: {result}");
    }

    #[test]
    fn test_format_relative_hours() {
        let now = chrono::Utc::now().timestamp();
        let result = format_relative(now - 7200);
        assert!(result.contains("h ago"), "Expected hours, got: {result}");
    }

    #[test]
    fn test_format_relative_boundary_60s() {
        let now = chrono::Utc::now().timestamp();
        let result = format_relative(now - 60);
        assert!(result.contains("m ago"), "At 60s should show minutes, got: {result}");
    }

    #[test]
    fn test_format_relative_boundary_3600s() {
        let now = chrono::Utc::now().timestamp();
        let result = format_relative(now - 3600);
        assert!(result.contains("h ago"), "At 3600s should show hours, got: {result}");
    }

    // === Truncate tests (bd-2x0m partial) ===

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("abc", 10), "abc");
    }

    #[test]
    fn test_truncate_long() {
        let result = truncate("abcdefghijklmnop", 5);
        assert!(result.contains('…'));
    }
}
