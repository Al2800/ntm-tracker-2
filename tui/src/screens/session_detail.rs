use crate::app::NtmApp;
use crate::theme;
use crate::widgets::pane_table;
use ftui::core::geometry::Rect;
use ftui::layout::{Constraint, Flex};
use ftui::render::frame::Frame;
use ftui::Style;
use ftui::widgets::block::Block;
use ftui::widgets::borders::Borders;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::Widget;

/// Render the full sessions screen (all sessions with their panes).
pub fn render(frame: &mut Frame, area: Rect, app: &NtmApp) {
    if app.sessions.is_empty() {
        let block = Block::new()
            .title(" Sessions ")
            .borders(Borders::ALL)
            .border_style(Style::new().fg(theme::BG_SURFACE))
            .style(theme::raised_style());
        let empty = Paragraph::new("  No active sessions")
            .style(theme::muted_style())
            .block(block);
        empty.render(area, frame);
        return;
    }

    let rows = Flex::vertical()
        .constraints([
            Constraint::Fixed(3), // session info
            Constraint::Min(5),   // pane table
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
        let info = format!(
            "  {badge} {}  │  Status: {}  │  Panes: {}  │  Source: {}",
            session.name, session.status, session.pane_count, session.source_id
        );
        let color = theme::status_color(&session.status);
        let block = Block::new()
            .title(" Session Detail ")
            .borders(Borders::ALL)
            .border_style(Style::new().fg(theme::INFO))
            .style(theme::raised_style());
        let para = Paragraph::new(info)
            .style(Style::new().fg(color).bg(theme::BG_RAISED))
            .block(block);
        para.render(rows[0], frame);

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
    } else {
        let block = Block::new()
            .title(" Session Detail ")
            .borders(Borders::ALL)
            .border_style(Style::new().fg(theme::BG_SURFACE))
            .style(theme::raised_style());
        let para = Paragraph::new("  Select a session from Dashboard")
            .style(theme::muted_style())
            .block(block);
        para.render(area, frame);
    }
}
