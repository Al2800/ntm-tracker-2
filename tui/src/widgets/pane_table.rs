use crate::rpc::types::PaneView;
use crate::theme;
use ftui::core::geometry::Rect;
use ftui::render::frame::Frame;
use ftui::Style;
use ftui::widgets::block::Block;
use ftui::widgets::borders::Borders;
use ftui::widgets::list::{List, ListItem, ListState};
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::{StatefulWidget, Widget};

pub struct PaneTableState {
    pub list_state: ListState,
}

impl PaneTableState {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
        }
    }

    pub fn select_next(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        let i = self.list_state.selected().unwrap_or(0);
        self.list_state.select(Some((i + 1).min(len - 1)));
    }

    pub fn select_prev(&mut self) {
        let i = self.list_state.selected().unwrap_or(0);
        self.list_state.select(Some(i.saturating_sub(1)));
    }
}

/// Render the pane detail table for a single session.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    panes: &[PaneView],
    session_name: &str,
    state: &mut PaneTableState,
    focused: bool,
) {
    let border_color = if focused {
        theme::INFO
    } else {
        theme::BG_SURFACE
    };

    let title = format!(" Panes ({session_name}) ");
    let block = Block::new()
        .title(title.leak())
        .borders(Borders::ALL)
        .border_style(Style::new().fg(border_color))
        .style(theme::raised_style());

    if panes.is_empty() {
        let empty = Paragraph::new("  No panes")
            .style(theme::muted_style())
            .block(block);
        empty.render(area, frame);
        return;
    }

    // Header line
    let header = format!(
        " {idx:<4} {agent:<6}   {status:<8} {cmd}",
        idx = "#",
        agent = "Agent",
        status = "Status",
        cmd = "Command"
    );

    let mut items = vec![ListItem::new(header).style(Style::new().fg(theme::TEXT_SECONDARY))];

    for pane in panes {
        let badge = theme::status_badge(&pane.status);
        let color = theme::status_color(&pane.status);
        let agent = pane.agent_type.as_deref().unwrap_or("--");
        let cmd = pane.current_command.as_deref().unwrap_or("--");
        let line = format!(
            " {idx:<4} {agent:<6} {badge} {status:<8} {cmd}",
            idx = format!("#{}", pane.pane_index),
            status = pane.status,
        );
        items.push(ListItem::new(line).style(Style::new().fg(color)));
    }

    let list = List::new(items)
        .block(block)
        .highlight_style(theme::highlight_style())
        .highlight_symbol(">> ");

    StatefulWidget::render(&list, area, frame, &mut state.list_state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_default_selection() {
        let state = PaneTableState::new();
        assert_eq!(state.list_state.selected(), None);
    }

    #[test]
    fn test_select_next_increments() {
        let mut state = PaneTableState::new();
        state.list_state.select(Some(0));
        state.select_next(5);
        assert_eq!(state.list_state.selected(), Some(1));
    }

    #[test]
    fn test_select_next_clamps() {
        let mut state = PaneTableState::new();
        state.list_state.select(Some(4));
        state.select_next(5);
        assert_eq!(state.list_state.selected(), Some(4));
    }

    #[test]
    fn test_select_next_empty() {
        let mut state = PaneTableState::new();
        state.select_next(0); // no panic
    }

    #[test]
    fn test_select_prev_decrements() {
        let mut state = PaneTableState::new();
        state.list_state.select(Some(3));
        state.select_prev();
        assert_eq!(state.list_state.selected(), Some(2));
    }

    #[test]
    fn test_select_prev_saturates_at_zero() {
        let mut state = PaneTableState::new();
        state.list_state.select(Some(0));
        state.select_prev();
        assert_eq!(state.list_state.selected(), Some(0));
    }
}
