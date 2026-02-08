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
