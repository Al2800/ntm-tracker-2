use crate::app::NtmApp;
use crate::theme;
use ftui::core::geometry::Rect;
use ftui::layout::{Constraint, Flex};
use ftui::render::frame::Frame;
use ftui::Style;
use ftui::widgets::block::Block;
use ftui::widgets::borders::Borders;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::Widget;

/// Render the health/diagnostics screen.
pub fn render(frame: &mut Frame, area: Rect, app: &NtmApp) {
    let rows = Flex::vertical()
        .constraints([
            Constraint::Fixed(5),  // connection status
            Constraint::Fixed(7),  // stats
            Constraint::Min(4),    // daemon info
        ])
        .split(area);

    // Connection status
    render_connection_card(frame, rows[0], app);

    // Stats summary
    render_stats_card(frame, rows[1], app);

    // Daemon info
    render_daemon_card(frame, rows[2], app);
}

fn render_connection_card(frame: &mut Frame, area: Rect, app: &NtmApp) {
    let (icon, color) = match &app.conn_state {
        crate::msg::ConnState::Connected => ("●", theme::ACTIVE),
        crate::msg::ConnState::Connecting => ("◌", theme::IDLE),
        crate::msg::ConnState::Disconnected => ("○", theme::TEXT_MUTED),
        crate::msg::ConnState::Error(_) => ("✕", theme::ERROR),
    };

    let text = format!(
        "  {icon} Connection: {}\n  Daemon version: {}",
        app.conn_state.label(),
        app.daemon_version,
    );

    let block = Block::new()
        .title(" Connection ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(color))
        .style(theme::raised_style());

    let para = Paragraph::new(text)
        .style(Style::new().fg(color).bg(theme::BG_RAISED))
        .block(block);

    para.render(area, frame);
}

fn render_stats_card(frame: &mut Frame, area: Rect, app: &NtmApp) {
    let s = &app.stats;
    let text = format!(
        "  Sessions: {}    Panes: {}\n  Compacts: {}    Active: {}m\n  Est. Tokens: {}",
        s.sessions, s.panes, s.total_compacts, s.active_minutes, s.estimated_tokens,
    );

    let block = Block::new()
        .title(" Statistics ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(theme::BG_SURFACE))
        .style(theme::raised_style());

    let para = Paragraph::new(text)
        .style(Style::new().fg(theme::TEXT_PRIMARY).bg(theme::BG_RAISED))
        .block(block);

    para.render(area, frame);
}

fn render_daemon_card(frame: &mut Frame, area: Rect, app: &NtmApp) {
    let event_count = app.events.len();
    let last_id = app.last_event_id;

    let text = format!(
        "  Events in cache: {event_count}\n  Last event ID: {last_id}",
    );

    let block = Block::new()
        .title(" Daemon Info ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(theme::BG_SURFACE))
        .style(theme::raised_style());

    let para = Paragraph::new(text)
        .style(Style::new().fg(theme::TEXT_SECONDARY).bg(theme::BG_RAISED))
        .block(block);

    para.render(area, frame);
}
