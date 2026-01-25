use crate::bus::{EventBus, StateChange};
use crate::cache::{Cache, HealthStatus};
use crate::metrics::{Timer, METRICS};
use crate::ntm::{NtmClient, NtmError};
use crate::reconcile::reconcile_ntm_markdown;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct NtmCollectorConfig {
    pub active_interval: Duration,
    pub idle_interval: Duration,
    pub idle_threshold_secs: i64,
}

impl Default for NtmCollectorConfig {
    fn default() -> Self {
        Self {
            active_interval: Duration::from_secs(15),
            idle_interval: Duration::from_secs(60),
            idle_threshold_secs: 300,
        }
    }
}

#[derive(Clone, Debug)]
pub struct NtmPollResult {
    pub changed: usize,
    pub ended: usize,
    pub degraded: bool,
    pub next_interval: Duration,
}

pub struct NtmCollector {
    client: NtmClient,
    bus: EventBus,
    cache: Cache,
    config: NtmCollectorConfig,
    session_uid_by_name: HashMap<String, String>,
    pane_uid_by_key: HashMap<String, String>,
    failure_count: u32,
}

impl NtmCollector {
    pub fn new(
        client: NtmClient,
        bus: EventBus,
        cache: Cache,
        config: NtmCollectorConfig,
    ) -> Self {
        Self {
            client,
            bus,
            cache,
            config,
            session_uid_by_name: HashMap::new(),
            pane_uid_by_key: HashMap::new(),
            failure_count: 0,
        }
    }

    pub async fn poll_once(&mut self) -> Result<NtmPollResult, String> {
        let _timer = Timer::new(&METRICS.poll_cycle);
        let now = current_unix_ts();
        let fallback_interval = self.next_interval(now);

        let markdown = match self.client.robot_markdown().await {
            Ok(markdown) => markdown,
            Err(err) => {
                self.failure_count = self.failure_count.saturating_add(1);
                let degraded = matches!(err, NtmError::Unavailable) || self.failure_count >= 3;
                let health = HealthStatus {
                    status: if degraded { "degraded" } else { "ok" }.to_string(),
                    last_error: Some(format!("ntm: {err:?}")),
                };
                self.cache.set_health(health);
                return Ok(NtmPollResult {
                    changed: 0,
                    ended: 0,
                    degraded,
                    next_interval: fallback_interval,
                });
            }
        };

        self.failure_count = 0;
        self.cache.set_health(HealthStatus {
            status: "ok".to_string(),
            last_error: None,
        });

        let reconcile = reconcile_ntm_markdown(
            &self.cache,
            &markdown,
            now,
            &mut self.session_uid_by_name,
            &mut self.pane_uid_by_key,
        );

        for session in reconcile.sessions.iter().cloned() {
            self.cache.upsert_session(session);
        }
        for pane in reconcile.panes.iter().cloned() {
            self.cache.upsert_pane(pane);
        }

        let changed = reconcile.change_count();
        if changed > 0 {
            let change = StateChange {
                sessions: reconcile.sessions.clone(),
                panes: reconcile.panes.clone(),
                observed_at: now,
            };
            let _ = self.bus.publish_state(change);
        }

        Ok(NtmPollResult {
            changed,
            ended: reconcile.ended_sessions,
            degraded: false,
            next_interval: self.next_interval(now),
        })
    }

    fn next_interval(&self, now: i64) -> Duration {
        let active = self
            .cache
            .all_sessions()
            .iter()
            .any(|session| {
                session.ended_at.is_none()
                    && now.saturating_sub(session.last_seen_at) <= self.config.idle_threshold_secs
            });
        if active {
            self.config.active_interval
        } else {
            self.config.idle_interval
        }
    }
}

fn current_unix_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs() as i64
}
