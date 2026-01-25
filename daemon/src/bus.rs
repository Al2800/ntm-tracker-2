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

    #[tokio::test]
    async fn state_channel_sends_and_receives() {
        let bus = EventBus::new(4);
        let mut rx = bus.subscribe_state();
        let session = Session {
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
        };
        let change = StateChange {
            sessions: vec![session],
            panes: vec![],
            observed_at: 1,
        };
        bus.publish_state(change).expect("publish");
        let received = rx.recv().await.expect("receive");
        assert_eq!(received.sessions.len(), 1);
        let metrics = bus.metrics();
        assert_eq!(metrics.state_sent, 1);
    }
}
