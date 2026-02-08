use crate::app::NtmApp;
use crate::msg::{EventFilter, FocusArea};
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
