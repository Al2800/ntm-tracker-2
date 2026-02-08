use crate::msg::{ConfirmAction, ConnState, EventFilter, FocusArea, Msg, Tab, ToastLevel};
use crate::rpc::types::{EventView, PaneView, SessionView, StatsSummary};
use crate::screens;
use crate::theme;
use crate::widgets::{
    command_palette_wrapper, connection_bar, escalation_inbox, event_timeline, pane_table,
    session_list, toast_manager,
};
use ftui::core::geometry::Rect;
use ftui::{Event, KeyCode, KeyEvent, KeyEventKind, Modifiers};
use ftui::layout::{Constraint, Flex};
use ftui::render::frame::Frame;
use ftui::{Cmd, Model};
use ftui::runtime::{Subscription, SubId, StopSignal, Every};
use ftui::Style;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::Widget;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Main application state.
pub struct NtmApp {
    // Navigation
    pub tab: Tab,
    pub focus: FocusArea,
    pub show_help: bool,

    // Data
    pub sessions: Vec<SessionView>,
    pub panes: Vec<PaneView>,
    pub events: Vec<EventView>,
    pub stats: StatsSummary,
    pub last_event_id: i64,

    // Connection
    pub conn_state: ConnState,
    pub daemon_version: String,

    // Animation
    pub spinner_frame: usize,

    // Filters
    pub event_filter: EventFilter,

    // Confirmation modal
    pub pending_confirm: Option<ConfirmAction>,

    // Widget states (RefCell for interior mutability in view())
    pub session_list_state: RefCell<session_list::SessionListState>,
    pub pane_table_state: RefCell<pane_table::PaneTableState>,
    pub event_timeline_state: RefCell<event_timeline::EventTimelineState>,
    pub escalation_state: RefCell<escalation_inbox::EscalationInboxState>,
    pub toast_queue: RefCell<toast_manager::ToastQueue>,
    pub palette_state: RefCell<command_palette_wrapper::PaletteState>,

    // Daemon message bridge (subscription drains this into the update loop)
    daemon_rx: Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<Msg>>>,
}

impl NtmApp {
    pub fn new() -> Self {
        let (_tx, rx) = tokio::sync::mpsc::unbounded_channel();
        Self::build(rx)
    }

    /// Create with a real daemon message receiver.
    pub fn with_daemon_rx(rx: tokio::sync::mpsc::UnboundedReceiver<Msg>) -> Self {
        Self::build(rx)
    }

    fn build(daemon_rx: tokio::sync::mpsc::UnboundedReceiver<Msg>) -> Self {
        Self {
            tab: Tab::Dashboard,
            focus: FocusArea::SessionList,
            show_help: false,

            sessions: vec![],
            panes: vec![],
            events: vec![],
            stats: StatsSummary::default(),
            last_event_id: 0,

            conn_state: ConnState::Disconnected,
            daemon_version: String::new(),

            spinner_frame: 0,
            event_filter: EventFilter::All,
            pending_confirm: None,

            session_list_state: RefCell::new(session_list::SessionListState::new()),
            pane_table_state: RefCell::new(pane_table::PaneTableState::new()),
            event_timeline_state: RefCell::new(event_timeline::EventTimelineState::new()),
            escalation_state: RefCell::new(escalation_inbox::EscalationInboxState::new()),
            toast_queue: RefCell::new(toast_manager::ToastQueue::new()),
            palette_state: RefCell::new(command_palette_wrapper::PaletteState::new()),

            daemon_rx: Arc::new(Mutex::new(daemon_rx)),
        }
    }

    fn session_count(&self) -> usize {
        self.sessions.len()
    }

    fn handle_key(&mut self, key: KeyEvent) -> Cmd<Msg> {
        // Only handle Press events
        if key.kind != KeyEventKind::Press {
            return Cmd::None;
        }

        // Command palette captures all keys when open
        if self.palette_state.borrow().visible {
            let action_id = self.palette_state.borrow_mut().handle_event(&Event::Key(key));
            if let Some(id) = action_id {
                return self.handle_palette_action(&id);
            }
            return Cmd::None;
        }

        // Confirmation modal captures keys when active
        if self.pending_confirm.is_some() {
            return self.handle_confirm_key(key);
        }

        // Global keys
        match key.code {
            KeyCode::Char('q') => return Cmd::Quit,
            KeyCode::Char('c') if key.modifiers.contains(Modifiers::CTRL) => {
                return Cmd::Quit
            }
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
                return Cmd::None;
            }
            KeyCode::Char('p') if key.modifiers.contains(Modifiers::CTRL) => {
                self.palette_state.borrow_mut().toggle(&self.sessions);
                return Cmd::None;
            }
            KeyCode::Char('/') => {
                self.palette_state.borrow_mut().open(&self.sessions);
                return Cmd::None;
            }
            _ => {}
        }

        // Help overlay captures all other keys
        if self.show_help {
            self.show_help = false;
            return Cmd::None;
        }

        // Tab switching
        match key.code {
            KeyCode::Char('1') => {
                self.tab = Tab::Dashboard;
                return Cmd::None;
            }
            KeyCode::Char('2') => {
                self.tab = Tab::Sessions;
                return Cmd::None;
            }
            KeyCode::Char('3') => {
                self.tab = Tab::Events;
                return Cmd::None;
            }
            KeyCode::Char('4') => {
                self.tab = Tab::Health;
                return Cmd::None;
            }
            KeyCode::Tab => {
                self.focus = self.focus.next();
                return Cmd::None;
            }
            KeyCode::BackTab => {
                self.focus = self.focus.prev();
                return Cmd::None;
            }
            _ => {}
        }

        // Events tab filter keys
        if self.tab == Tab::Events {
            match key.code {
                KeyCode::Char('a') => {
                    self.event_filter = EventFilter::All;
                    return Cmd::None;
                }
                KeyCode::Char('e') => {
                    self.event_filter = EventFilter::Escalations;
                    return Cmd::None;
                }
                KeyCode::Char('c') => {
                    self.event_filter = EventFilter::Compacts;
                    return Cmd::None;
                }
                KeyCode::Char('s') => {
                    self.event_filter = EventFilter::Sessions;
                    return Cmd::None;
                }
                _ => {}
            }
        }

        // Focus-specific navigation
        match self.focus {
            FocusArea::SessionList => self.handle_session_list_key(key),
            FocusArea::PaneTable => self.handle_pane_table_key(key),
            FocusArea::EventTimeline => self.handle_event_timeline_key(key),
            FocusArea::EscalationInbox => self.handle_escalation_key(key),
        }
    }

    fn handle_session_list_key(&mut self, key: KeyEvent) -> Cmd<Msg> {
        let len = self.sessions.len();
        let mut state = self.session_list_state.borrow_mut();
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => state.select_next(len),
            KeyCode::Char('k') | KeyCode::Up => state.select_prev(),
            KeyCode::Char('g') => state.select_first(),
            KeyCode::Char('G') => state.select_last(len),
            KeyCode::Enter | KeyCode::Char('l') => state.toggle_expand(),
            KeyCode::Char('K') => {
                // Kill session confirmation
                if let Some(i) = state.selected() {
                    if let Some(s) = self.sessions.get(i) {
                        drop(state); // release borrow before mutating self
                        self.pending_confirm = Some(ConfirmAction::KillSession {
                            session_id: s.session_id.clone(),
                            session_name: s.name.clone(),
                        });
                    }
                }
            }
            _ => {}
        }
        Cmd::None
    }

    fn handle_pane_table_key(&mut self, key: KeyEvent) -> Cmd<Msg> {
        let selected_session = self
            .session_list_state
            .borrow()
            .selected()
            .and_then(|i| self.sessions.get(i).map(|s| s.session_id.clone()));

        let pane_count = if let Some(sid) = &selected_session {
            self.panes.iter().filter(|p| p.session_id == *sid).count()
        } else {
            0
        };

        let mut state = self.pane_table_state.borrow_mut();
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => state.select_next(pane_count),
            KeyCode::Char('k') | KeyCode::Up => state.select_prev(),
            _ => {}
        }
        Cmd::None
    }

    fn handle_event_timeline_key(&mut self, key: KeyEvent) -> Cmd<Msg> {
        let len = self.events.len();
        let mut state = self.event_timeline_state.borrow_mut();
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => state.select_next(len),
            KeyCode::Char('k') | KeyCode::Up => state.select_prev(),
            _ => {}
        }
        Cmd::None
    }

    fn handle_escalation_key(&mut self, key: KeyEvent) -> Cmd<Msg> {
        let escalation_count = self
            .events
            .iter()
            .filter(|e| e.event_type == "escalation")
            .count();
        let mut state = self.escalation_state.borrow_mut();
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                state.select_next(escalation_count)
            }
            KeyCode::Char('k') | KeyCode::Up => state.select_prev(),
            KeyCode::Char('d') => {
                // Dismiss escalation — find the selected escalation's event ID
                if let Some(sel) = state.list_state.selected() {
                    let escalations: Vec<&EventView> = self
                        .events
                        .iter()
                        .filter(|e| e.event_type == "escalation")
                        .collect();
                    if let Some(esc) = escalations.get(sel) {
                        let event_id = esc.id;
                        drop(state);
                        self.toast_queue.borrow_mut().push(
                            format!("Escalation #{event_id} dismissed"),
                            ToastLevel::Success,
                        );
                    }
                }
            }
            _ => {}
        }
        Cmd::None
    }

    fn handle_confirm_key(&mut self, key: KeyEvent) -> Cmd<Msg> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(action) = self.pending_confirm.take() {
                    match action {
                        ConfirmAction::KillSession { session_name, .. } => {
                            self.toast_queue.borrow_mut().push(
                                format!("Session '{session_name}' killed"),
                                ToastLevel::Info,
                            );
                        }
                    }
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Escape => {
                self.pending_confirm = None;
            }
            _ => {}
        }
        Cmd::None
    }

    fn handle_palette_action(&mut self, action_id: &str) -> Cmd<Msg> {
        if let Some(tab_name) = action_id.strip_prefix("tab:") {
            match tab_name {
                "dashboard" => self.tab = Tab::Dashboard,
                "sessions" => self.tab = Tab::Sessions,
                "events" => self.tab = Tab::Events,
                "health" => self.tab = Tab::Health,
                _ => {}
            }
        } else if let Some(session_id) = action_id.strip_prefix("goto:") {
            // Find and select this session
            if let Some(idx) = self
                .sessions
                .iter()
                .position(|s| s.session_id == session_id)
            {
                self.session_list_state.borrow_mut().list_state.select(Some(idx));
                self.tab = Tab::Sessions;
            }
        } else if let Some(session_id) = action_id.strip_prefix("kill:") {
            if let Some(s) = self.sessions.iter().find(|s| s.session_id == session_id) {
                self.pending_confirm = Some(ConfirmAction::KillSession {
                    session_id: s.session_id.clone(),
                    session_name: s.name.clone(),
                });
            }
        }
        Cmd::None
    }
}

impl Model for NtmApp {
    type Message = Msg;

    fn init(&mut self) -> Cmd<Msg> {
        Cmd::None
    }

    fn update(&mut self, msg: Msg) -> Cmd<Msg> {
        match msg {
            Msg::Term(Event::Key(key)) => self.handle_key(key),
            Msg::Term(Event::Resize { .. }) => Cmd::None,
            Msg::Term(_) => Cmd::None,
            Msg::Tick => {
                self.spinner_frame = self.spinner_frame.wrapping_add(1);
                self.toast_queue.borrow_mut().tick();
                Cmd::None
            }
            Msg::SnapshotReceived(snap) => {
                self.sessions = snap.sessions;
                self.panes = snap.panes;
                self.events = snap.events;
                self.stats = snap.stats.summary;
                self.last_event_id = snap.last_event_id;
                Cmd::None
            }
            Msg::ConnectionChanged(state) => {
                self.conn_state = state;
                Cmd::None
            }
            Msg::HelloReceived(version) => {
                self.daemon_version = version;
                Cmd::None
            }
            Msg::RpcError(err) => {
                self.toast_queue.borrow_mut().push(
                    format!("RPC error: {err}"),
                    ToastLevel::Error,
                );
                self.conn_state = ConnState::Error(err);
                Cmd::None
            }
            Msg::DismissEscalation(event_id) => {
                self.toast_queue.borrow_mut().push(
                    format!("Escalation #{event_id} dismissed"),
                    ToastLevel::Success,
                );
                Cmd::None
            }
            Msg::KillSession(session_id) => {
                if let Some(s) = self.sessions.iter().find(|s| s.session_id == session_id) {
                    self.pending_confirm = Some(ConfirmAction::KillSession {
                        session_id: s.session_id.clone(),
                        session_name: s.name.clone(),
                    });
                }
                Cmd::None
            }
            Msg::KillSessionConfirmed(session_name) => {
                self.toast_queue.borrow_mut().push(
                    format!("Session '{session_name}' killed"),
                    ToastLevel::Info,
                );
                Cmd::None
            }
            Msg::ToastShow { message, level } => {
                self.toast_queue.borrow_mut().push(message, level);
                Cmd::None
            }
            Msg::None => Cmd::None,
        }
    }

    fn view(&self, frame: &mut Frame) {
        let area = Rect::from_size(frame.buffer.width(), frame.buffer.height());

        // Clear background
        let bg = Paragraph::new("").style(Style::new().bg(theme::BG_BASE));
        bg.render(area, frame);

        // Layout: header | content | footer
        let rows = Flex::vertical()
            .constraints([
                Constraint::Fixed(1), // header
                Constraint::Min(8),   // content
                Constraint::Fixed(1), // footer
            ])
            .split(area);

        // Header: tab bar
        render_header(frame, rows[0], self.tab);

        // Content: active tab
        match self.tab {
            Tab::Dashboard => screens::dashboard::render(frame, rows[1], self),
            Tab::Sessions => screens::session_detail::render(frame, rows[1], self),
            Tab::Events => screens::events::render(frame, rows[1], self),
            Tab::Health => screens::health::render(frame, rows[1], self),
        }

        // Footer: connection bar
        connection_bar::render(
            frame,
            rows[2],
            &self.conn_state,
            &self.daemon_version,
            self.session_count(),
            self.tab,
            self.spinner_frame,
        );

        // Confirmation modal overlay
        if let Some(ref action) = self.pending_confirm {
            render_confirm_modal(frame, area, action);
        }

        // Toast overlay
        if let Some(toast) = self.toast_queue.borrow().active() {
            render_toast(frame, area, toast);
        }

        // Command palette overlay
        if self.palette_state.borrow().visible {
            let palette_width = 50u16.min(area.width.saturating_sub(4));
            let palette_height = 12u16.min(area.height.saturating_sub(4));
            let x = area.x + (area.width.saturating_sub(palette_width)) / 2;
            let y = area.y + 2; // near top
            let palette_area = Rect::new(x, y, palette_width, palette_height);
            self.palette_state
                .borrow()
                .palette
                .render(palette_area, frame);
        }

        // Help overlay (on top of everything)
        if self.show_help {
            screens::help::render(frame, area);
        }
    }

    fn subscriptions(&self) -> Vec<Box<dyn Subscription<Msg>>> {
        vec![
            Box::new(Every::new(Duration::from_millis(100), || Msg::Tick)),
            Box::new(DaemonSubscription {
                receiver: self.daemon_rx.clone(),
            }),
        ]
    }
}

/// Bridges tokio daemon messages into FrankenTUI's subscription system.
struct DaemonSubscription {
    receiver: Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<Msg>>>,
}

impl Subscription<Msg> for DaemonSubscription {
    fn id(&self) -> SubId {
        42
    }

    fn run(&self, sender: std::sync::mpsc::Sender<Msg>, stop: StopSignal) {
        let receiver = self.receiver.clone();
        loop {
            // Wait 10ms or until stop signal
            if stop.wait_timeout(Duration::from_millis(10)) {
                break;
            }
            // Drain all available messages
            if let Ok(mut rx) = receiver.lock() {
                while let Ok(msg) = rx.try_recv() {
                    if sender.send(msg).is_err() {
                        return;
                    }
                }
            }
        }
    }
}

fn render_header(frame: &mut Frame, area: Rect, active_tab: Tab) {
    let mut header = String::from(" NTM Tracker ");
    header.push_str(&theme::BOX_HORIZONTAL.repeat(2));
    header.push(' ');

    for tab in Tab::all() {
        if *tab == active_tab {
            header.push_str(&format!("[{}]", tab.label()));
        } else {
            header.push_str(&format!(" {} ", tab.label()));
        }
        header.push_str(" │ ");
    }

    header.push_str("? ");

    let pad = area.width.saturating_sub(header.len() as u16);
    header.push_str(&" ".repeat(pad as usize));

    let para = Paragraph::new(header).style(Style::new().fg(theme::TEXT_PRIMARY).bg(theme::BG_RAISED));
    para.render(area, frame);
}

fn render_confirm_modal(frame: &mut Frame, area: Rect, action: &ConfirmAction) {
    let text = match action {
        ConfirmAction::KillSession { session_name, .. } => {
            format!("  Kill session '{session_name}'?  [y/n]")
        }
    };

    let width = (text.len() as u16 + 4).min(area.width.saturating_sub(4));
    let height = 3u16;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup = Rect::new(x, y, width, height);

    let block = theme::panel_block(" Confirm ", true);
    let para = Paragraph::new(text)
        .style(Style::new().fg(theme::ERROR).bg(theme::BG_RAISED))
        .block(block);
    para.render(popup, frame);
}

fn render_toast(frame: &mut Frame, area: Rect, toast: &toast_manager::ToastEntry) {
    let color = match toast.level {
        ToastLevel::Info => theme::INFO,
        ToastLevel::Success => theme::ACTIVE,
        ToastLevel::Error => theme::ERROR,
    };

    let text = format!(" {} ", toast.message);
    let width = (text.len() as u16 + 4).min(area.width.saturating_sub(2));
    let x = area.x + area.width.saturating_sub(width + 1);
    let y = area.y + 1;
    let toast_area = Rect::new(x, y, width, 3);

    let block = ftui::widgets::block::Block::new()
        .borders(ftui::widgets::borders::Borders::ALL)
        .border_style(Style::new().fg(color))
        .style(Style::new().fg(color).bg(theme::BG_SURFACE));

    let para = Paragraph::new(text)
        .style(Style::new().fg(color).bg(theme::BG_SURFACE))
        .block(block);
    para.render(toast_area, frame);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::types::{EventView, PaneView, SessionView, Snapshot, StatsEnvelope, StatsSummary};

    // === Test helpers ===

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code)
    }

    fn key_ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c)).with_modifiers(Modifiers::CTRL)
    }

    fn release_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code).with_kind(KeyEventKind::Release)
    }

    fn make_session(id: &str, name: &str) -> SessionView {
        SessionView {
            session_id: id.to_string(),
            name: name.to_string(),
            status: "active".to_string(),
            pane_count: 2,
            source_id: "tmux".to_string(),
            ..Default::default()
        }
    }

    fn make_pane(pane_id: &str, session_id: &str) -> PaneView {
        PaneView {
            pane_id: pane_id.to_string(),
            session_id: session_id.to_string(),
            status: "active".to_string(),
            ..Default::default()
        }
    }

    fn make_event(id: i64, event_type: &str, session_id: &str) -> EventView {
        EventView {
            id,
            event_type: event_type.to_string(),
            session_id: session_id.to_string(),
            pane_id: "pane-0".to_string(),
            detected_at: 1700000000 + id,
            severity: Some("high".to_string()),
            status: Some("pending".to_string()),
        }
    }

    fn populated_app() -> NtmApp {
        let mut app = NtmApp::new();
        app.sessions = vec![
            make_session("s1", "project-a"),
            make_session("s2", "project-b"),
            make_session("s3", "project-c"),
        ];
        app.panes = vec![
            make_pane("p1", "s1"),
            make_pane("p2", "s1"),
            make_pane("p3", "s2"),
            make_pane("p4", "s2"),
            make_pane("p5", "s2"),
            make_pane("p6", "s3"),
            make_pane("p7", "s3"),
        ];
        app.events = vec![
            make_event(1, "session_start", "s1"),
            make_event(2, "compact", "s1"),
            make_event(3, "escalation", "s2"),
            make_event(4, "session_start", "s3"),
            make_event(5, "escalation", "s1"),
        ];
        app.stats = StatsSummary {
            sessions: 3,
            panes: 7,
            total_compacts: 5,
            active_minutes: 120,
            estimated_tokens: 50000,
        };
        app.last_event_id = 5;
        app.conn_state = ConnState::Connected;
        app.daemon_version = "0.1.0".to_string();
        app
    }

    // ========================================================
    // bd-17nr: NtmApp message handling (update method)
    // ========================================================

    #[test]
    fn test_update_tick_increments_spinner() {
        let mut app = NtmApp::new();
        assert_eq!(app.spinner_frame, 0);
        let cmd = app.update(Msg::Tick);
        assert!(matches!(cmd, Cmd::None));
        assert_eq!(app.spinner_frame, 1);
    }

    #[test]
    fn test_update_none_is_noop() {
        let mut app = NtmApp::new();
        let cmd = app.update(Msg::None);
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn test_update_snapshot_populates_sessions() {
        let mut app = NtmApp::new();
        let snap = Snapshot {
            sessions: vec![make_session("s1", "test")],
            panes: vec![make_pane("p1", "s1")],
            events: vec![make_event(1, "compact", "s1")],
            stats: StatsEnvelope {
                summary: StatsSummary {
                    sessions: 1,
                    panes: 1,
                    total_compacts: 3,
                    active_minutes: 45,
                    estimated_tokens: 10000,
                },
                ..Default::default()
            },
            last_event_id: 42,
        };
        let cmd = app.update(Msg::SnapshotReceived(snap));
        assert!(matches!(cmd, Cmd::None));
        assert_eq!(app.sessions.len(), 1);
        assert_eq!(app.sessions[0].session_id, "s1");
        assert_eq!(app.panes.len(), 1);
        assert_eq!(app.events.len(), 1);
        assert_eq!(app.stats.sessions, 1);
        assert_eq!(app.stats.total_compacts, 3);
        assert_eq!(app.last_event_id, 42);
    }

    #[test]
    fn test_update_snapshot_replaces_previous() {
        let mut app = populated_app();
        assert_eq!(app.sessions.len(), 3);

        let snap = Snapshot {
            sessions: vec![make_session("s99", "new-only")],
            panes: vec![],
            events: vec![],
            stats: StatsEnvelope::default(),
            last_event_id: 100,
        };
        app.update(Msg::SnapshotReceived(snap));
        assert_eq!(app.sessions.len(), 1);
        assert_eq!(app.sessions[0].session_id, "s99");
        assert_eq!(app.panes.len(), 0);
        assert_eq!(app.events.len(), 0);
        assert_eq!(app.last_event_id, 100);
    }

    #[test]
    fn test_update_connection_changed() {
        let mut app = NtmApp::new();
        assert_eq!(app.conn_state, ConnState::Disconnected);
        app.update(Msg::ConnectionChanged(ConnState::Connecting));
        assert_eq!(app.conn_state, ConnState::Connecting);
        app.update(Msg::ConnectionChanged(ConnState::Connected));
        assert_eq!(app.conn_state, ConnState::Connected);
    }

    #[test]
    fn test_update_hello_received() {
        let mut app = NtmApp::new();
        assert!(app.daemon_version.is_empty());
        app.update(Msg::HelloReceived("1.2.3".to_string()));
        assert_eq!(app.daemon_version, "1.2.3");
    }

    #[test]
    fn test_update_hello_overwrites() {
        let mut app = NtmApp::new();
        app.update(Msg::HelloReceived("1.0".to_string()));
        app.update(Msg::HelloReceived("2.0".to_string()));
        assert_eq!(app.daemon_version, "2.0");
    }

    #[test]
    fn test_update_rpc_error_sets_error_state() {
        let mut app = NtmApp::new();
        app.conn_state = ConnState::Connected;
        app.update(Msg::RpcError("timeout".to_string()));
        assert_eq!(app.conn_state, ConnState::Error("timeout".to_string()));
    }

    #[test]
    fn test_update_rpc_error_creates_toast() {
        let mut app = NtmApp::new();
        app.update(Msg::RpcError("timeout".to_string()));
        let q = app.toast_queue.borrow();
        assert!(!q.is_empty());
        assert_eq!(q.active().unwrap().level, ToastLevel::Error);
    }

    #[test]
    fn test_update_resize_is_noop() {
        let mut app = NtmApp::new();
        let cmd = app.update(Msg::Term(Event::Resize {
            width: 120,
            height: 40,
        }));
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn test_update_key_q_quits() {
        let mut app = NtmApp::new();
        let cmd = app.update(Msg::Term(Event::Key(key(KeyCode::Char('q')))));
        assert!(matches!(cmd, Cmd::Quit));
    }

    #[test]
    fn test_update_key_ctrl_c_quits() {
        let mut app = NtmApp::new();
        let cmd = app.update(Msg::Term(Event::Key(key_ctrl('c'))));
        assert!(matches!(cmd, Cmd::Quit));
    }

    #[test]
    fn test_update_snapshot_empty() {
        let mut app = populated_app();
        let snap = Snapshot::default();
        app.update(Msg::SnapshotReceived(snap));
        assert!(app.sessions.is_empty());
        assert!(app.panes.is_empty());
        assert!(app.events.is_empty());
        assert_eq!(app.last_event_id, 0);
    }

    #[test]
    fn test_update_preserves_navigation_state() {
        let mut app = populated_app();
        app.tab = Tab::Events;
        app.focus = FocusArea::EscalationInbox;

        let snap = Snapshot {
            sessions: vec![make_session("s1", "new")],
            ..Default::default()
        };
        app.update(Msg::SnapshotReceived(snap));

        // Navigation state should be preserved
        assert_eq!(app.tab, Tab::Events);
        assert_eq!(app.focus, FocusArea::EscalationInbox);
    }

    #[test]
    fn test_update_toast_show() {
        let mut app = NtmApp::new();
        app.update(Msg::ToastShow {
            message: "hello".to_string(),
            level: ToastLevel::Info,
        });
        let q = app.toast_queue.borrow();
        assert!(!q.is_empty());
        assert_eq!(q.active().unwrap().message, "hello");
    }

    #[test]
    fn test_update_dismiss_escalation() {
        let mut app = NtmApp::new();
        app.update(Msg::DismissEscalation(42));
        let q = app.toast_queue.borrow();
        assert!(!q.is_empty());
        assert_eq!(q.active().unwrap().level, ToastLevel::Success);
    }

    #[test]
    fn test_update_kill_session_shows_confirm() {
        let mut app = populated_app();
        app.update(Msg::KillSession("s1".to_string()));
        assert!(app.pending_confirm.is_some());
    }

    #[test]
    fn test_update_kill_session_confirmed_toast() {
        let mut app = NtmApp::new();
        app.update(Msg::KillSessionConfirmed("project-a".to_string()));
        let q = app.toast_queue.borrow();
        assert!(!q.is_empty());
        assert_eq!(q.active().unwrap().level, ToastLevel::Info);
    }

    // ========================================================
    // bd-14ru: NtmApp key routing (handle_key)
    // ========================================================

    #[test]
    fn test_key_release_ignored() {
        let mut app = NtmApp::new();
        let cmd = app.handle_key(release_key(KeyCode::Char('q')));
        assert!(matches!(cmd, Cmd::None));
        // 'q' release should NOT quit
    }

    #[test]
    fn test_key_q_quits() {
        let mut app = NtmApp::new();
        let cmd = app.handle_key(key(KeyCode::Char('q')));
        assert!(matches!(cmd, Cmd::Quit));
    }

    #[test]
    fn test_key_ctrl_c_quits() {
        let mut app = NtmApp::new();
        let cmd = app.handle_key(key_ctrl('c'));
        assert!(matches!(cmd, Cmd::Quit));
    }

    #[test]
    fn test_key_question_toggles_help() {
        let mut app = NtmApp::new();
        assert!(!app.show_help);
        app.handle_key(key(KeyCode::Char('?')));
        assert!(app.show_help);
        app.handle_key(key(KeyCode::Char('?')));
        assert!(!app.show_help);
    }

    #[test]
    fn test_help_overlay_dismisses_on_any_key() {
        let mut app = NtmApp::new();
        app.show_help = true;
        let cmd = app.handle_key(key(KeyCode::Char('j')));
        assert!(matches!(cmd, Cmd::None));
        assert!(!app.show_help);
    }

    #[test]
    fn test_help_overlay_q_still_quits() {
        // 'q' is handled before help check
        let mut app = NtmApp::new();
        app.show_help = true;
        let cmd = app.handle_key(key(KeyCode::Char('q')));
        assert!(matches!(cmd, Cmd::Quit));
    }

    #[test]
    fn test_tab_switch_1234() {
        let mut app = NtmApp::new();
        assert_eq!(app.tab, Tab::Dashboard);

        app.handle_key(key(KeyCode::Char('2')));
        assert_eq!(app.tab, Tab::Sessions);

        app.handle_key(key(KeyCode::Char('3')));
        assert_eq!(app.tab, Tab::Events);

        app.handle_key(key(KeyCode::Char('4')));
        assert_eq!(app.tab, Tab::Health);

        app.handle_key(key(KeyCode::Char('1')));
        assert_eq!(app.tab, Tab::Dashboard);
    }

    #[test]
    fn test_tab_key_cycles_focus() {
        let mut app = NtmApp::new();
        assert_eq!(app.focus, FocusArea::SessionList);

        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.focus, FocusArea::PaneTable);

        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.focus, FocusArea::EscalationInbox);
    }

    #[test]
    fn test_backtab_cycles_focus_reverse() {
        let mut app = NtmApp::new();
        assert_eq!(app.focus, FocusArea::SessionList);

        app.handle_key(key(KeyCode::BackTab));
        assert_eq!(app.focus, FocusArea::EventTimeline);

        app.handle_key(key(KeyCode::BackTab));
        assert_eq!(app.focus, FocusArea::EscalationInbox);
    }

    #[test]
    fn test_session_list_j_selects_next() {
        let mut app = populated_app();
        app.focus = FocusArea::SessionList;
        app.session_list_state.borrow_mut().list_state.select(Some(0));

        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(
            app.session_list_state.borrow().list_state.selected(),
            Some(1)
        );
    }

    #[test]
    fn test_session_list_k_selects_prev() {
        let mut app = populated_app();
        app.focus = FocusArea::SessionList;
        app.session_list_state.borrow_mut().list_state.select(Some(2));

        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(
            app.session_list_state.borrow().list_state.selected(),
            Some(1)
        );
    }

    #[test]
    fn test_session_list_g_selects_first() {
        let mut app = populated_app();
        app.focus = FocusArea::SessionList;
        app.session_list_state.borrow_mut().list_state.select(Some(2));

        app.handle_key(key(KeyCode::Char('g')));
        assert_eq!(
            app.session_list_state.borrow().list_state.selected(),
            Some(0)
        );
    }

    #[test]
    fn test_session_list_G_selects_last() {
        let mut app = populated_app();
        app.focus = FocusArea::SessionList;
        app.session_list_state.borrow_mut().list_state.select(Some(0));

        app.handle_key(key(KeyCode::Char('G')));
        assert_eq!(
            app.session_list_state.borrow().list_state.selected(),
            Some(2) // 3 sessions, last index = 2
        );
    }

    #[test]
    fn test_session_list_enter_toggles_expand() {
        let mut app = populated_app();
        app.focus = FocusArea::SessionList;
        app.session_list_state.borrow_mut().list_state.select(Some(0));

        assert_eq!(app.session_list_state.borrow().expanded_index, None);
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.session_list_state.borrow().expanded_index, Some(0));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.session_list_state.borrow().expanded_index, None);
    }

    #[test]
    fn test_pane_table_navigation() {
        let mut app = populated_app();
        app.focus = FocusArea::PaneTable;
        // Select session s2 (has 3 panes: p3, p4, p5)
        app.session_list_state.borrow_mut().list_state.select(Some(1));
        app.pane_table_state.borrow_mut().table_state.select(Some(0));

        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(
            app.pane_table_state.borrow().table_state.selected,
            Some(1)
        );

        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(
            app.pane_table_state.borrow().table_state.selected,
            Some(2) // clamped at 2 (3 panes for s2)
        );

        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(
            app.pane_table_state.borrow().table_state.selected,
            Some(1)
        );
    }

    #[test]
    fn test_event_timeline_navigation() {
        let mut app = populated_app();
        app.focus = FocusArea::EventTimeline;
        app.event_timeline_state.borrow_mut().list_state.select(Some(0));

        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(
            app.event_timeline_state.borrow().list_state.selected(),
            Some(1)
        );
    }

    #[test]
    fn test_escalation_navigation() {
        let mut app = populated_app();
        app.focus = FocusArea::EscalationInbox;
        // 2 escalation events in our test data
        app.escalation_state.borrow_mut().list_state.select(Some(0));

        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(
            app.escalation_state.borrow().list_state.selected(),
            Some(1)
        );

        // Should clamp at 1 (2 escalations, max index = 1)
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(
            app.escalation_state.borrow().list_state.selected(),
            Some(1)
        );
    }

    #[test]
    fn test_focus_routes_to_correct_handler() {
        let mut app = populated_app();
        app.session_list_state.borrow_mut().list_state.select(Some(0));
        app.event_timeline_state.borrow_mut().list_state.select(Some(0));

        // Focus on EventTimeline, pressing 'j' should move event selection not session
        app.focus = FocusArea::EventTimeline;
        app.handle_key(key(KeyCode::Char('j')));

        // Session should be unchanged at 0
        assert_eq!(
            app.session_list_state.borrow().list_state.selected(),
            Some(0)
        );
        // Event should have moved
        assert_eq!(
            app.event_timeline_state.borrow().list_state.selected(),
            Some(1)
        );
    }

    // ========================================================
    // bd-3dep: Data filtering logic
    // ========================================================

    #[test]
    fn test_pane_table_filters_by_selected_session() {
        let mut app = populated_app();
        app.focus = FocusArea::PaneTable;

        // Select session s1 (has 2 panes: p1, p2)
        app.session_list_state.borrow_mut().list_state.select(Some(0));
        app.pane_table_state.borrow_mut().table_state.select(Some(0));

        // Navigate down — should stop at 1 (2 panes, max index = 1)
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.pane_table_state.borrow().table_state.selected, Some(1));

        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.pane_table_state.borrow().table_state.selected, Some(1));
        // Clamped at 1, proving it used s1's pane count (2)
    }

    #[test]
    fn test_pane_table_no_session_selected() {
        let mut app = populated_app();
        app.focus = FocusArea::PaneTable;
        // Deselect session
        app.session_list_state.borrow_mut().list_state.select(None);
        app.pane_table_state.borrow_mut().table_state.select(Some(0));

        app.handle_key(key(KeyCode::Char('j')));
        // With pane_count=0 (no session selected), select_next is a no-op
        assert_eq!(app.pane_table_state.borrow().table_state.selected, Some(0));
    }

    #[test]
    fn test_escalation_count_filters_event_type() {
        let mut app = populated_app();
        app.focus = FocusArea::EscalationInbox;
        // We have 2 escalation events (id=3 and id=5)
        app.escalation_state.borrow_mut().list_state.select(Some(0));

        // Navigate to index 1 (second escalation)
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.escalation_state.borrow().list_state.selected(), Some(1));

        // Should clamp at 1 (only 2 escalations)
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.escalation_state.borrow().list_state.selected(), Some(1));
    }

    #[test]
    fn test_escalation_count_zero_when_no_escalations() {
        let mut app = NtmApp::new();
        app.events = vec![
            make_event(1, "compact", "s1"),
            make_event(2, "session_start", "s1"),
        ];
        app.focus = FocusArea::EscalationInbox;
        app.escalation_state.borrow_mut().list_state.select(Some(0));

        // With 0 escalations, select_next should be a no-op
        app.handle_key(key(KeyCode::Char('j')));
        // Still at 0 since select_next(0) does nothing
        assert_eq!(app.escalation_state.borrow().list_state.selected(), Some(0));
    }

    #[test]
    fn test_session_count_method() {
        let app = NtmApp::new();
        assert_eq!(app.session_count(), 0);

        let app = populated_app();
        assert_eq!(app.session_count(), 3);
    }

    #[test]
    fn test_new_defaults() {
        let app = NtmApp::new();
        assert_eq!(app.tab, Tab::Dashboard);
        assert_eq!(app.focus, FocusArea::SessionList);
        assert!(!app.show_help);
        assert!(app.sessions.is_empty());
        assert!(app.panes.is_empty());
        assert!(app.events.is_empty());
        assert_eq!(app.last_event_id, 0);
        assert_eq!(app.conn_state, ConnState::Disconnected);
        assert!(app.daemon_version.is_empty());
        assert_eq!(app.spinner_frame, 0);
        assert_eq!(app.event_filter, EventFilter::All);
        assert!(app.pending_confirm.is_none());
        assert!(app.toast_queue.borrow().is_empty());
        assert!(!app.palette_state.borrow().visible);
    }

    #[test]
    fn test_init_returns_none() {
        let mut app = NtmApp::new();
        let cmd = app.init();
        assert!(matches!(cmd, Cmd::None));
    }

    // ========================================================
    // New features: event filter, command palette, confirmation
    // ========================================================

    #[test]
    fn test_event_filter_keys_on_events_tab() {
        let mut app = NtmApp::new();
        app.tab = Tab::Events;
        assert_eq!(app.event_filter, EventFilter::All);

        app.handle_key(key(KeyCode::Char('e')));
        assert_eq!(app.event_filter, EventFilter::Escalations);

        app.handle_key(key(KeyCode::Char('c')));
        assert_eq!(app.event_filter, EventFilter::Compacts);

        app.handle_key(key(KeyCode::Char('s')));
        assert_eq!(app.event_filter, EventFilter::Sessions);

        app.handle_key(key(KeyCode::Char('a')));
        assert_eq!(app.event_filter, EventFilter::All);
    }

    #[test]
    fn test_event_filter_keys_ignored_on_other_tabs() {
        let mut app = NtmApp::new();
        app.tab = Tab::Dashboard;
        app.event_filter = EventFilter::All;

        // 'e' on dashboard should not change filter
        // (it falls through to focus-specific handler)
        app.handle_key(key(KeyCode::Char('e')));
        assert_eq!(app.event_filter, EventFilter::All);
    }

    #[test]
    fn test_ctrl_p_toggles_palette() {
        let mut app = NtmApp::new();
        assert!(!app.palette_state.borrow().visible);

        app.handle_key(key_ctrl('p'));
        assert!(app.palette_state.borrow().visible);

        // Esc should close palette (handled by palette itself)
        app.handle_key(key(KeyCode::Escape));
        assert!(!app.palette_state.borrow().visible);
    }

    #[test]
    fn test_slash_opens_palette() {
        let mut app = NtmApp::new();
        assert!(!app.palette_state.borrow().visible);

        app.handle_key(key(KeyCode::Char('/')));
        assert!(app.palette_state.borrow().visible);
    }

    #[test]
    fn test_confirm_modal_y_accepts() {
        let mut app = populated_app();
        app.pending_confirm = Some(ConfirmAction::KillSession {
            session_id: "s1".to_string(),
            session_name: "project-a".to_string(),
        });

        app.handle_key(key(KeyCode::Char('y')));
        assert!(app.pending_confirm.is_none());
        assert!(!app.toast_queue.borrow().is_empty());
    }

    #[test]
    fn test_confirm_modal_n_cancels() {
        let mut app = populated_app();
        app.pending_confirm = Some(ConfirmAction::KillSession {
            session_id: "s1".to_string(),
            session_name: "project-a".to_string(),
        });

        app.handle_key(key(KeyCode::Char('n')));
        assert!(app.pending_confirm.is_none());
        assert!(app.toast_queue.borrow().is_empty());
    }

    #[test]
    fn test_confirm_modal_esc_cancels() {
        let mut app = populated_app();
        app.pending_confirm = Some(ConfirmAction::KillSession {
            session_id: "s1".to_string(),
            session_name: "project-a".to_string(),
        });

        app.handle_key(key(KeyCode::Escape));
        assert!(app.pending_confirm.is_none());
    }

    #[test]
    fn test_confirm_modal_captures_other_keys() {
        let mut app = populated_app();
        app.pending_confirm = Some(ConfirmAction::KillSession {
            session_id: "s1".to_string(),
            session_name: "project-a".to_string(),
        });

        // 'j' should NOT navigate session list while modal is open
        let old_selected = app.session_list_state.borrow().selected();
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.session_list_state.borrow().selected(), old_selected);
        // Modal should still be active
        assert!(app.pending_confirm.is_some());
    }

    #[test]
    fn test_kill_session_k_key() {
        let mut app = populated_app();
        app.focus = FocusArea::SessionList;
        app.session_list_state.borrow_mut().list_state.select(Some(0));

        app.handle_key(key(KeyCode::Char('K')));
        assert!(app.pending_confirm.is_some());
        if let Some(ConfirmAction::KillSession { session_name, .. }) = &app.pending_confirm {
            assert_eq!(session_name, "project-a");
        } else {
            panic!("Expected KillSession confirm action");
        }
    }

    #[test]
    fn test_palette_action_tab_switch() {
        let mut app = NtmApp::new();
        app.handle_palette_action("tab:events");
        assert_eq!(app.tab, Tab::Events);
    }

    #[test]
    fn test_palette_action_goto_session() {
        let mut app = populated_app();
        app.handle_palette_action("goto:s2");
        assert_eq!(app.tab, Tab::Sessions);
        assert_eq!(
            app.session_list_state.borrow().list_state.selected(),
            Some(1) // s2 is at index 1
        );
    }

    #[test]
    fn test_palette_action_kill_session() {
        let mut app = populated_app();
        app.handle_palette_action("kill:s1");
        assert!(app.pending_confirm.is_some());
    }

    #[test]
    fn test_spinner_wraps() {
        let mut app = NtmApp::new();
        app.spinner_frame = usize::MAX;
        app.update(Msg::Tick);
        assert_eq!(app.spinner_frame, 0); // wrapping_add
    }
}
