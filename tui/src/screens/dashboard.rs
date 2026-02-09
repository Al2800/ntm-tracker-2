use crate::app::NtmApp;
use crate::msg::{ConnState, EventFilter, FocusArea};
use crate::widgets::{
    activity_spark, escalation_inbox, event_timeline, overview_cards, pane_table, session_list,
};
use ftui::core::geometry::Rect;
use ftui::layout::{Constraint, Flex};
use ftui::render::frame::Frame;

/// Render the main dashboard screen with 3-zone layout.
pub fn render(frame: &mut Frame, area: Rect, app: &NtmApp) {
    // Vertical layout: overview | main | bottom
    let rows = Flex::vertical()
        .constraints([
            Constraint::Fixed(4),  // overview cards
            Constraint::Min(8),    // main (sessions + panes)
            Constraint::Fixed(5),  // sparkline + escalations
            Constraint::Fixed(7),  // recent events
        ])
        .split(area);

    // Overview cards row
    overview_cards::render(frame, rows[0], &app.stats);

    // Main section: sessions (left) | panes (right)
    let main_cols = Flex::horizontal()
        .constraints([
            Constraint::Ratio(2, 5), // sessions
            Constraint::Ratio(3, 5), // panes
        ])
        .split(rows[1]);

    session_list::render(
        frame,
        main_cols[0],
        &app.sessions,
        &app.panes,
        &mut app.session_list_state.borrow_mut(),
        app.focus == FocusArea::SessionList,
        app.conn_state == ConnState::Connected,
        app.spinner_frame,
    );

    // Show panes for selected session
    let selected_session = app
        .session_list_state
        .borrow()
        .selected_session_index()
        .and_then(|i| app.sessions.get(i).map(|s| (s.name.clone(), s.session_id.clone())));

    let (session_name, session_panes) = if let Some((name, sid)) = &selected_session {
        let panes: Vec<_> = app.panes
            .iter()
            .filter(|p| p.session_id == *sid)
            .cloned()
            .collect();
        (name.as_str(), panes)
    } else {
        ("--", vec![])
    };

    pane_table::render(
        frame,
        main_cols[1],
        &session_panes,
        session_name,
        &mut app.pane_table_state.borrow_mut(),
        app.focus == FocusArea::PaneTable,
    );

    // Bottom row: sparkline (left) + escalations (right)
    let bottom_cols = Flex::horizontal()
        .constraints([
            Constraint::Ratio(2, 5), // sparkline
            Constraint::Ratio(3, 5), // escalations
        ])
        .split(rows[2]);

    activity_spark::render(frame, bottom_cols[0], &app.events, false);

    let escalations: Vec<_> = app
        .events
        .iter()
        .filter(|e| e.event_type == "escalation")
        .cloned()
        .collect();

    escalation_inbox::render(
        frame,
        bottom_cols[1],
        &escalations,
        &mut app.escalation_state.borrow_mut(),
        app.focus == FocusArea::EscalationInbox,
    );

    // Events row
    event_timeline::render(
        frame,
        rows[3],
        &app.events,
        &mut app.event_timeline_state.borrow_mut(),
        app.focus == FocusArea::EventTimeline,
        EventFilter::All,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::NtmApp;
    use crate::rpc::types::{EventView, PaneView, SessionView, StatsSummary};
    use crate::test_helpers::*;
    use ftui::core::geometry::Rect;

    fn populated_app() -> NtmApp {
        let mut app = NtmApp::new();
        app.conn_state = crate::msg::ConnState::Connected;
        app.stats = StatsSummary {
            sessions: 2,
            panes: 4,
            total_compacts: 10,
            active_minutes: 60,
            estimated_tokens: 50_000,
        };
        app.sessions = vec![SessionView {
            session_id: "s1".to_string(),
            name: "dev-session".to_string(),
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
        app.events = vec![EventView {
            event_type: "escalation".to_string(),
            session_id: "s1".to_string(),
            pane_id: "p1".to_string(),
            detected_at: chrono::Utc::now().timestamp(),
            ..Default::default()
        }];
        app
    }

    #[test]
    fn test_render_empty_app_no_panic() {
        test_frame!(pool, frame, 100, 30);
        let area = Rect::new(0, 0, 100, 30);
        let app = NtmApp::new();
        render(&mut frame, area, &app);
        // Overview cards still render with zero stats
        assert_text_present(&frame.buffer, "Sessions");
    }

    #[test]
    fn test_render_shows_overview_cards() {
        test_frame!(pool, frame, 100, 30);
        let area = Rect::new(0, 0, 100, 30);
        let app = populated_app();
        render(&mut frame, area, &app);
        assert_text_present(&frame.buffer, "Panes");
        assert_text_present(&frame.buffer, "Compacts");
    }

    #[test]
    fn test_render_shows_session_list() {
        test_frame!(pool, frame, 100, 30);
        let area = Rect::new(0, 0, 100, 30);
        let app = populated_app();
        render(&mut frame, area, &app);
        assert_text_present(&frame.buffer, "dev-session");
    }

    #[test]
    fn test_render_shows_event_timeline() {
        test_frame!(pool, frame, 100, 30);
        let area = Rect::new(0, 0, 100, 30);
        let app = populated_app();
        render(&mut frame, area, &app);
        assert_text_present(&frame.buffer, "escalation");
    }

    #[test]
    fn test_render_shows_escalation_inbox() {
        test_frame!(pool, frame, 100, 30);
        let area = Rect::new(0, 0, 100, 30);
        let app = populated_app();
        render(&mut frame, area, &app);
        assert_text_present(&frame.buffer, "Escalations");
    }
}
