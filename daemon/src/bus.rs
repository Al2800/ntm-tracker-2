use crate::metrics::{Timer, METRICS};
use crate::models::pane::Pane;
use crate::models::session::Session;
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::broadcast;

#[derive(Clone, Debug)]
pub struct StateChange {
    pub sessions: Vec<Session>,
    pub panes: Vec<Pane>,
    pub observed_at: i64,
}

#[derive(Clone, Debug)]
pub enum DaemonEventType {
    Compact,
    Escalation,
    PaneStatus,
    SessionStatus,
    Custom(String),
}

#[derive(Clone, Debug)]
pub struct DaemonEvent {
    pub event_type: DaemonEventType,
    pub session_uid: String,
    pub pane_uid: Option<String>,
    pub detected_at: i64,
    pub payload: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct ClientUpdate {
    pub kind: String,
    pub payload: Option<Value>,
}

#[derive(Clone, Debug, Default)]
pub struct BusMetricsSnapshot {
    pub state_sent: u64,
    pub state_errors: u64,
    pub events_sent: u64,
    pub events_errors: u64,
    pub client_sent: u64,
    pub client_errors: u64,
}

pub struct EventBus {
    state_tx: broadcast::Sender<StateChange>,
    event_tx: broadcast::Sender<DaemonEvent>,
    client_tx: broadcast::Sender<ClientUpdate>,
    state_sent: AtomicU64,
    state_errors: AtomicU64,
    events_sent: AtomicU64,
    events_errors: AtomicU64,
    client_sent: AtomicU64,
    client_errors: AtomicU64,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        let (state_tx, _) = broadcast::channel(capacity);
        let (event_tx, _) = broadcast::channel(capacity);
        let (client_tx, _) = broadcast::channel(capacity);
        Self {
            state_tx,
            event_tx,
            client_tx,
            state_sent: AtomicU64::new(0),
            state_errors: AtomicU64::new(0),
            events_sent: AtomicU64::new(0),
            events_errors: AtomicU64::new(0),
            client_sent: AtomicU64::new(0),
            client_errors: AtomicU64::new(0),
        }
    }

    pub fn subscribe_state(&self) -> broadcast::Receiver<StateChange> {
        self.state_tx.subscribe()
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<DaemonEvent> {
        self.event_tx.subscribe()
    }

    pub fn subscribe_clients(&self) -> broadcast::Receiver<ClientUpdate> {
        self.client_tx.subscribe()
    }

    pub fn publish_state(&self, change: StateChange) -> Result<usize, broadcast::error::SendError<StateChange>> {
        let _timer = Timer::new(&METRICS.event_processing);
        match self.state_tx.send(change) {
            Ok(count) => {
                self.state_sent.fetch_add(1, Ordering::Relaxed);
                Ok(count)
            }
            Err(err) => {
                self.state_errors.fetch_add(1, Ordering::Relaxed);
                Err(err)
            }
        }
    }

    pub fn publish_event(&self, event: DaemonEvent) -> Result<usize, broadcast::error::SendError<DaemonEvent>> {
        let _timer = Timer::new(&METRICS.event_processing);
        match self.event_tx.send(event) {
            Ok(count) => {
                self.events_sent.fetch_add(1, Ordering::Relaxed);
                Ok(count)
            }
            Err(err) => {
                self.events_errors.fetch_add(1, Ordering::Relaxed);
                Err(err)
            }
        }
    }

    pub fn publish_client_update(
        &self,
        update: ClientUpdate,
    ) -> Result<usize, broadcast::error::SendError<ClientUpdate>> {
        let _timer = Timer::new(&METRICS.event_processing);
        match self.client_tx.send(update) {
            Ok(count) => {
                self.client_sent.fetch_add(1, Ordering::Relaxed);
                Ok(count)
            }
            Err(err) => {
                self.client_errors.fetch_add(1, Ordering::Relaxed);
                Err(err)
            }
        }
    }

    pub fn metrics(&self) -> BusMetricsSnapshot {
        BusMetricsSnapshot {
            state_sent: self.state_sent.load(Ordering::Relaxed),
            state_errors: self.state_errors.load(Ordering::Relaxed),
            events_sent: self.events_sent.load(Ordering::Relaxed),
            events_errors: self.events_errors.load(Ordering::Relaxed),
            client_sent: self.client_sent.load(Ordering::Relaxed),
            client_errors: self.client_errors.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::session::SessionStatus;

    fn make_state_change() -> StateChange {
        StateChange {
            sessions: vec![Session {
                session_uid: "sess".to_string(),
                source_id: "src".to_string(),
                tmux_session_id: None,
                name: "name".to_string(),
                created_at: 1,
                last_seen_at: 1,
                ended_at: None,
                status: SessionStatus::Active,
                status_reason: None,
                pane_count: 0,
                metadata: None,
            }],
            panes: vec![],
            observed_at: 1,
        }
    }

    fn make_event() -> DaemonEvent {
        DaemonEvent {
            event_type: DaemonEventType::Compact,
            session_uid: "sess-1".to_string(),
            pane_uid: Some("pane-1".to_string()),
            detected_at: 1000,
            payload: None,
        }
    }

    fn make_client_update() -> ClientUpdate {
        ClientUpdate {
            kind: "snapshot".to_string(),
            payload: Some(serde_json::json!({"sessions": 3})),
        }
    }

    #[tokio::test]
    async fn state_channel_sends_and_receives() {
        let bus = EventBus::new(4);
        let mut rx = bus.subscribe_state();
        bus.publish_state(make_state_change()).expect("publish");
        let received = rx.recv().await.expect("receive");
        assert_eq!(received.sessions.len(), 1);
        let metrics = bus.metrics();
        assert_eq!(metrics.state_sent, 1);
    }

    #[tokio::test]
    async fn event_channel_sends_and_receives() {
        let bus = EventBus::new(4);
        let mut rx = bus.subscribe_events();
        bus.publish_event(make_event()).expect("publish");
        let received = rx.recv().await.expect("receive");
        assert_eq!(received.session_uid, "sess-1");
        assert!(matches!(received.event_type, DaemonEventType::Compact));
        assert_eq!(bus.metrics().events_sent, 1);
    }

    #[tokio::test]
    async fn client_channel_sends_and_receives() {
        let bus = EventBus::new(4);
        let mut rx = bus.subscribe_clients();
        bus.publish_client_update(make_client_update()).expect("publish");
        let received = rx.recv().await.expect("receive");
        assert_eq!(received.kind, "snapshot");
        assert!(received.payload.is_some());
        assert_eq!(bus.metrics().client_sent, 1);
    }

    #[tokio::test]
    async fn multiple_subscribers_receive_state() {
        let bus = EventBus::new(4);
        let mut rx1 = bus.subscribe_state();
        let mut rx2 = bus.subscribe_state();
        let count = bus.publish_state(make_state_change()).expect("publish");
        assert_eq!(count, 2);
        let r1 = rx1.recv().await.expect("rx1");
        let r2 = rx2.recv().await.expect("rx2");
        assert_eq!(r1.sessions.len(), 1);
        assert_eq!(r2.sessions.len(), 1);
    }

    #[tokio::test]
    async fn multiple_subscribers_receive_events() {
        let bus = EventBus::new(4);
        let mut rx1 = bus.subscribe_events();
        let mut rx2 = bus.subscribe_events();
        let mut rx3 = bus.subscribe_events();
        let count = bus.publish_event(make_event()).expect("publish");
        assert_eq!(count, 3);
        rx1.recv().await.unwrap();
        rx2.recv().await.unwrap();
        rx3.recv().await.unwrap();
    }

    #[test]
    fn publish_without_subscribers_errors() {
        let bus = EventBus::new(4);
        // No subscribers — send should error
        let result = bus.publish_state(make_state_change());
        assert!(result.is_err());
        assert_eq!(bus.metrics().state_errors, 1);
    }

    #[test]
    fn publish_event_without_subscribers_errors() {
        let bus = EventBus::new(4);
        let result = bus.publish_event(make_event());
        assert!(result.is_err());
        assert_eq!(bus.metrics().events_errors, 1);
    }

    #[test]
    fn publish_client_without_subscribers_errors() {
        let bus = EventBus::new(4);
        let result = bus.publish_client_update(make_client_update());
        assert!(result.is_err());
        assert_eq!(bus.metrics().client_errors, 1);
    }

    #[test]
    fn metrics_start_at_zero() {
        let bus = EventBus::new(4);
        let m = bus.metrics();
        assert_eq!(m.state_sent, 0);
        assert_eq!(m.state_errors, 0);
        assert_eq!(m.events_sent, 0);
        assert_eq!(m.events_errors, 0);
        assert_eq!(m.client_sent, 0);
        assert_eq!(m.client_errors, 0);
    }

    #[tokio::test]
    async fn metrics_accumulate() {
        let bus = EventBus::new(4);
        let _rx = bus.subscribe_state();
        bus.publish_state(make_state_change()).unwrap();
        bus.publish_state(make_state_change()).unwrap();
        bus.publish_state(make_state_change()).unwrap();
        assert_eq!(bus.metrics().state_sent, 3);
    }

    #[tokio::test]
    async fn dropped_subscriber_doesnt_panic_publisher() {
        let bus = EventBus::new(4);
        let rx = bus.subscribe_state();
        drop(rx);
        // After subscriber drops, publish returns error but doesn't panic
        let result = bus.publish_state(make_state_change());
        assert!(result.is_err());
    }

    #[test]
    fn capacity_minimum_is_one() {
        // Capacity 0 should be clamped to 1
        let bus = EventBus::new(0);
        let _rx = bus.subscribe_state();
        bus.publish_state(make_state_change()).unwrap();
    }

    #[test]
    fn daemon_event_type_custom() {
        let event = DaemonEvent {
            event_type: DaemonEventType::Custom("my-event".to_string()),
            session_uid: "s".to_string(),
            pane_uid: None,
            detected_at: 0,
            payload: None,
        };
        assert!(matches!(event.event_type, DaemonEventType::Custom(ref s) if s == "my-event"));
    }

    #[test]
    fn bus_metrics_snapshot_default() {
        let m = BusMetricsSnapshot::default();
        assert_eq!(m.state_sent, 0);
        assert_eq!(m.events_sent, 0);
        assert_eq!(m.client_sent, 0);
    }
}
