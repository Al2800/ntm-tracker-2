use crate::models::pane::Pane;
use crate::models::session::Session;
use serde::Serialize;
use dashmap::DashMap;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

#[derive(Clone, Debug, Default)]
pub struct StatsAggregate {
    pub total_compacts: u64,
    pub active_minutes: u64,
    pub estimated_tokens: u64,
}

#[derive(Clone, Debug)]
pub struct EventRecord {
    pub event_id: Option<i64>,
    pub session_uid: String,
    pub pane_uid: String,
    pub event_type: String,
    pub detected_at: i64,
    pub severity: Option<String>,
    pub status: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct HealthStatus {
    pub status: String,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct PollingDatum {
    pub interval_ms: u64,
    pub mode: String,
    pub reason: String,
    pub last_change_at: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct PollingState {
    pub snapshot: PollingDatum,
    pub tmux: PollingDatum,
    pub ntm: PollingDatum,
}

#[derive(Clone, Debug, Default)]
pub struct CacheSnapshot {
    pub sessions: Vec<Session>,
    pub panes: Vec<Pane>,
    pub events: Vec<EventRecord>,
    pub stats_today: StatsAggregate,
    pub health: HealthStatus,
}

#[derive(Clone, Debug, Default)]
pub struct CacheMetrics {
    pub session_hits: u64,
    pub session_misses: u64,
    pub pane_hits: u64,
    pub pane_misses: u64,
}

pub struct Cache {
    sessions: DashMap<String, Session>,
    panes: DashMap<String, Pane>,
    recent_events: RwLock<VecDeque<EventRecord>>,
    stats_today: RwLock<StatsAggregate>,
    health: RwLock<HealthStatus>,
    polling_state: RwLock<PollingState>,
    max_events: usize,
    session_hits: AtomicU64,
    session_misses: AtomicU64,
    pane_hits: AtomicU64,
    pane_misses: AtomicU64,
}

impl Cache {
    pub fn new(max_events: usize) -> Self {
        Self {
            sessions: DashMap::new(),
            panes: DashMap::new(),
            recent_events: RwLock::new(VecDeque::with_capacity(max_events)),
            stats_today: RwLock::new(StatsAggregate::default()),
            health: RwLock::new(HealthStatus::default()),
            polling_state: RwLock::new(PollingState::default()),
            max_events: max_events.max(1),
            session_hits: AtomicU64::new(0),
            session_misses: AtomicU64::new(0),
            pane_hits: AtomicU64::new(0),
            pane_misses: AtomicU64::new(0),
        }
    }

    pub fn upsert_session(&self, session: Session) {
        self.sessions
            .insert(session.session_uid.clone(), session);
    }

    pub fn get_session(&self, session_uid: &str) -> Option<Session> {
        if let Some(entry) = self.sessions.get(session_uid) {
            self.session_hits.fetch_add(1, Ordering::Relaxed);
            Some(entry.clone())
        } else {
            self.session_misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    pub fn all_sessions(&self) -> Vec<Session> {
        self.sessions.iter().map(|entry| entry.value().clone()).collect()
    }

    pub fn remove_session(&self, session_uid: &str) {
        self.sessions.remove(session_uid);
    }

    pub fn upsert_pane(&self, pane: Pane) {
        self.panes.insert(pane.pane_uid.clone(), pane);
    }

    pub fn get_pane(&self, pane_uid: &str) -> Option<Pane> {
        if let Some(entry) = self.panes.get(pane_uid) {
            self.pane_hits.fetch_add(1, Ordering::Relaxed);
            Some(entry.clone())
        } else {
            self.pane_misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    pub fn all_panes(&self) -> Vec<Pane> {
        self.panes.iter().map(|entry| entry.value().clone()).collect()
    }

    pub fn remove_pane(&self, pane_uid: &str) {
        self.panes.remove(pane_uid);
    }

    pub fn record_event(&self, event: EventRecord) {
        let mut events = self
            .recent_events
            .write()
            .expect("cache recent_events lock");
        if events.len() == self.max_events {
            events.pop_front();
        }
        events.push_back(event);
    }

    pub fn recent_events(&self) -> Vec<EventRecord> {
        self.recent_events
            .read()
            .expect("cache recent_events lock")
            .iter()
            .cloned()
            .collect()
    }

    pub fn set_stats_today(&self, stats: StatsAggregate) {
        let mut guard = self.stats_today.write().expect("cache stats lock");
        *guard = stats;
    }

    pub fn stats_today(&self) -> StatsAggregate {
        self.stats_today
            .read()
            .expect("cache stats lock")
            .clone()
    }

    pub fn set_health(&self, health: HealthStatus) {
        let mut guard = self.health.write().expect("cache health lock");
        *guard = health;
    }

    pub fn health(&self) -> HealthStatus {
        self.health
            .read()
            .expect("cache health lock")
            .clone()
    }

    pub fn polling_state(&self) -> PollingState {
        self.polling_state
            .read()
            .expect("cache polling_state lock")
            .clone()
    }

    pub fn update_polling_snapshot(&self, next: PollingDatum) -> bool {
        let mut guard = self
            .polling_state
            .write()
            .expect("cache polling_state lock");
        if guard.snapshot == next {
            return false;
        }
        guard.snapshot = next;
        true
    }

    pub fn update_polling_tmux(&self, next: PollingDatum) -> bool {
        let mut guard = self
            .polling_state
            .write()
            .expect("cache polling_state lock");
        if guard.tmux == next {
            return false;
        }
        guard.tmux = next;
        true
    }

    pub fn update_polling_ntm(&self, next: PollingDatum) -> bool {
        let mut guard = self
            .polling_state
            .write()
            .expect("cache polling_state lock");
        if guard.ntm == next {
            return false;
        }
        guard.ntm = next;
        true
    }

    pub fn metrics(&self) -> CacheMetrics {
        CacheMetrics {
            session_hits: self.session_hits.load(Ordering::Relaxed),
            session_misses: self.session_misses.load(Ordering::Relaxed),
            pane_hits: self.pane_hits.load(Ordering::Relaxed),
            pane_misses: self.pane_misses.load(Ordering::Relaxed),
        }
    }

    /// Get the number of cached sessions.
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Get the number of cached panes.
    pub fn pane_count(&self) -> usize {
        self.panes.len()
    }

    /// Get the number of cached events.
    pub fn event_count(&self) -> usize {
        self.recent_events
            .read()
            .expect("cache recent_events lock")
            .len()
    }

    pub fn apply_snapshot(&self, snapshot: CacheSnapshot) {
        self.sessions.clear();
        self.panes.clear();

        for session in snapshot.sessions {
            self.upsert_session(session);
        }
        for pane in snapshot.panes {
            self.upsert_pane(pane);
        }

        {
            let mut events = self
                .recent_events
                .write()
                .expect("cache recent_events lock");
            events.clear();
            for event in snapshot.events.into_iter().take(self.max_events) {
                events.push_back(event);
            }
        }

        self.set_stats_today(snapshot.stats_today);
        self.set_health(snapshot.health);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session(uid: &str, name: &str) -> Session {
        Session {
            session_uid: uid.to_string(),
            source_id: "src".to_string(),
            tmux_session_id: None,
            name: name.to_string(),
            created_at: 1,
            last_seen_at: 1,
            ended_at: None,
            status: crate::models::session::SessionStatus::Active,
            status_reason: None,
            pane_count: 0,
            metadata: None,
        }
    }

    fn make_pane(uid: &str, session_uid: &str) -> Pane {
        Pane {
            pane_uid: uid.to_string(),
            session_uid: session_uid.to_string(),
            pane_index: 0,
            tmux_pane_id: None,
            tmux_window_id: None,
            tmux_pane_pid: None,
            agent_type: None,
            created_at: 1,
            last_seen_at: 1,
            last_activity_at: Some(1),
            current_command: None,
            ended_at: None,
            status: crate::models::pane::PaneStatus::Active,
            status_reason: None,
        }
    }

    #[test]
    fn cache_metrics_track_hits() {
        let cache = Cache::new(10);
        assert!(cache.get_session("missing").is_none());

        cache.upsert_session(make_session("sess-1", "alpha"));

        assert!(cache.get_session("sess-1").is_some());
        let metrics = cache.metrics();
        assert_eq!(metrics.session_hits, 1);
        assert_eq!(metrics.session_misses, 1);
    }

    #[test]
    fn event_ring_buffer_caps_entries() {
        let cache = Cache::new(2);
        for idx in 0..3 {
            cache.record_event(EventRecord {
                event_id: Some(idx),
                session_uid: "sess".to_string(),
                pane_uid: "pane".to_string(),
                event_type: "compact".to_string(),
                detected_at: idx,
                severity: None,
                status: None,
            });
        }

        let events = cache.recent_events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_id, Some(1));
        assert_eq!(events[1].event_id, Some(2));
    }

    #[test]
    fn snapshot_overwrites_state() {
        let cache = Cache::new(5);
        cache.upsert_session(Session {
            session_uid: "old".to_string(),
            source_id: "src".to_string(),
            tmux_session_id: None,
            name: "old".to_string(),
            created_at: 0,
            last_seen_at: 0,
            ended_at: None,
            status: crate::models::session::SessionStatus::Idle,
            status_reason: None,
            pane_count: 0,
            metadata: None,
        });

        let snapshot = CacheSnapshot {
            sessions: vec![Session {
                session_uid: "new".to_string(),
                source_id: "src".to_string(),
                tmux_session_id: None,
                name: "new".to_string(),
                created_at: 2,
                last_seen_at: 2,
                ended_at: None,
                status: crate::models::session::SessionStatus::Active,
                status_reason: None,
                pane_count: 0,
                metadata: None,
            }],
            panes: vec![],
            events: vec![],
            stats_today: StatsAggregate {
                total_compacts: 1,
                active_minutes: 5,
                estimated_tokens: 10,
            },
            health: HealthStatus {
                status: "ok".to_string(),
                last_error: None,
            },
        };

        cache.apply_snapshot(snapshot);
        assert!(cache.get_session("old").is_none());
        assert!(cache.get_session("new").is_some());
        assert_eq!(cache.stats_today().total_compacts, 1);
        assert_eq!(cache.health().status, "ok");
    }

    #[test]
    fn pane_metrics_track_hits_and_misses() {
        let cache = Cache::new(10);
        assert!(cache.get_pane("missing").is_none());

        cache.upsert_pane(make_pane("pane-1", "sess-1"));

        assert!(cache.get_pane("pane-1").is_some());
        let metrics = cache.metrics();
        assert_eq!(metrics.pane_hits, 1);
        assert_eq!(metrics.pane_misses, 1);
    }

    #[test]
    fn all_sessions_returns_all() {
        let cache = Cache::new(10);
        cache.upsert_session(make_session("sess-1", "alpha"));
        cache.upsert_session(make_session("sess-2", "beta"));

        let sessions = cache.all_sessions();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn all_panes_returns_all() {
        let cache = Cache::new(10);
        cache.upsert_pane(make_pane("pane-1", "sess-1"));
        cache.upsert_pane(make_pane("pane-2", "sess-1"));

        let panes = cache.all_panes();
        assert_eq!(panes.len(), 2);
    }

    #[test]
    fn remove_session_works() {
        let cache = Cache::new(10);
        cache.upsert_session(make_session("sess-1", "alpha"));
        assert!(cache.get_session("sess-1").is_some());

        cache.remove_session("sess-1");
        assert!(cache.get_session("sess-1").is_none());
    }

    #[test]
    fn remove_pane_works() {
        let cache = Cache::new(10);
        cache.upsert_pane(make_pane("pane-1", "sess-1"));
        assert!(cache.get_pane("pane-1").is_some());

        cache.remove_pane("pane-1");
        assert!(cache.get_pane("pane-1").is_none());
    }

    #[test]
    fn upsert_session_updates_existing() {
        let cache = Cache::new(10);
        let mut session = make_session("sess-1", "alpha");
        cache.upsert_session(session.clone());

        session.name = "updated".to_string();
        cache.upsert_session(session);

        let retrieved = cache.get_session("sess-1").unwrap();
        assert_eq!(retrieved.name, "updated");
        assert_eq!(cache.session_count(), 1);
    }

    #[test]
    fn upsert_pane_updates_existing() {
        let cache = Cache::new(10);
        let mut pane = make_pane("pane-1", "sess-1");
        cache.upsert_pane(pane.clone());

        pane.pane_index = 5;
        cache.upsert_pane(pane);

        let retrieved = cache.get_pane("pane-1").unwrap();
        assert_eq!(retrieved.pane_index, 5);
        assert_eq!(cache.pane_count(), 1);
    }

    #[test]
    fn set_and_get_stats_today() {
        let cache = Cache::new(10);
        let stats = StatsAggregate {
            total_compacts: 10,
            active_minutes: 120,
            estimated_tokens: 50000,
        };
        cache.set_stats_today(stats);

        let retrieved = cache.stats_today();
        assert_eq!(retrieved.total_compacts, 10);
        assert_eq!(retrieved.active_minutes, 120);
        assert_eq!(retrieved.estimated_tokens, 50000);
    }

    #[test]
    fn set_and_get_health() {
        let cache = Cache::new(10);
        let health = HealthStatus {
            status: "degraded".to_string(),
            last_error: Some("connection timeout".to_string()),
        };
        cache.set_health(health);

        let retrieved = cache.health();
        assert_eq!(retrieved.status, "degraded");
        assert_eq!(retrieved.last_error.as_deref(), Some("connection timeout"));
    }

    #[test]
    fn session_count_works() {
        let cache = Cache::new(10);
        assert_eq!(cache.session_count(), 0);

        cache.upsert_session(make_session("sess-1", "alpha"));
        assert_eq!(cache.session_count(), 1);

        cache.upsert_session(make_session("sess-2", "beta"));
        assert_eq!(cache.session_count(), 2);

        cache.remove_session("sess-1");
        assert_eq!(cache.session_count(), 1);
    }

    #[test]
    fn pane_count_works() {
        let cache = Cache::new(10);
        assert_eq!(cache.pane_count(), 0);

        cache.upsert_pane(make_pane("pane-1", "sess-1"));
        assert_eq!(cache.pane_count(), 1);

        cache.upsert_pane(make_pane("pane-2", "sess-1"));
        assert_eq!(cache.pane_count(), 2);
    }

    #[test]
    fn event_count_works() {
        let cache = Cache::new(10);
        assert_eq!(cache.event_count(), 0);

        cache.record_event(EventRecord {
            event_id: Some(1),
            session_uid: "sess".to_string(),
            pane_uid: "pane".to_string(),
            event_type: "compact".to_string(),
            detected_at: 1,
            severity: None,
            status: None,
        });
        assert_eq!(cache.event_count(), 1);
    }

    #[test]
    fn snapshot_includes_panes() {
        let cache = Cache::new(5);
        cache.upsert_pane(make_pane("old-pane", "sess"));

        let snapshot = CacheSnapshot {
            sessions: vec![],
            panes: vec![make_pane("new-pane", "sess")],
            events: vec![],
            stats_today: StatsAggregate::default(),
            health: HealthStatus::default(),
        };

        cache.apply_snapshot(snapshot);
        assert!(cache.get_pane("old-pane").is_none());
        assert!(cache.get_pane("new-pane").is_some());
    }

    #[test]
    fn snapshot_caps_events_to_max() {
        let cache = Cache::new(2);

        let snapshot = CacheSnapshot {
            sessions: vec![],
            panes: vec![],
            events: vec![
                EventRecord {
                    event_id: Some(1),
                    session_uid: "s".to_string(),
                    pane_uid: "p".to_string(),
                    event_type: "a".to_string(),
                    detected_at: 1,
                    severity: None,
                    status: None,
                },
                EventRecord {
                    event_id: Some(2),
                    session_uid: "s".to_string(),
                    pane_uid: "p".to_string(),
                    event_type: "b".to_string(),
                    detected_at: 2,
                    severity: None,
                    status: None,
                },
                EventRecord {
                    event_id: Some(3),
                    session_uid: "s".to_string(),
                    pane_uid: "p".to_string(),
                    event_type: "c".to_string(),
                    detected_at: 3,
                    severity: None,
                    status: None,
                },
            ],
            stats_today: StatsAggregate::default(),
            health: HealthStatus::default(),
        };

        cache.apply_snapshot(snapshot);
        let events = cache.recent_events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_id, Some(1));
        assert_eq!(events[1].event_id, Some(2));
    }

    #[test]
    fn max_events_at_least_one() {
        let cache = Cache::new(0); // Should be clamped to 1
        cache.record_event(EventRecord {
            event_id: Some(1),
            session_uid: "s".to_string(),
            pane_uid: "p".to_string(),
            event_type: "a".to_string(),
            detected_at: 1,
            severity: None,
            status: None,
        });
        assert_eq!(cache.event_count(), 1);
    }

    #[test]
    fn default_stats_aggregate() {
        let stats = StatsAggregate::default();
        assert_eq!(stats.total_compacts, 0);
        assert_eq!(stats.active_minutes, 0);
        assert_eq!(stats.estimated_tokens, 0);
    }

    #[test]
    fn default_health_status() {
        let health = HealthStatus::default();
        assert_eq!(health.status, "");
        assert!(health.last_error.is_none());
    }

    // --- Polling state tests ---

    #[test]
    fn update_polling_snapshot_returns_true_on_change() {
        let cache = Cache::new(10);
        let datum = PollingDatum {
            interval_ms: 2000,
            mode: "active".to_string(),
            reason: "sessions detected".to_string(),
            last_change_at: 100,
        };
        assert!(cache.update_polling_snapshot(datum));
    }

    #[test]
    fn update_polling_snapshot_returns_false_when_unchanged() {
        let cache = Cache::new(10);
        let datum = PollingDatum {
            interval_ms: 2000,
            mode: "active".to_string(),
            reason: "sessions".to_string(),
            last_change_at: 100,
        };
        cache.update_polling_snapshot(datum.clone());
        assert!(!cache.update_polling_snapshot(datum));
    }

    #[test]
    fn update_polling_tmux() {
        let cache = Cache::new(10);
        let datum = PollingDatum {
            interval_ms: 5000,
            mode: "idle".to_string(),
            reason: "no activity".to_string(),
            last_change_at: 200,
        };
        assert!(cache.update_polling_tmux(datum.clone()));
        assert!(!cache.update_polling_tmux(datum));
    }

    #[test]
    fn update_polling_ntm() {
        let cache = Cache::new(10);
        let datum = PollingDatum {
            interval_ms: 10000,
            mode: "background".to_string(),
            reason: "no sessions".to_string(),
            last_change_at: 300,
        };
        assert!(cache.update_polling_ntm(datum.clone()));
        assert!(!cache.update_polling_ntm(datum));
    }

    #[test]
    fn polling_state_reflects_all_channels() {
        let cache = Cache::new(10);
        cache.update_polling_snapshot(PollingDatum {
            interval_ms: 1000,
            mode: "fast".to_string(),
            reason: "r1".to_string(),
            last_change_at: 1,
        });
        cache.update_polling_tmux(PollingDatum {
            interval_ms: 2000,
            mode: "normal".to_string(),
            reason: "r2".to_string(),
            last_change_at: 2,
        });
        cache.update_polling_ntm(PollingDatum {
            interval_ms: 3000,
            mode: "slow".to_string(),
            reason: "r3".to_string(),
            last_change_at: 3,
        });
        let state = cache.polling_state();
        assert_eq!(state.snapshot.interval_ms, 1000);
        assert_eq!(state.tmux.interval_ms, 2000);
        assert_eq!(state.ntm.interval_ms, 3000);
    }

    // --- Ring buffer edge cases ---

    #[test]
    fn event_ring_buffer_exact_capacity() {
        let cache = Cache::new(3);
        for i in 0..3 {
            cache.record_event(EventRecord {
                event_id: Some(i),
                session_uid: "s".to_string(),
                pane_uid: "p".to_string(),
                event_type: "compact".to_string(),
                detected_at: i,
                severity: None,
                status: None,
            });
        }
        assert_eq!(cache.event_count(), 3);
        // Add one more — should evict oldest
        cache.record_event(EventRecord {
            event_id: Some(3),
            session_uid: "s".to_string(),
            pane_uid: "p".to_string(),
            event_type: "compact".to_string(),
            detected_at: 3,
            severity: None,
            status: None,
        });
        assert_eq!(cache.event_count(), 3);
        let events = cache.recent_events();
        assert_eq!(events[0].event_id, Some(1));
        assert_eq!(events[2].event_id, Some(3));
    }

    // --- Health status transitions ---

    #[test]
    fn health_status_transitions() {
        let cache = Cache::new(10);
        cache.set_health(HealthStatus {
            status: "ok".to_string(),
            last_error: None,
        });
        assert_eq!(cache.health().status, "ok");

        cache.set_health(HealthStatus {
            status: "degraded".to_string(),
            last_error: Some("tmux timeout".to_string()),
        });
        assert_eq!(cache.health().status, "degraded");
        assert_eq!(cache.health().last_error.as_deref(), Some("tmux timeout"));

        cache.set_health(HealthStatus {
            status: "ok".to_string(),
            last_error: None,
        });
        assert_eq!(cache.health().status, "ok");
        assert!(cache.health().last_error.is_none());
    }

    // --- Concurrent access ---

    #[test]
    fn concurrent_session_updates() {
        use std::sync::Arc;
        use std::thread;

        let cache = Arc::new(Cache::new(100));
        let mut handles = vec![];

        for i in 0..10 {
            let cache = Arc::clone(&cache);
            handles.push(thread::spawn(move || {
                let session = make_session(&format!("sess-{i}"), &format!("name-{i}"));
                cache.upsert_session(session);
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(cache.session_count(), 10);
    }

    #[test]
    fn concurrent_pane_updates() {
        use std::sync::Arc;
        use std::thread;

        let cache = Arc::new(Cache::new(100));
        let mut handles = vec![];

        for i in 0..10 {
            let cache = Arc::clone(&cache);
            handles.push(thread::spawn(move || {
                let pane = make_pane(&format!("pane-{i}"), "sess-1");
                cache.upsert_pane(pane);
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(cache.pane_count(), 10);
    }

    #[test]
    fn concurrent_event_recording() {
        use std::sync::Arc;
        use std::thread;

        let cache = Arc::new(Cache::new(100));
        let mut handles = vec![];

        for i in 0..20 {
            let cache = Arc::clone(&cache);
            handles.push(thread::spawn(move || {
                cache.record_event(EventRecord {
                    event_id: Some(i),
                    session_uid: format!("s-{i}"),
                    pane_uid: format!("p-{i}"),
                    event_type: "compact".to_string(),
                    detected_at: i,
                    severity: None,
                    status: None,
                });
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(cache.event_count(), 20);
    }

    // --- Large snapshot ---

    #[test]
    fn apply_snapshot_with_many_sessions() {
        let cache = Cache::new(200);
        let sessions: Vec<Session> = (0..100)
            .map(|i| make_session(&format!("sess-{i}"), &format!("name-{i}")))
            .collect();
        let panes: Vec<Pane> = (0..200)
            .map(|i| make_pane(&format!("pane-{i}"), &format!("sess-{}", i / 2)))
            .collect();

        let snapshot = CacheSnapshot {
            sessions,
            panes,
            events: vec![],
            stats_today: StatsAggregate {
                total_compacts: 50,
                active_minutes: 300,
                estimated_tokens: 100_000,
            },
            health: HealthStatus {
                status: "ok".to_string(),
                last_error: None,
            },
        };

        cache.apply_snapshot(snapshot);
        assert_eq!(cache.session_count(), 100);
        assert_eq!(cache.pane_count(), 200);
        assert_eq!(cache.stats_today().total_compacts, 50);
    }

    // --- Metrics accuracy ---

    #[test]
    fn metrics_accuracy_after_multiple_operations() {
        let cache = Cache::new(10);
        // Miss
        cache.get_session("no-exist");
        cache.get_session("no-exist-2");
        // Add then hit
        cache.upsert_session(make_session("s1", "alpha"));
        cache.get_session("s1");
        cache.get_session("s1");

        let m = cache.metrics();
        assert_eq!(m.session_misses, 2);
        assert_eq!(m.session_hits, 2);

        // Pane metrics
        cache.get_pane("no-pane");
        cache.upsert_pane(make_pane("p1", "s1"));
        cache.get_pane("p1");
        let m = cache.metrics();
        assert_eq!(m.pane_misses, 1);
        assert_eq!(m.pane_hits, 1);
    }
}
