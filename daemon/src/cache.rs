use crate::models::pane::Pane;
use crate::models::session::Session;
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

    pub fn metrics(&self) -> CacheMetrics {
        CacheMetrics {
            session_hits: self.session_hits.load(Ordering::Relaxed),
            session_misses: self.session_misses.load(Ordering::Relaxed),
            pane_hits: self.pane_hits.load(Ordering::Relaxed),
            pane_misses: self.pane_misses.load(Ordering::Relaxed),
        }
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

    #[test]
    fn cache_metrics_track_hits() {
        let cache = Cache::new(10);
        assert!(cache.get_session("missing").is_none());

        cache.upsert_session(Session {
            session_uid: "sess-1".to_string(),
            source_id: "src".to_string(),
            tmux_session_id: None,
            name: "alpha".to_string(),
            created_at: 1,
            last_seen_at: 1,
            ended_at: None,
            status: crate::models::session::SessionStatus::Active,
            status_reason: None,
            pane_count: 0,
            metadata: None,
        });

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
}
