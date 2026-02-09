use crate::msg::{ConnState, FocusArea, Tab};
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
    focus: FocusArea,
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

    // Right segment: dynamic key hints based on focus
    let focus_hints = match focus {
        FocusArea::SessionList => "j/k:nav  Enter:expand  K:kill  Tab:next",
        FocusArea::PaneTable => "j/k:nav  s:send  Tab:next",
        FocusArea::EscalationInbox => "j/k:nav  d:dismiss  Tab:next",
        FocusArea::EventTimeline => "j/k:nav  Tab:next",
    };

    let tab_hints = if active_tab == Tab::Events {
        "  a:all e:esc c:comp s:sess"
    } else {
        ""
    };

    let right = format!(" {focus_hints}{tab_hints}  q:quit ?:help ^P:cmd ");

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;

    #[test]
    fn test_render_connected_shows_icon_and_label() {
        test_frame!(pool, frame, 120, 1);
        let area = Rect::new(0, 0, 120, 1);
        render(
            &mut frame, area, &ConnState::Connected, "1.2.3",
            3, Tab::Dashboard, 0, FocusArea::SessionList,
        );
        let row = row_text(&frame.buffer, 0);
        assert!(row.contains("●"), "Missing connected icon: {row}");
        assert!(row.contains("connected"), "Missing label: {row}");
        assert!(row.contains("v1.2.3"), "Missing version: {row}");
    }

    #[test]
    fn test_render_disconnected_shows_empty_icon() {
        test_frame!(pool, frame, 120, 1);
        let area = Rect::new(0, 0, 120, 1);
        render(
            &mut frame, area, &ConnState::Disconnected, "0.1",
            0, Tab::Dashboard, 0, FocusArea::SessionList,
        );
        let row = row_text(&frame.buffer, 0);
        assert!(row.contains("○"), "Missing disconnected icon: {row}");
        assert!(row.contains("disconnected"), "Missing label: {row}");
    }

    #[test]
    fn test_render_error_shows_x_icon() {
        test_frame!(pool, frame, 120, 1);
        let area = Rect::new(0, 0, 120, 1);
        render(
            &mut frame, area, &ConnState::Error("timeout".into()), "0.1",
            0, Tab::Dashboard, 0, FocusArea::SessionList,
        );
        let row = row_text(&frame.buffer, 0);
        assert!(row.contains("✕"), "Missing error icon: {row}");
        assert!(row.contains("timeout"), "Missing error label: {row}");
    }

    #[test]
    fn test_render_shows_active_tab_breadcrumb() {
        test_frame!(pool, frame, 120, 1);
        let area = Rect::new(0, 0, 120, 1);
        render(
            &mut frame, area, &ConnState::Connected, "1.0",
            0, Tab::Sessions, 0, FocusArea::SessionList,
        );
        let row = row_text(&frame.buffer, 0);
        assert!(row.contains("Sessions"), "Missing tab breadcrumb: {row}");
    }

    #[test]
    fn test_render_session_list_focus_hints() {
        test_frame!(pool, frame, 120, 1);
        let area = Rect::new(0, 0, 120, 1);
        render(
            &mut frame, area, &ConnState::Connected, "1.0",
            0, Tab::Dashboard, 0, FocusArea::SessionList,
        );
        let row = row_text(&frame.buffer, 0);
        assert!(row.contains("K:kill"), "Missing kill hint: {row}");
        assert!(row.contains("Enter:expand"), "Missing expand hint: {row}");
    }

    #[test]
    fn test_render_pane_table_focus_hints() {
        test_frame!(pool, frame, 120, 1);
        let area = Rect::new(0, 0, 120, 1);
        render(
            &mut frame, area, &ConnState::Connected, "1.0",
            0, Tab::Dashboard, 0, FocusArea::PaneTable,
        );
        let row = row_text(&frame.buffer, 0);
        assert!(row.contains("s:send"), "Missing send hint: {row}");
    }

    #[test]
    fn test_render_events_tab_shows_filter_hints() {
        test_frame!(pool, frame, 120, 1);
        let area = Rect::new(0, 0, 120, 1);
        render(
            &mut frame, area, &ConnState::Connected, "1.0",
            0, Tab::Events, 0, FocusArea::EventTimeline,
        );
        let row = row_text(&frame.buffer, 0);
        assert!(row.contains("a:all"), "Missing filter hints: {row}");
        assert!(row.contains("e:esc"), "Missing escalation filter: {row}");
    }

    #[test]
    fn test_render_always_shows_quit_help_cmd() {
        test_frame!(pool, frame, 120, 1);
        let area = Rect::new(0, 0, 120, 1);
        render(
            &mut frame, area, &ConnState::Connected, "1.0",
            0, Tab::Dashboard, 0, FocusArea::SessionList,
        );
        let row = row_text(&frame.buffer, 0);
        assert!(row.contains("q:quit"), "Missing quit hint: {row}");
        assert!(row.contains("?:help"), "Missing help hint: {row}");
    }
}
