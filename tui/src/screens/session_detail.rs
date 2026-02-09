use crate::app::NtmApp;
use crate::msg::EventFilter;
use crate::theme;
use crate::widgets::{event_timeline, pane_table};
use ftui::core::geometry::Rect;
use ftui::layout::{Constraint, Flex};
use ftui::render::frame::Frame;
use ftui::Style;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::Widget;

/// Render the comprehensive sessions screen.
pub fn render(frame: &mut Frame, area: Rect, app: &NtmApp) {
    if app.sessions.is_empty() {
        let block = theme::panel_block(" Sessions ", true);
        let empty = Paragraph::new("  No active sessions")
            .style(theme::muted_style())
            .block(block);
        empty.render(area, frame);
        return;
    }

    let rows = Flex::vertical()
        .constraints([
            Constraint::Fixed(3), // session info header
            Constraint::Min(5),   // pane table
            Constraint::Fixed(7), // session events
        ])
        .split(area);

    // Session info header
    let selected = app
        .session_list_state
        .borrow()
        .selected_session_index()
        .and_then(|i| app.sessions.get(i).cloned());

    if let Some(session) = selected {
        let badge = theme::status_badge(&session.status);
        let color = theme::status_color(&session.status);
        let rel_time = theme::relative_time(session.last_seen_at);
        let info = format!(
            "  {badge} {}  │  Status: {}  │  Panes: {}  │  Source: {}  │  Last: {rel_time}",
            session.name, session.status, session.pane_count, session.source_id
        );

        let block = theme::panel_block(" Session Detail ", true);
        let para = Paragraph::new(info)
            .style(Style::new().fg(color).bg(theme::BG_RAISED))
            .block(block);
        para.render(rows[0], frame);

        // Pane table
        let session_panes: Vec<_> = app
            .panes
            .iter()
            .filter(|p| p.session_id == session.session_id)
            .cloned()
            .collect();

        pane_table::render(
            frame,
            rows[1],
            &session_panes,
            &session.name,
            &mut app.pane_table_state.borrow_mut(),
            true,
        );

        // Events filtered to this session
        let session_events: Vec<_> = app
            .events
            .iter()
            .filter(|e| e.session_id == session.session_id)
            .cloned()
            .collect();

        event_timeline::render(
            frame,
            rows[2],
            &session_events,
            &mut app.event_timeline_state.borrow_mut(),
            false,
            EventFilter::All,
        );
    } else {
        let block = theme::panel_block(" Session Detail ", false);
        let para = Paragraph::new("  \u{2190} Select a session from the list")
            .style(theme::muted_style())
            .block(block);
        para.render(area, frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::NtmApp;
    use crate::rpc::types::{PaneView, SessionView};
    use crate::test_helpers::*;
    use crate::widgets::session_list::RowKind;
    use ftui::core::geometry::Rect;

    /// Set up session_list_state so selected_session_index() returns Some(idx).
    fn select_session(app: &NtmApp, idx: usize) {
        let mut state = app.session_list_state.borrow_mut();
        state.row_map = vec![RowKind::Session(idx)];
        state.list_state.select(Some(0));
    }

    #[test]
    fn test_render_empty_sessions_shows_no_active() {
        test_frame!(pool, frame, 80, 20);
        let area = Rect::new(0, 0, 80, 20);
        let app = NtmApp::new();
        render(&mut frame, area, &app);
        assert_text_present(&frame.buffer, "No active sessions");
    }

    #[test]
    fn test_render_empty_sessions_shows_sessions_title() {
        test_frame!(pool, frame, 80, 20);
        let area = Rect::new(0, 0, 80, 20);
        let app = NtmApp::new();
        render(&mut frame, area, &app);
        assert_text_present(&frame.buffer, "Sessions");
    }

    #[test]
    fn test_render_sessions_no_selection_shows_select_hint() {
        test_frame!(pool, frame, 80, 20);
        let area = Rect::new(0, 0, 80, 20);
        let mut app = NtmApp::new();
        app.sessions = vec![SessionView {
            session_id: "s1".to_string(),
            name: "my-session".to_string(),
            status: "active".to_string(),
            pane_count: 1,
            source_id: "tmux".to_string(),
            ..Default::default()
        }];
        // No session selected in session_list_state → shows "Select a session" hint
        render(&mut frame, area, &app);
        assert_text_present(&frame.buffer, "Select a session");
    }

    #[test]
    fn test_render_with_selected_session_shows_detail() {
        test_frame!(pool, frame, 100, 25);
        let area = Rect::new(0, 0, 100, 25);
        let mut app = NtmApp::new();
        app.sessions = vec![SessionView {
            session_id: "s1".to_string(),
            name: "work-session".to_string(),
            status: "active".to_string(),
            pane_count: 2,
            source_id: "tmux".to_string(),
            ..Default::default()
        }];
        app.panes = vec![PaneView {
            pane_id: "p1".to_string(),
            session_id: "s1".to_string(),
            status: "active".to_string(),
            ..Default::default()
        }];
        select_session(&app, 0);
        render(&mut frame, area, &app);
        assert_text_present(&frame.buffer, "Session Detail");
        assert_text_present(&frame.buffer, "work-session");
    }

    #[test]
    fn test_render_selected_session_shows_status() {
        test_frame!(pool, frame, 100, 25);
        let area = Rect::new(0, 0, 100, 25);
        let mut app = NtmApp::new();
        app.sessions = vec![SessionView {
            session_id: "s1".to_string(),
            name: "test-sess".to_string(),
            status: "active".to_string(),
            pane_count: 1,
            source_id: "tmux".to_string(),
            ..Default::default()
        }];
        select_session(&app, 0);
        render(&mut frame, area, &app);
        assert_text_present(&frame.buffer, "active");
    }
}
