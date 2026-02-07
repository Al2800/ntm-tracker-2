use crate::rpc::types::EventView;
use crate::theme;
use ftui::core::geometry::Rect;
use ftui::render::frame::Frame;
use ftui::PackedRgba;
use ftui::Style;
use ftui::widgets::block::Block;
use ftui::widgets::borders::Borders;
use ftui::widgets::list::{List, ListItem, ListState};
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::{StatefulWidget, Widget};

pub struct EventTimelineState {
    pub list_state: ListState,
}

impl EventTimelineState {
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

/// Render recent events.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    events: &[EventView],
    state: &mut EventTimelineState,
    focused: bool,
) {
    let border_color = if focused {
        theme::INFO
    } else {
        theme::BG_SURFACE
    };

    let block = Block::new()
        .title(" Recent Events ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(border_color))
        .style(theme::raised_style());

    if events.is_empty() {
        let empty = Paragraph::new("  No events yet")
            .style(theme::muted_style())
            .block(block);
        empty.render(area, frame);
        return;
    }

    let items: Vec<ListItem> = events
        .iter()
        .rev()
        .take(50)
        .map(|ev| {
            let time = format_timestamp(ev.detected_at);
            let color = event_type_color(&ev.event_type);
            let session = truncate_id(&ev.session_id, 12);
            let pane = truncate_id(&ev.pane_id, 8);
            let line = format!(
                " {time}  {etype:<12} {session}:{pane}",
                etype = ev.event_type,
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

pub(crate) fn event_type_color(event_type: &str) -> PackedRgba {
    match event_type {
        "escalation" => theme::ERROR,
        "compact" => theme::ACCENT,
        "session_start" | "session_end" => theme::INFO,
        _ => theme::TEXT_SECONDARY,
    }
}

pub(crate) fn format_timestamp(ts: i64) -> String {
    use chrono::prelude::*;
    let dt = chrono::DateTime::from_timestamp(ts, 0)
        .unwrap_or_else(|| Utc::now());
    dt.with_timezone(&Local).format("%H:%M").to_string()
}

pub(crate) fn truncate_id(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === EventTimelineState navigation tests (bd-283) ===

    #[test]
    fn test_new_default_selection() {
        let state = EventTimelineState::new();
        assert_eq!(state.list_state.selected(), None);
    }

    #[test]
    fn test_select_next_increments() {
        let mut state = EventTimelineState::new();
        state.list_state.select(Some(0));
        state.select_next(5);
        assert_eq!(state.list_state.selected(), Some(1));
    }

    #[test]
    fn test_select_next_clamps() {
        let mut state = EventTimelineState::new();
        state.list_state.select(Some(4));
        state.select_next(5);
        assert_eq!(state.list_state.selected(), Some(4));
    }

    #[test]
    fn test_select_next_empty() {
        let mut state = EventTimelineState::new();
        state.select_next(0); // no panic
    }

    #[test]
    fn test_select_prev_decrements() {
        let mut state = EventTimelineState::new();
        state.list_state.select(Some(3));
        state.select_prev();
        assert_eq!(state.list_state.selected(), Some(2));
    }

    #[test]
    fn test_select_prev_saturates() {
        let mut state = EventTimelineState::new();
        state.list_state.select(Some(0));
        state.select_prev();
        assert_eq!(state.list_state.selected(), Some(0));
    }

    // === Event type color mapping tests (bd-3gfm) ===

    #[test]
    fn test_escalation_returns_error_color() {
        assert_eq!(event_type_color("escalation"), theme::ERROR);
    }

    #[test]
    fn test_compact_returns_accent_color() {
        assert_eq!(event_type_color("compact"), theme::ACCENT);
    }

    #[test]
    fn test_session_start_returns_info_color() {
        assert_eq!(event_type_color("session_start"), theme::INFO);
    }

    #[test]
    fn test_session_end_returns_info_color() {
        assert_eq!(event_type_color("session_end"), theme::INFO);
    }

    #[test]
    fn test_unknown_type_returns_secondary_color() {
        assert_eq!(event_type_color("something_else"), theme::TEXT_SECONDARY);
    }

    #[test]
    fn test_empty_string_returns_secondary_color() {
        assert_eq!(event_type_color(""), theme::TEXT_SECONDARY);
    }

    // === Timestamp formatting tests (bd-zd7b partial) ===

    #[test]
    fn test_format_timestamp_valid() {
        // 2024-01-15 12:30:00 UTC = 1705318200
        let result = format_timestamp(1705318200);
        // Should produce HH:MM format
        assert!(result.len() == 5, "Expected HH:MM format, got: {result}");
        assert!(result.contains(':'), "Expected colon in time, got: {result}");
    }

    #[test]
    fn test_format_timestamp_zero() {
        let result = format_timestamp(0);
        assert!(result.contains(':'));
    }

    #[test]
    fn test_format_timestamp_output_format() {
        let result = format_timestamp(1700000000);
        let parts: Vec<&str> = result.split(':').collect();
        assert_eq!(parts.len(), 2);
        assert!(parts[0].len() == 2);
        assert!(parts[1].len() == 2);
    }

    // === Truncate ID tests (bd-2x0m partial) ===

    #[test]
    fn test_truncate_id_short() {
        assert_eq!(truncate_id("abc", 10), "abc");
    }

    #[test]
    fn test_truncate_id_exact() {
        assert_eq!(truncate_id("abcde", 5), "abcde");
    }

    #[test]
    fn test_truncate_id_long() {
        let result = truncate_id("abcdefghij", 5);
        assert!(result.contains('…'));
    }
}
