use crate::msg::{ConnState, FocusArea, Msg, Tab};
use crate::rpc::types::{EventView, PaneView, SessionView, StatsSummary};
use crate::screens;
use crate::theme;
use crate::widgets::{connection_bar, escalation_inbox, event_timeline, pane_table, session_list};
use ftui::core::geometry::Rect;
use ftui::{Event, KeyCode, KeyEvent, KeyEventKind, Modifiers};
use ftui::layout::{Constraint, Flex};
use ftui::render::frame::Frame;
use ftui::{Cmd, Model};
use ftui::runtime::{Subscription, Every};
use ftui::Style;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::Widget;
use std::cell::RefCell;
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

    // Widget states (RefCell for interior mutability in view())
    pub session_list_state: RefCell<session_list::SessionListState>,
    pub pane_table_state: RefCell<pane_table::PaneTableState>,
    pub event_timeline_state: RefCell<event_timeline::EventTimelineState>,
    pub escalation_state: RefCell<escalation_inbox::EscalationInboxState>,
}

impl NtmApp {
    pub fn new() -> Self {
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

            session_list_state: RefCell::new(session_list::SessionListState::new()),
            pane_table_state: RefCell::new(pane_table::PaneTableState::new()),
            event_timeline_state: RefCell::new(event_timeline::EventTimelineState::new()),
            escalation_state: RefCell::new(escalation_inbox::EscalationInboxState::new()),
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
                // TODO: send dismiss RPC
            }
            _ => {}
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
            Msg::Tick => Cmd::None,
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
                self.conn_state = ConnState::Error(err);
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
        // Widget states use RefCell for interior mutability since view() takes &self.
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
        );

        // Help overlay (on top of everything)
        if self.show_help {
            screens::help::render(frame, area);
        }
    }

    fn subscriptions(&self) -> Vec<Box<dyn Subscription<Msg>>> {
        vec![Box::new(Every::new(Duration::from_millis(100), || Msg::Tick))]
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
    fn test_update_tick_is_noop() {
        let mut app = NtmApp::new();
        let cmd = app.update(Msg::Tick);
        assert!(matches!(cmd, Cmd::None));
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
        app.pane_table_state.borrow_mut().list_state.select(Some(0));

        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(
            app.pane_table_state.borrow().list_state.selected(),
            Some(1)
        );

        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(
            app.pane_table_state.borrow().list_state.selected(),
            Some(2) // clamped at 2 (3 panes for s2)
        );

        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(
            app.pane_table_state.borrow().list_state.selected(),
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
        app.pane_table_state.borrow_mut().list_state.select(Some(0));

        // Navigate down — should stop at 1 (2 panes, max index = 1)
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.pane_table_state.borrow().list_state.selected(), Some(1));

        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.pane_table_state.borrow().list_state.selected(), Some(1));
        // Clamped at 1, proving it used s1's pane count (2)
    }

    #[test]
    fn test_pane_table_no_session_selected() {
        let mut app = populated_app();
        app.focus = FocusArea::PaneTable;
        // Deselect session
        app.session_list_state.borrow_mut().list_state.select(None);
        app.pane_table_state.borrow_mut().list_state.select(Some(0));

        app.handle_key(key(KeyCode::Char('j')));
        // With pane_count=0 (no session selected), select_next is a no-op
        assert_eq!(app.pane_table_state.borrow().list_state.selected(), Some(0));
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
    }

    #[test]
    fn test_init_returns_none() {
        let mut app = NtmApp::new();
        let cmd = app.init();
        assert!(matches!(cmd, Cmd::None));
    }
}
