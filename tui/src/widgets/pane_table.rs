use crate::rpc::types::PaneView;
use crate::theme;
use ftui::core::geometry::Rect;
use ftui::layout::Constraint;
use ftui::render::frame::Frame;
use ftui::Style;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::table::{Row, Table, TableState};
use ftui::widgets::{StatefulWidget, Widget};

pub struct PaneTableState {
    pub table_state: TableState,
}

impl PaneTableState {
    pub fn new() -> Self {
        Self {
            table_state: TableState::default(),
        }
    }

    pub fn selected(&self) -> Option<usize> {
        self.table_state.selected
    }

    pub fn select_next(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        let i = self.table_state.selected.unwrap_or(0);
        self.table_state.select(Some((i + 1).min(len - 1)));
    }

    pub fn select_prev(&mut self) {
        let i = self.table_state.selected.unwrap_or(0);
        self.table_state.select(Some(i.saturating_sub(1)));
    }
}

/// Render the pane detail table for a single session using FrankenTUI Table widget.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    panes: &[PaneView],
    session_name: &str,
    state: &mut PaneTableState,
    focused: bool,
) {
    let title: &str = Box::leak(format!(" Panes ({session_name}) ").into_boxed_str());
    let block = theme::panel_block(title, focused);

    if panes.is_empty() {
        let empty = Paragraph::new("  No panes")
            .style(theme::muted_style())
            .block(block);
        empty.render(area, frame);
        return;
    }

    let header = Row::new(["#", "Agent", "Status", "Command/Waiting", "Activity"])
        .style(Style::new().fg(theme::TEXT_MUTED));

    let rows: Vec<Row> = panes
        .iter()
        .map(|pane| {
            let idx = format!("#{}", pane.pane_index);
            let agent = theme::agent_label(pane.agent_type.as_deref().unwrap_or("--"));
            let badge = theme::status_badge(&pane.status);
            let status_text = format!("{badge} {}", pane.status);
            let color = theme::status_color(&pane.status);

            // Command/Waiting column — KEY FEATURE
            let cmd_text = match pane.status.as_str() {
                "waiting" | "paused" => {
                    pane.status_reason.as_deref().unwrap_or("waiting...").to_string()
                }
                "active" => {
                    pane.current_command.as_deref().unwrap_or("--").to_string()
                }
                "idle" => {
                    if let Some(ts) = pane.last_activity_at {
                        format!("idle {}", theme::relative_time(ts))
                    } else {
                        "--".to_string()
                    }
                }
                _ => pane.current_command.as_deref().unwrap_or("--").to_string(),
            };

            let activity = if let Some(ts) = pane.last_activity_at {
                theme::relative_time(ts)
            } else {
                "--".to_string()
            };

            Row::new([idx, agent.to_string(), status_text, cmd_text, activity])
                .style(Style::new().fg(color))
        })
        .collect();

    let widths = [
        Constraint::Fixed(4),
        Constraint::Fixed(8),
        Constraint::Fixed(14),
        Constraint::Min(16),
        Constraint::Fixed(8),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .highlight_style(theme::highlight_style())
        .column_spacing(1);

    StatefulWidget::render(&table, area, frame, &mut state.table_state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_default_selection() {
        let state = PaneTableState::new();
        assert_eq!(state.table_state.selected, None);
    }

    #[test]
    fn test_select_next_increments() {
        let mut state = PaneTableState::new();
        state.table_state.select(Some(0));
        state.select_next(5);
        assert_eq!(state.table_state.selected, Some(1));
    }

    #[test]
    fn test_select_next_clamps() {
        let mut state = PaneTableState::new();
        state.table_state.select(Some(4));
        state.select_next(5);
        assert_eq!(state.table_state.selected, Some(4));
    }

    #[test]
    fn test_select_next_empty() {
        let mut state = PaneTableState::new();
        state.select_next(0); // no panic
    }

    #[test]
    fn test_select_prev_decrements() {
        let mut state = PaneTableState::new();
        state.table_state.select(Some(3));
        state.select_prev();
        assert_eq!(state.table_state.selected, Some(2));
    }

    #[test]
    fn test_select_prev_saturates_at_zero() {
        let mut state = PaneTableState::new();
        state.table_state.select(Some(0));
        state.select_prev();
        assert_eq!(state.table_state.selected, Some(0));
    }

    #[test]
    fn test_selected_accessor() {
        let mut state = PaneTableState::new();
        assert_eq!(state.selected(), None);
        state.table_state.select(Some(2));
        assert_eq!(state.selected(), Some(2));
    }

    // === Render tests ===

    use crate::rpc::types::PaneView;
    use crate::test_helpers::*;
    use ftui::core::geometry::Rect;

    fn make_pane(id: &str, session_id: &str, status: &str, cmd: Option<&str>) -> PaneView {
        PaneView {
            pane_id: id.to_string(),
            session_id: session_id.to_string(),
            status: status.to_string(),
            pane_index: 0,
            current_command: cmd.map(|c| c.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn test_render_empty_shows_no_panes() {
        test_frame!(pool, frame, 60, 8);
        let area = Rect::new(0, 0, 60, 8);
        let mut state = PaneTableState::new();
        render(&mut frame, area, &[], "test-session", &mut state, false);
        assert_text_present(&frame.buffer, "No panes");
    }

    #[test]
    fn test_render_title_includes_session_name() {
        test_frame!(pool, frame, 60, 8);
        let area = Rect::new(0, 0, 60, 8);
        let mut state = PaneTableState::new();
        render(&mut frame, area, &[], "my-project", &mut state, false);
        assert_text_present(&frame.buffer, "Panes (my-project)");
    }

    #[test]
    fn test_render_with_panes_shows_header() {
        test_frame!(pool, frame, 80, 10);
        let area = Rect::new(0, 0, 80, 10);
        let panes = vec![make_pane("p1", "s1", "active", Some("vim"))];
        let mut state = PaneTableState::new();
        render(&mut frame, area, &panes, "dev", &mut state, true);
        assert_text_present(&frame.buffer, "Agent");
        assert_text_present(&frame.buffer, "Status");
    }

    #[test]
    fn test_render_shows_pane_status() {
        test_frame!(pool, frame, 80, 10);
        let area = Rect::new(0, 0, 80, 10);
        let panes = vec![
            make_pane("p1", "s1", "active", Some("cargo test")),
            make_pane("p2", "s1", "idle", None),
        ];
        let mut state = PaneTableState::new();
        render(&mut frame, area, &panes, "dev", &mut state, false);
        assert_text_present(&frame.buffer, "active");
    }
}
