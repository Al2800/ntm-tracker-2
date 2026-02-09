use crate::rpc::types::StatsSummary;
use crate::theme;
use ftui::layout::{Constraint, Flex};
use ftui::render::frame::Frame;
use ftui::core::geometry::Rect;
use ftui::Style;
use ftui::PackedRgba;
use ftui::widgets::block::Block;
use ftui::widgets::borders::Borders;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::Widget;

/// Render 5 stat cards in a horizontal row.
pub fn render(frame: &mut Frame, area: Rect, stats: &StatsSummary) {
    let cols = Flex::horizontal()
        .constraints([
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
        ])
        .split(area);

    let cards: [(&str, String, PackedRgba); 5] = [
        ("Sessions", format!("{}", stats.sessions), theme::INFO),
        ("Panes", format!("{}", stats.panes), theme::ACTIVE),
        ("Compacts", format!("{}", stats.total_compacts), theme::ACCENT),
        ("Tokens", theme::format_tokens(stats.estimated_tokens), theme::IDLE),
        (
            "Active",
            format_active_time(stats.active_minutes),
            theme::PAUSED,
        ),
    ];

    for (i, (label, value, color)) in cards.iter().enumerate() {
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(theme::BORDER_DIM))
            .style(Style::new().fg(*color).bg(theme::BG_RAISED));

        let text = format!("  {value}\n  {label}");
        let para = Paragraph::new(text)
            .style(Style::new().fg(*color).bg(theme::BG_RAISED))
            .block(block);

        para.render(cols[i], frame);
    }
}

pub(crate) fn format_active_time(minutes: u64) -> String {
    let hours = minutes / 60;
    let mins = minutes % 60;
    if hours > 0 && mins > 0 {
        format!("{hours}h {mins}m")
    } else if hours > 0 {
        format!("{hours}h")
    } else {
        format!("{mins}m")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_zero_minutes() {
        assert_eq!(format_active_time(0), "0m");
    }

    #[test]
    fn test_format_under_60() {
        assert_eq!(format_active_time(45), "45m");
    }

    #[test]
    fn test_format_exactly_60() {
        assert_eq!(format_active_time(60), "1h");
    }

    #[test]
    fn test_format_90_minutes() {
        assert_eq!(format_active_time(90), "1h 30m");
    }

    #[test]
    fn test_format_large_value() {
        assert_eq!(format_active_time(1440), "24h");
    }

    #[test]
    fn test_format_1_minute() {
        assert_eq!(format_active_time(1), "1m");
    }

    // === Render tests ===

    use crate::test_helpers::*;
    use ftui::core::geometry::Rect;

    fn sample_stats() -> StatsSummary {
        StatsSummary {
            sessions: 5,
            panes: 12,
            total_compacts: 42,
            active_minutes: 90,
            estimated_tokens: 150_000,
        }
    }

    #[test]
    fn test_render_shows_session_count() {
        test_frame!(pool, frame, 80, 4);
        let area = Rect::new(0, 0, 80, 4);
        render(&mut frame, area, &sample_stats());
        assert_text_present(&frame.buffer, "5");
        assert_text_present(&frame.buffer, "Sessions");
    }

    #[test]
    fn test_render_shows_pane_count() {
        test_frame!(pool, frame, 80, 4);
        let area = Rect::new(0, 0, 80, 4);
        render(&mut frame, area, &sample_stats());
        assert_text_present(&frame.buffer, "12");
        assert_text_present(&frame.buffer, "Panes");
    }

    #[test]
    fn test_render_shows_compacts_and_tokens() {
        test_frame!(pool, frame, 80, 4);
        let area = Rect::new(0, 0, 80, 4);
        render(&mut frame, area, &sample_stats());
        assert_text_present(&frame.buffer, "42");
        assert_text_present(&frame.buffer, "Compacts");
        assert_text_present(&frame.buffer, "150K");
        assert_text_present(&frame.buffer, "Tokens");
    }

    #[test]
    fn test_render_shows_active_time() {
        test_frame!(pool, frame, 80, 4);
        let area = Rect::new(0, 0, 80, 4);
        render(&mut frame, area, &sample_stats());
        assert_text_present(&frame.buffer, "1h 30m");
        assert_text_present(&frame.buffer, "Active");
    }

    #[test]
    fn test_render_zero_stats() {
        test_frame!(pool, frame, 80, 4);
        let area = Rect::new(0, 0, 80, 4);
        let stats = StatsSummary::default();
        render(&mut frame, area, &stats);
        // Should render without panicking and show labels
        assert_text_present(&frame.buffer, "Sessions");
        assert_text_present(&frame.buffer, "0m");
    }
}
