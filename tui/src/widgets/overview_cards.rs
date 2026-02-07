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

/// Render 4 stat cards in a horizontal row.
pub fn render(frame: &mut Frame, area: Rect, stats: &StatsSummary) {
    let cols = Flex::horizontal()
        .constraints([
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
        ])
        .split(area);

    let cards: [(&str, String, PackedRgba); 4] = [
        ("Sessions", format!("{}", stats.sessions), theme::INFO),
        ("Panes", format!("{}", stats.panes), theme::ACTIVE),
        ("Compacts", format!("{}", stats.total_compacts), theme::ACCENT),
        (
            "Active",
            format_active_time(stats.active_minutes),
            theme::IDLE,
        ),
    ];

    for (i, (label, value, color)) in cards.iter().enumerate() {
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(theme::BG_SURFACE))
            .style(Style::new().fg(*color).bg(theme::BG_RAISED));

        let text = format!("  {label}\n  {value}");
        let para = Paragraph::new(text)
            .style(Style::new().fg(*color).bg(theme::BG_RAISED))
            .block(block);

        para.render(cols[i], frame);
    }
}

pub(crate) fn format_active_time(minutes: u64) -> String {
    let hours = minutes / 60;
    let mins = minutes % 60;
    if hours > 0 {
        format!("{hours}.{mins_frac}h", mins_frac = mins * 10 / 60)
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
        let result = format_active_time(60);
        assert!(result.contains("1."), "Expected 1.Xh, got: {result}");
        assert!(result.contains('h'));
    }

    #[test]
    fn test_format_90_minutes() {
        let result = format_active_time(90);
        assert!(result.contains('h'), "Expected hours format, got: {result}");
    }

    #[test]
    fn test_format_large_value() {
        let result = format_active_time(1440);
        assert!(result.contains("24."), "Expected 24h, got: {result}");
    }

    #[test]
    fn test_format_1_minute() {
        assert_eq!(format_active_time(1), "1m");
    }
}
