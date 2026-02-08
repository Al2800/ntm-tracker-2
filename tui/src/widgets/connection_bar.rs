use crate::msg::{ConnState, Tab};
use crate::theme;
use ftui::core::geometry::Rect;
use ftui::render::frame::Frame;
use ftui::Style;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::Widget;

/// Render the segmented bottom status bar.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    conn: &ConnState,
    version: &str,
    _session_count: usize,
    active_tab: Tab,
    spinner_frame: usize,
) {
    let (conn_icon, conn_color) = match conn {
        ConnState::Connected => ("●", theme::ACTIVE),
        ConnState::Connecting => {
            let icon = theme::SPINNER_FRAMES[spinner_frame % theme::SPINNER_FRAMES.len()];
            (icon, theme::IDLE)
        }
        ConnState::Disconnected => ("○", theme::TEXT_MUTED),
        ConnState::Error(_) => ("✕", theme::ERROR),
    };

    // Left segment: connection status
    let left = format!(
        " {conn_icon} {label} v{version}",
        label = conn.label()
    );

    // Middle segment: navigation breadcrumb
    let breadcrumb = format!(" {tab} ", tab = active_tab.label());

    // Right segment: key hints
    let right = " q:quit ?:help Ctrl+P:cmd Tab:focus ";

    let separator = " │ ";
    let total_len = left.len() + separator.len() + breadcrumb.len()
        + separator.len() + right.len();
    let pad = area.width.saturating_sub(total_len as u16);

    let line = format!(
        "{left}{sep}{breadcrumb}{sep2}{pad:>width$}{right}",
        sep = separator,
        sep2 = separator,
        pad = "",
        width = pad as usize,
    );

    let para = Paragraph::new(line)
        .style(Style::new().fg(conn_color).bg(theme::BG_RAISED));

    para.render(area, frame);
}
