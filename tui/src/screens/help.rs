use crate::theme;
use ftui::core::geometry::Rect;
use ftui::render::frame::Frame;
use ftui::Style;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::Widget;

/// Render the help overlay with scroll support.
pub fn render(frame: &mut Frame, area: Rect, scroll: u16) {
    let width = 68u16.min(area.width.saturating_sub(4));
    let height = 28u16.min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup = Rect::new(x, y, width, height);

    let help_text = "\
  NAVIGATION
  1-4          Switch screen
  Tab          Cycle focus to next panel
  Shift+Tab    Cycle focus to previous panel
  Ctrl+P  /    Open command palette
  ?            Toggle this help
  q  Ctrl+C    Quit

  LISTS (when a panel is focused)
  j / Down     Select next item
  k / Up       Select previous item
  g            Jump to first item
  G            Jump to last item
  Enter / l    Expand or collapse session

  ACTIONS
  K            Kill selected session
  s            Send text to selected pane
  d            Dismiss selected escalation

  EVENTS SCREEN FILTERS
  a  All    e  Escalations
  c  Compacts    s  Sessions

  SCREENS
  1 Dashboard   Overview with all panels
  2 Sessions    Detailed session view
  3 Events      Filterable event log
  4 Health      Connection diagnostics

  DAEMON ACTIONS
  K / kill     Kill a tmux session (via RPC)
  s / send     Send text to a tmux pane (via RPC)
  Both available via command palette (Ctrl+P)

        j/k to scroll, any other key to close";

    let block = theme::panel_block(" Help ", true);

    let para = Paragraph::new(help_text)
        .style(Style::new().fg(theme::TEXT_PRIMARY).bg(theme::BG_RAISED))
        .block(block)
        .scroll((scroll, 0));

    para.render(popup, frame);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;

    #[test]
    fn test_render_shows_navigation_section() {
        test_frame!(pool, frame, 80, 40);
        let area = Rect::new(0, 0, 80, 40);
        render(&mut frame, area, 0);
        assert_text_present(&frame.buffer, "NAVIGATION");
    }

    #[test]
    fn test_render_shows_key_bindings() {
        test_frame!(pool, frame, 80, 40);
        let area = Rect::new(0, 0, 80, 40);
        render(&mut frame, area, 0);
        assert_text_present(&frame.buffer, "Ctrl+P");
    }

    #[test]
    fn test_render_shows_actions_section() {
        test_frame!(pool, frame, 80, 40);
        let area = Rect::new(0, 0, 80, 40);
        render(&mut frame, area, 0);
        assert_text_present(&frame.buffer, "ACTIONS");
    }

    #[test]
    fn test_render_shows_help_title() {
        test_frame!(pool, frame, 80, 40);
        let area = Rect::new(0, 0, 80, 40);
        render(&mut frame, area, 0);
        assert_text_present(&frame.buffer, "Help");
    }

    #[test]
    fn test_render_centered_popup() {
        test_frame!(pool, frame, 80, 40);
        let area = Rect::new(0, 0, 80, 40);
        render(&mut frame, area, 0);
        // Row 0 should be empty (help popup is centered)
        let top = row_text(&frame.buffer, 0);
        assert!(top.trim().is_empty(), "Top row should be empty (centered popup): '{top}'");
    }
}
