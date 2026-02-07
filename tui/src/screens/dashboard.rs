use crate::app::NtmApp;
use crate::msg::FocusArea;
use crate::widgets::{
    activity_spark, escalation_inbox, event_timeline, overview_cards, pane_table, session_list,
};
use ftui::core::geometry::Rect;
use ftui::layout::{Constraint, Flex};
use ftui::render::frame::Frame;

/// Render the main dashboard screen.
pub fn render(frame: &mut Frame, area: Rect, app: &NtmApp) {
    // Vertical layout: overview cards | middle section | event timeline
    let rows = Flex::vertical()
        .constraints([
            Constraint::Fixed(4),  // overview cards
            Constraint::Min(8),    // middle (sessions + sidebar)
            Constraint::Fixed(7),  // recent events
        ])
        .split(area);

    // Overview cards row
    overview_cards::render(frame, rows[0], &app.stats);

    // Middle section: sessions (left) | sidebar (right)
    let mid_cols = Flex::horizontal()
        .constraints([
            Constraint::Ratio(1, 2), // sessions + panes
            Constraint::Ratio(1, 2), // activity + escalations
        ])
        .split(rows[1]);

    // Left column: session list + pane table
    let left_rows = Flex::vertical()
        .constraints([
            Constraint::Ratio(1, 2),
            Constraint::Ratio(1, 2),
        ])
        .split(mid_cols[0]);

    session_list::render(
        frame,
        left_rows[0],
        &app.sessions,
        &mut app.session_list_state.borrow_mut(),
        app.focus == FocusArea::SessionList,
    );

    // Show panes for selected session
    let selected_session = app
        .session_list_state
        .borrow()
        .selected()
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
        left_rows[1],
        &session_panes,
        session_name,
        &mut app.pane_table_state.borrow_mut(),
        app.focus == FocusArea::PaneTable,
    );

    // Right column: activity sparkline + escalations
    let right_rows = Flex::vertical()
        .constraints([
            Constraint::Fixed(5),  // sparkline
            Constraint::Min(4),    // escalations
        ])
        .split(mid_cols[1]);

    activity_spark::render(frame, right_rows[0], &app.events);

    let escalations: Vec<_> = app
        .events
        .iter()
        .filter(|e| e.event_type == "escalation")
        .cloned()
        .collect();

    escalation_inbox::render(
        frame,
        right_rows[1],
        &escalations,
        &mut app.escalation_state.borrow_mut(),
        app.focus == FocusArea::EscalationInbox,
    );

    // Bottom row: recent events
    event_timeline::render(
        frame,
        rows[2],
        &app.events,
        &mut app.event_timeline_state.borrow_mut(),
        app.focus == FocusArea::EventTimeline,
    );
}
