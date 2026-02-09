use crate::app::NtmApp;
use crate::msg::EventFilter;
use crate::theme;
use crate::widgets::event_timeline;
use ftui::core::geometry::Rect;
use ftui::layout::{Constraint, Flex};
use ftui::render::frame::Frame;
use ftui::Style;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::Widget;

/// Render the full events screen with filter bar.
pub fn render(frame: &mut Frame, area: Rect, app: &NtmApp) {
    let rows = Flex::vertical()
        .constraints([
            Constraint::Fixed(1), // filter bar
            Constraint::Min(5),   // event list
        ])
        .split(area);

    // Filter bar
    render_filter_bar(frame, rows[0], app.event_filter);

    // Event list with filter applied
    event_timeline::render(
        frame,
        rows[1],
        &app.events,
        &mut app.event_timeline_state.borrow_mut(),
        true,
        app.event_filter,
    );
}

fn render_filter_bar(frame: &mut Frame, area: Rect, active_filter: EventFilter) {
    let filters = [
        (EventFilter::All, "a"),
        (EventFilter::Escalations, "e"),
        (EventFilter::Compacts, "c"),
        (EventFilter::Sessions, "s"),
    ];

    let mut bar = String::from(" Filter: ");
    for (filter, key) in &filters {
        if *filter == active_filter {
            bar.push_str(&format!("[{}] ", filter.label()));
        } else {
            bar.push_str(&format!(" {}:{} ", key, filter.label()));
        }
    }

    let pad = area.width.saturating_sub(bar.len() as u16);
    bar.push_str(&" ".repeat(pad as usize));

    let para = Paragraph::new(bar)
        .style(Style::new().fg(theme::TEXT_PRIMARY).bg(theme::BG_SURFACE));
    para.render(area, frame);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::NtmApp;
    use crate::rpc::types::EventView;
    use crate::test_helpers::*;
    use ftui::core::geometry::Rect;

    #[test]
    fn test_render_shows_filter_bar() {
        test_frame!(pool, frame, 80, 15);
        let area = Rect::new(0, 0, 80, 15);
        let app = NtmApp::new();
        render(&mut frame, area, &app);
        assert_text_present(&frame.buffer, "Filter");
    }

    #[test]
    fn test_render_default_filter_all_highlighted() {
        test_frame!(pool, frame, 80, 15);
        let area = Rect::new(0, 0, 80, 15);
        let app = NtmApp::new(); // default filter is All
        render(&mut frame, area, &app);
        assert_text_present(&frame.buffer, "[All]");
    }

    #[test]
    fn test_render_escalation_filter_highlighted() {
        test_frame!(pool, frame, 80, 15);
        let area = Rect::new(0, 0, 80, 15);
        let mut app = NtmApp::new();
        app.event_filter = EventFilter::Escalations;
        render(&mut frame, area, &app);
        assert_text_present(&frame.buffer, "[Escalations]");
    }

    #[test]
    fn test_render_empty_shows_no_events() {
        test_frame!(pool, frame, 80, 15);
        let area = Rect::new(0, 0, 80, 15);
        let app = NtmApp::new();
        render(&mut frame, area, &app);
        assert_text_present(&frame.buffer, "No events");
    }

    #[test]
    fn test_render_with_events_shows_event_list() {
        test_frame!(pool, frame, 80, 15);
        let area = Rect::new(0, 0, 80, 15);
        let mut app = NtmApp::new();
        app.events = vec![EventView {
            event_type: "compact".to_string(),
            session_id: "s1".to_string(),
            pane_id: "p1".to_string(),
            detected_at: chrono::Utc::now().timestamp(),
            ..Default::default()
        }];
        render(&mut frame, area, &app);
        assert_text_present(&frame.buffer, "compact");
    }
}
