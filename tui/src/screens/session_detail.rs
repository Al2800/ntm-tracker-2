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
        .selected()
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
        let para = Paragraph::new("  Select a session from Dashboard")
            .style(theme::muted_style())
            .block(block);
        para.render(area, frame);
    }
}
