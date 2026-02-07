use crate::msg::ConnState;
use crate::theme;
use ftui::core::geometry::Rect;
use ftui::render::frame::Frame;
use ftui::Style;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::Widget;

/// Render the bottom status bar.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    conn: &ConnState,
    version: &str,
    session_count: usize,
) {
    let (conn_icon, conn_color) = match conn {
        ConnState::Connected => ("●", theme::ACTIVE),
        ConnState::Connecting => ("◌", theme::IDLE),
        ConnState::Disconnected => ("○", theme::TEXT_MUTED),
        ConnState::Error(_) => ("✕", theme::ERROR),
    };

    let left = format!(
        " {conn_icon} {label} │ v{version} │ {session_count} sessions",
        label = conn.label()
    );

    let right = " q:quit  ?:help  Tab ";
    let pad = area
        .width
        .saturating_sub(left.len() as u16 + right.len() as u16);

    let line = format!("{left}{:pad$}{right}", "", pad = pad as usize);

    let para = Paragraph::new(line)
        .style(Style::new().fg(conn_color).bg(theme::BG_RAISED));

    para.render(area, frame);
}
