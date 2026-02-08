use crate::theme;
use ftui::core::geometry::Rect;
use ftui::render::frame::Frame;
use ftui::Style;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::Widget;

/// Render the help overlay.
pub fn render(frame: &mut Frame, area: Rect) {
    let width = 60u16.min(area.width.saturating_sub(4));
    let height = 22u16.min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup = Rect::new(x, y, width, height);

    let help_text = "\
  Keyboard Shortcuts

  q / Ctrl+c      Quit
  ?                Toggle this help
  1-4              Switch tab
  Tab / Shift+Tab  Cycle focus
  Ctrl+P / /       Command palette

  j / Down         Move down
  k / Up           Move up
  Enter / l        Expand/collapse session
  g / G            Jump to top / bottom

  d                Dismiss escalation
  f                Focus escalation pane
  a                Copy attach command
  o                Output preview
  K                Kill session (confirm)

  Events Screen:
  a/e/c/s          Filter: All/Escalations/Compacts/Sessions";

    let block = theme::panel_block(" Help ", true);

    let para = Paragraph::new(help_text)
        .style(Style::new().fg(theme::TEXT_PRIMARY).bg(theme::BG_RAISED))
        .block(block);

    para.render(popup, frame);
}
