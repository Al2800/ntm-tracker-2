use crate::app::NtmApp;
use crate::theme;
use crate::widgets::overview_cards;
use ftui::core::geometry::Rect;
use ftui::layout::{Constraint, Flex};
use ftui::render::frame::Frame;
use ftui::Style;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::Widget;

/// Render the health/diagnostics screen.
pub fn render(frame: &mut Frame, area: Rect, app: &NtmApp) {
    let rows = Flex::vertical()
        .constraints([
            Constraint::Fixed(5),  // connection + daemon side by side
            Constraint::Fixed(7),  // stats
            Constraint::Min(4),    // daemon info
        ])
        .split(area);

    // Top row: connection + daemon info side by side
    let top_cols = Flex::horizontal()
        .constraints([
            Constraint::Ratio(1, 2),
            Constraint::Ratio(1, 2),
        ])
        .split(rows[0]);

    render_connection_card(frame, top_cols[0], app);
    render_daemon_card(frame, top_cols[1], app);

    // Stats summary
    render_stats_card(frame, rows[1], app);

    // Cache info
    render_cache_card(frame, rows[2], app);
}

fn render_connection_card(frame: &mut Frame, area: Rect, app: &NtmApp) {
    let (icon, color) = match &app.conn_state {
        crate::msg::ConnState::Connected => ("●", theme::ACTIVE),
        crate::msg::ConnState::Connecting => ("◌", theme::IDLE),
        crate::msg::ConnState::Disconnected => ("○", theme::TEXT_MUTED),
        crate::msg::ConnState::Error(_) => ("✕", theme::ERROR),
    };

    let error_line = if let crate::msg::ConnState::Error(e) = &app.conn_state {
        format!("\n  Last error: {e}")
    } else {
        "\n  Last error: none".to_string()
    };

    let text = format!(
        "  {icon} {status}\n  Version: {version}{error_line}",
        status = app.conn_state.label(),
        version = app.daemon_version,
    );

    let block = theme::panel_block(" Connection ", true);
    let para = Paragraph::new(text)
        .style(Style::new().fg(color).bg(theme::BG_RAISED))
        .block(block);

    para.render(area, frame);
}

fn render_daemon_card(frame: &mut Frame, area: Rect, app: &NtmApp) {
    let text = format!(
        "  Protocol: JSON-RPC 2.0\n  Transport: stdio\n  Sessions: {}",
        app.sessions.len(),
    );

    let block = theme::panel_block(" Daemon ", false);
    let para = Paragraph::new(text)
        .style(Style::new().fg(theme::TEXT_PRIMARY).bg(theme::BG_RAISED))
        .block(block);

    para.render(area, frame);
}

fn render_stats_card(frame: &mut Frame, area: Rect, app: &NtmApp) {
    let s = &app.stats;
    let tokens = theme::format_tokens(s.estimated_tokens);
    let active = overview_cards::format_active_time(s.active_minutes);
    let text = format!(
        "  Sessions: {}    Panes: {}\n  Compacts: {}    Active: {active}\n  Est. Tokens: {tokens}",
        s.sessions, s.panes, s.total_compacts,
    );

    let block = theme::panel_block(" Statistics ", false);
    let para = Paragraph::new(text)
        .style(Style::new().fg(theme::TEXT_PRIMARY).bg(theme::BG_RAISED))
        .block(block);

    para.render(area, frame);
}

fn render_cache_card(frame: &mut Frame, area: Rect, app: &NtmApp) {
    let event_count = app.events.len();
    let last_id = app.last_event_id;
    let escalation_count = app.events.iter().filter(|e| e.event_type == "escalation").count();
    let compact_count = app.events.iter().filter(|e| e.event_type == "compact").count();

    let text = format!(
        "  Events in cache: {event_count}\n  Last event ID: {last_id}\n  Escalations: {escalation_count}    Compacts: {compact_count}",
    );

    let block = theme::panel_block(" Cache Info ", false);
    let para = Paragraph::new(text)
        .style(Style::new().fg(theme::TEXT_SECONDARY).bg(theme::BG_RAISED))
        .block(block);

    para.render(area, frame);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::NtmApp;
    use crate::msg::ConnState;
    use crate::rpc::types::StatsSummary;
    use crate::test_helpers::*;
    use ftui::core::geometry::Rect;

    fn app_with_connection() -> NtmApp {
        let mut app = NtmApp::new();
        app.conn_state = ConnState::Connected;
        app.daemon_version = "2.1.0".to_string();
        app.stats = StatsSummary {
            sessions: 3,
            panes: 8,
            total_compacts: 50,
            active_minutes: 120,
            estimated_tokens: 200_000,
        };
        app
    }

    #[test]
    fn test_render_shows_connection_panel() {
        test_frame!(pool, frame, 80, 20);
        let area = Rect::new(0, 0, 80, 20);
        let app = app_with_connection();
        render(&mut frame, area, &app);
        assert_text_present(&frame.buffer, "Connection");
    }

    #[test]
    fn test_render_shows_connected_status() {
        test_frame!(pool, frame, 80, 20);
        let area = Rect::new(0, 0, 80, 20);
        let app = app_with_connection();
        render(&mut frame, area, &app);
        assert_text_present(&frame.buffer, "connected");
        assert_text_present(&frame.buffer, "2.1.0");
    }

    #[test]
    fn test_render_shows_daemon_panel() {
        test_frame!(pool, frame, 80, 20);
        let area = Rect::new(0, 0, 80, 20);
        let app = app_with_connection();
        render(&mut frame, area, &app);
        assert_text_present(&frame.buffer, "Daemon");
        assert_text_present(&frame.buffer, "JSON-RPC 2.0");
    }

    #[test]
    fn test_render_shows_statistics() {
        test_frame!(pool, frame, 80, 20);
        let area = Rect::new(0, 0, 80, 20);
        let app = app_with_connection();
        render(&mut frame, area, &app);
        assert_text_present(&frame.buffer, "Statistics");
        assert_text_present(&frame.buffer, "Sessions: 3");
        assert_text_present(&frame.buffer, "Panes: 8");
    }

    #[test]
    fn test_render_shows_cache_info() {
        test_frame!(pool, frame, 80, 20);
        let area = Rect::new(0, 0, 80, 20);
        let app = app_with_connection();
        render(&mut frame, area, &app);
        assert_text_present(&frame.buffer, "Cache Info");
    }

    #[test]
    fn test_render_disconnected_shows_status() {
        test_frame!(pool, frame, 80, 20);
        let area = Rect::new(0, 0, 80, 20);
        let app = NtmApp::new();
        render(&mut frame, area, &app);
        assert_text_present(&frame.buffer, "disconnected");
    }
}
