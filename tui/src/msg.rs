use ftui::Event;

/// Active tab in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard,
    Sessions,
    Events,
    Health,
}

impl Tab {
    pub fn label(&self) -> &'static str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::Sessions => "Sessions",
            Tab::Events => "Events",
            Tab::Health => "Health",
        }
    }

    pub fn all() -> &'static [Tab] {
        &[Tab::Dashboard, Tab::Sessions, Tab::Events, Tab::Health]
    }
}

/// Which panel has focus within the dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusArea {
    SessionList,
    PaneTable,
    EventTimeline,
    EscalationInbox,
}

impl FocusArea {
    pub fn next(&self) -> FocusArea {
        match self {
            FocusArea::SessionList => FocusArea::PaneTable,
            FocusArea::PaneTable => FocusArea::EscalationInbox,
            FocusArea::EscalationInbox => FocusArea::EventTimeline,
            FocusArea::EventTimeline => FocusArea::SessionList,
        }
    }

    pub fn prev(&self) -> FocusArea {
        match self {
            FocusArea::SessionList => FocusArea::EventTimeline,
            FocusArea::PaneTable => FocusArea::SessionList,
            FocusArea::EscalationInbox => FocusArea::PaneTable,
            FocusArea::EventTimeline => FocusArea::EscalationInbox,
        }
    }
}

/// Daemon connection state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnState {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

impl ConnState {
    pub fn label(&self) -> &str {
        match self {
            ConnState::Disconnected => "disconnected",
            ConnState::Connecting => "connecting",
            ConnState::Connected => "connected",
            ConnState::Error(e) => e.as_str(),
        }
    }
}

/// All messages the TUI can receive.
#[derive(Debug, Clone)]
pub enum Msg {
    /// Terminal event (key press, resize, etc.)
    Term(Event),
    /// Periodic tick for animations/status.
    Tick,
    /// Snapshot received from daemon.
    SnapshotReceived(crate::rpc::types::Snapshot),
    /// Connection state changed.
    ConnectionChanged(ConnState),
    /// Daemon hello received.
    HelloReceived(String),
    /// RPC error.
    RpcError(String),
    /// No-op.
    None,
}

impl From<Event> for Msg {
    fn from(event: Event) -> Self {
        Msg::Term(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Tab enum tests (bd-1mw) ===

    #[test]
    fn test_tab_labels() {
        assert_eq!(Tab::Dashboard.label(), "Dashboard");
        assert_eq!(Tab::Sessions.label(), "Sessions");
        assert_eq!(Tab::Events.label(), "Events");
        assert_eq!(Tab::Health.label(), "Health");
    }

    #[test]
    fn test_tab_all_returns_four_variants() {
        let all = Tab::all();
        assert_eq!(all.len(), 4);
        assert_eq!(all[0], Tab::Dashboard);
        assert_eq!(all[1], Tab::Sessions);
        assert_eq!(all[2], Tab::Events);
        assert_eq!(all[3], Tab::Health);
    }

    #[test]
    fn test_tab_clone_eq() {
        let t = Tab::Dashboard;
        let t2 = t;
        assert_eq!(t, t2);
        assert_ne!(Tab::Dashboard, Tab::Sessions);
    }

    // === FocusArea state machine tests (bd-3jx) ===

    #[test]
    fn test_focus_next_full_cycle() {
        let start = FocusArea::SessionList;
        let a = start.next();
        assert_eq!(a, FocusArea::PaneTable);
        let b = a.next();
        assert_eq!(b, FocusArea::EscalationInbox);
        let c = b.next();
        assert_eq!(c, FocusArea::EventTimeline);
        let d = c.next();
        assert_eq!(d, FocusArea::SessionList);
    }

    #[test]
    fn test_focus_prev_full_cycle() {
        let start = FocusArea::SessionList;
        let a = start.prev();
        assert_eq!(a, FocusArea::EventTimeline);
        let b = a.prev();
        assert_eq!(b, FocusArea::EscalationInbox);
        let c = b.prev();
        assert_eq!(c, FocusArea::PaneTable);
        let d = c.prev();
        assert_eq!(d, FocusArea::SessionList);
    }

    #[test]
    fn test_focus_next_prev_inverse() {
        for &focus in &[
            FocusArea::SessionList,
            FocusArea::PaneTable,
            FocusArea::EscalationInbox,
            FocusArea::EventTimeline,
        ] {
            assert_eq!(focus.next().prev(), focus);
            assert_eq!(focus.prev().next(), focus);
        }
    }

    #[test]
    fn test_focus_all_variants_reachable() {
        let mut visited = std::collections::HashSet::new();
        let mut current = FocusArea::SessionList;
        for _ in 0..4 {
            visited.insert(format!("{:?}", current));
            current = current.next();
        }
        assert_eq!(visited.len(), 4);
    }

    // === ConnState label tests (bd-2oq) ===

    #[test]
    fn test_connstate_connected_label() {
        assert_eq!(ConnState::Connected.label(), "connected");
    }

    #[test]
    fn test_connstate_connecting_label() {
        assert_eq!(ConnState::Connecting.label(), "connecting");
    }

    #[test]
    fn test_connstate_disconnected_label() {
        assert_eq!(ConnState::Disconnected.label(), "disconnected");
    }

    #[test]
    fn test_connstate_error_label() {
        let state = ConnState::Error("daemon crashed".to_string());
        assert_eq!(state.label(), "daemon crashed");
    }

    #[test]
    fn test_connstate_error_empty_string() {
        let state = ConnState::Error(String::new());
        assert_eq!(state.label(), "");
    }

    #[test]
    fn test_connstate_clone() {
        let s1 = ConnState::Connected;
        let s2 = s1.clone();
        assert_eq!(s1, s2);

        let s3 = ConnState::Error("test".to_string());
        let s4 = s3.clone();
        assert_eq!(s3, s4);
    }
}
