use crate::app::NtmApp;
use crate::widgets::event_timeline;
use ftui::core::geometry::Rect;
use ftui::render::frame::Frame;

/// Render the full events screen.
pub fn render(frame: &mut Frame, area: Rect, app: &NtmApp) {
    event_timeline::render(
        frame,
        area,
        &app.events,
        &mut app.event_timeline_state.borrow_mut(),
        true,
    );
}
