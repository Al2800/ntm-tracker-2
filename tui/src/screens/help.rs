use crate::theme;
use ftui::core::geometry::Rect;
use ftui::render::frame::Frame;
use ftui::Style;
use ftui::widgets::block::Block;
use ftui::widgets::borders::Borders;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::Widget;

/// Render the help overlay.
pub fn render(frame: &mut Frame, area: Rect) {
    // Center a box in the middle of the area.
    let width = 56u16.min(area.width.saturating_sub(4));
    let height = 18u16.min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup = Rect::new(x, y, width, height);

    let help_text = "\
  Keyboard Shortcuts

  q / Ctrl+c      Quit
  ?                Toggle this help
  1-4              Switch tab
  Tab / Shift+Tab  Cycle focus

  j / Down         Move down
  k / Up           Move up
  Enter / l        Expand/collapse session
  g / G            Jump to top / bottom

  d                Dismiss escalation
  f                Focus escalation
  a                Copy attach command
  o                Output preview
  K                Kill session (confirm)";

    let block = Block::new()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(theme::INFO))
        .style(Style::new().fg(theme::TEXT_PRIMARY).bg(theme::BG_RAISED));

    let para = Paragraph::new(help_text)
        .style(Style::new().fg(theme::TEXT_PRIMARY).bg(theme::BG_RAISED))
        .block(block);

    para.render(popup, frame);
}
