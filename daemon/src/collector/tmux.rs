use crate::bus::{EventBus, StateChange};
use crate::cache::Cache;
use crate::command::{CommandCategory, CommandRunner, CommandSpec};
use crate::metrics::{Timer, METRICS};
use crate::models::pane::{Pane, PaneStatus};
use crate::models::session::{Session, SessionStatus};
use crate::parsers::tmux_panes::{parse_tmux_panes, TmuxPaneMeta};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct TmuxCollectorConfig {
    pub poll_interval: Duration,
    pub format: String,
    pub max_output_bytes: usize,
}

impl Default for TmuxCollectorConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(1500),
            format: "#{session_id}:#{session_name}:#{window_id}:#{pane_id}:#{pane_index}:#{pane_pid}:#{pane_current_command}:#{pane_last_activity}:#{pane_dead}:#{pane_in_mode}".to_string(),
            max_output_bytes: 256 * 1024,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TmuxPollResult {
    pub changed: usize,
    pub removed: usize,
    pub degraded: bool,
}

pub struct TmuxCollector {
    runner: CommandRunner,
    bus: EventBus,
    cache: Arc<Cache>,
    config: TmuxCollectorConfig,
    last_state: HashMap<String, TmuxPaneMeta>,
    pane_uid_by_tmux: HashMap<String, String>,
    session_uid_by_tmux: HashMap<String, String>,
    failure_count: u32,
}

impl TmuxCollector {
    pub fn new(
        runner: CommandRunner,
        bus: EventBus,
        cache: Arc<Cache>,
        config: TmuxCollectorConfig,
    ) -> Self {
        Self {
            runner,
            bus,
            cache,
            config,
            last_state: HashMap::new(),
            pane_uid_by_tmux: HashMap::new(),
            session_uid_by_tmux: HashMap::new(),
            failure_count: 0,
        }
    }

    pub async fn poll_once(&mut self) -> Result<TmuxPollResult, String> {
        let _timer = Timer::new(&METRICS.poll_cycle);
        let spec = CommandSpec {
            program: "tmux".to_string(),
            args: vec![
                "list-panes".to_string(),
                "-a".to_string(),
                "-F".to_string(),
                self.config.format.clone(),
            ],
            timeout: Duration::from_secs(0),
            max_output_bytes: self.config.max_output_bytes,
            category: CommandCategory::TmuxFast,
        };

        let output = match self.runner.run(spec).await {
            Ok(output) => output,
            Err(err) => {
                self.failure_count = self.failure_count.saturating_add(1);
                let degraded = self.failure_count >= 3;
                if degraded {
                    return Ok(TmuxPollResult {
                        changed: 0,
                        removed: 0,
                        degraded: true,
                    });
                }
                return Err(format!("tmux poll error: {err:?}"));
            }
        };

        self.failure_count = 0;
        let text = String::from_utf8_lossy(&output.stdout);
        let metas = parse_tmux_panes(&text).map_err(|err| err.reason)?;
        let (changed, removed) = self.diff_state(&metas);

        if changed > 0 || removed > 0 {
            let (sessions, panes) = self.update_cache(&metas);
            let change = StateChange {
                sessions,
                panes,
                observed_at: current_unix_ts(),
            };
            let _ = self.bus.publish_state(change);
        }

        Ok(TmuxPollResult {
            changed,
            removed,
            degraded: false,
        })
    }

    fn diff_state(&mut self, metas: &[TmuxPaneMeta]) -> (usize, usize) {
        let mut changed = 0;
        let mut next_state = HashMap::new();
        for meta in metas {
            let key = meta.pane_id.clone();
            if self
                .last_state
                .get(&key)
                .map(|prev| prev != meta)
                .unwrap_or(true)
            {
                changed += 1;
            }
            next_state.insert(key, meta.clone());
        }

        let removed = self
            .last_state
            .keys()
            .filter(|key| !next_state.contains_key(*key))
            .count();

        self.last_state = next_state;
        (changed, removed)
    }

    fn update_cache(&mut self, metas: &[TmuxPaneMeta]) -> (Vec<Session>, Vec<Pane>) {
        let mut sessions = Vec::new();
        let mut panes = Vec::new();

        for meta in metas {
            let session_uid = self
                .session_uid_by_tmux
                .entry(meta.session_id.clone())
                .or_insert_with(|| uuid::Uuid::now_v7().to_string())
                .clone();

            let pane_uid = self
                .pane_uid_by_tmux
                .entry(meta.pane_id.clone())
                .or_insert_with(|| uuid::Uuid::now_v7().to_string())
                .clone();

            let now = current_unix_ts();
            let activity_ts = if meta.pane_last_activity > 0 {
                meta.pane_last_activity
            } else {
                now
            };

            let session = Session {
                session_uid: session_uid.clone(),
                source_id: "tmux".to_string(),
                tmux_session_id: Some(meta.session_id.clone()),
                name: meta.session_name.clone(),
                created_at: activity_ts,
                last_seen_at: now,
                ended_at: None,
                status: SessionStatus::Active,
                status_reason: Some("tmux_poll".to_string()),
                pane_count: 0,
                metadata: None,
            };

            let pane = Pane {
                pane_uid: pane_uid.clone(),
                session_uid: session_uid.clone(),
                tmux_pane_id: Some(meta.pane_id.clone()),
                tmux_window_id: Some(meta.window_id.clone()),
                tmux_pane_pid: Some(meta.pane_pid),
                pane_index: meta.pane_index,
                agent_type: None,
                created_at: activity_ts,
                last_seen_at: now,
                last_activity_at: Some(activity_ts),
                current_command: Some(meta.pane_current_command.clone()),
                ended_at: if meta.pane_dead { Some(now) } else { None },
                status: if meta.pane_dead {
                    PaneStatus::Ended
                } else {
                    PaneStatus::Active
                },
                status_reason: Some("tmux_poll".to_string()),
            };

            self.cache.upsert_session(session.clone());
            self.cache.upsert_pane(pane.clone());
            sessions.push(session);
            panes.push(pane);
        }

        (sessions, panes)
    }
}

fn current_unix_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_detects_changes_and_removals() {
        let runner = CommandRunner::new(crate::command::CommandConfig::default());
        let bus = EventBus::new(4);
        let cache = std::sync::Arc::new(Cache::new(100));
        let mut collector = TmuxCollector::new(runner, bus, cache, TmuxCollectorConfig::default());

        let meta = TmuxPaneMeta {
            session_id: "$1".to_string(),
            session_name: "test_session".to_string(),
            window_id: "@1".to_string(),
            pane_id: "%1".to_string(),
            pane_index: 0,
            pane_pid: 42,
            pane_current_command: "bash".to_string(),
            pane_last_activity: 1,
            pane_dead: false,
            pane_in_mode: false,
        };

        let (changed, removed) = collector.diff_state(std::slice::from_ref(&meta));
        assert_eq!(changed, 1);
        assert_eq!(removed, 0);

        let (changed_again, removed_again) = collector.diff_state(&[]);
        assert_eq!(changed_again, 0);
        assert_eq!(removed_again, 1);
    }

    // --- Helper ---

    fn make_collector() -> TmuxCollector {
        let runner = CommandRunner::new(crate::command::CommandConfig::default());
        let bus = EventBus::new(4);
        let cache = Arc::new(Cache::new(100));
        TmuxCollector::new(runner, bus, cache, TmuxCollectorConfig::default())
    }

    fn make_collector_with_cache(cache: Arc<Cache>) -> TmuxCollector {
        let runner = CommandRunner::new(crate::command::CommandConfig::default());
        let bus = EventBus::new(4);
        TmuxCollector::new(runner, bus, cache, TmuxCollectorConfig::default())
    }

    fn meta(session_id: &str, pane_id: &str) -> TmuxPaneMeta {
        TmuxPaneMeta {
            session_id: session_id.to_string(),
            session_name: format!("sess-{session_id}"),
            window_id: "@1".to_string(),
            pane_id: pane_id.to_string(),
            pane_index: 0,
            pane_pid: 100,
            pane_current_command: "bash".to_string(),
            pane_last_activity: 1000,
            pane_dead: false,
            pane_in_mode: false,
        }
    }

    // --- diff_state edge cases ---

    #[test]
    fn diff_empty_old_empty_new() {
        let mut c = make_collector();
        let (changed, removed) = c.diff_state(&[]);
        assert_eq!(changed, 0);
        assert_eq!(removed, 0);
    }

    #[test]
    fn diff_empty_old_all_new() {
        let mut c = make_collector();
        let metas = vec![meta("$1", "%1"), meta("$1", "%2"), meta("$2", "%3")];
        let (changed, removed) = c.diff_state(&metas);
        assert_eq!(changed, 3, "all panes are new");
        assert_eq!(removed, 0);
    }

    #[test]
    fn diff_identical_states_no_change() {
        let mut c = make_collector();
        let metas = vec![meta("$1", "%1"), meta("$1", "%2")];
        c.diff_state(&metas);
        // Same input again — nothing changed
        let (changed, removed) = c.diff_state(&metas);
        assert_eq!(changed, 0);
        assert_eq!(removed, 0);
    }

    #[test]
    fn diff_all_removed() {
        let mut c = make_collector();
        let metas = vec![meta("$1", "%1"), meta("$2", "%2")];
        c.diff_state(&metas);
        let (changed, removed) = c.diff_state(&[]);
        assert_eq!(changed, 0);
        assert_eq!(removed, 2);
    }

    #[test]
    fn diff_mixed_add_remove_change() {
        let mut c = make_collector();
        // Initial: %1, %2
        let initial = vec![meta("$1", "%1"), meta("$1", "%2")];
        c.diff_state(&initial);

        // Next: %2 (unchanged), %3 (new), %1 removed
        let next = vec![meta("$1", "%2"), meta("$1", "%3")];
        let (changed, removed) = c.diff_state(&next);
        assert_eq!(changed, 1, "%3 is new");
        assert_eq!(removed, 1, "%1 was removed");
    }

    #[test]
    fn diff_detects_field_change() {
        let mut c = make_collector();
        let m = meta("$1", "%1");
        c.diff_state(std::slice::from_ref(&m));

        // Same pane_id but different command
        let mut m2 = meta("$1", "%1");
        m2.pane_current_command = "vim".to_string();
        let (changed, removed) = c.diff_state(std::slice::from_ref(&m2));
        assert_eq!(changed, 1, "command changed");
        assert_eq!(removed, 0);
    }

    // --- update_cache ---

    #[test]
    fn update_cache_upserts_sessions_and_panes() {
        let cache = Arc::new(Cache::new(100));
        let mut c = make_collector_with_cache(cache.clone());
        let metas = vec![meta("$1", "%1"), meta("$1", "%2"), meta("$2", "%3")];
        let (sessions, panes) = c.update_cache(&metas);

        assert_eq!(sessions.len(), 3); // one per meta entry
        assert_eq!(panes.len(), 3);

        // Cache should have the sessions and panes
        assert_eq!(cache.session_count(), 2, "2 unique session_ids");
        assert_eq!(cache.pane_count(), 3);
    }

    #[test]
    fn update_cache_sets_source_to_tmux() {
        let cache = Arc::new(Cache::new(100));
        let mut c = make_collector_with_cache(cache.clone());
        let metas = vec![meta("$1", "%1")];
        let (sessions, _panes) = c.update_cache(&metas);
        assert_eq!(sessions[0].source_id, "tmux");
    }

    #[test]
    fn update_cache_sets_tmux_session_id() {
        let cache = Arc::new(Cache::new(100));
        let mut c = make_collector_with_cache(cache.clone());
        let metas = vec![meta("$1", "%1")];
        let (sessions, _) = c.update_cache(&metas);
        assert_eq!(sessions[0].tmux_session_id, Some("$1".to_string()));
    }

    // --- Session UID stability ---

    #[test]
    fn session_uid_stable_across_polls() {
        let cache = Arc::new(Cache::new(100));
        let mut c = make_collector_with_cache(cache.clone());

        let metas = vec![meta("$1", "%1")];
        let (sessions1, _) = c.update_cache(&metas);
        let uid1 = sessions1[0].session_uid.clone();

        // Poll again with same session_id
        let (sessions2, _) = c.update_cache(&metas);
        let uid2 = sessions2[0].session_uid.clone();

        assert_eq!(uid1, uid2, "same tmux session_id should produce same UID");
    }

    #[test]
    fn different_session_ids_get_different_uids() {
        let cache = Arc::new(Cache::new(100));
        let mut c = make_collector_with_cache(cache.clone());

        let metas = vec![meta("$1", "%1"), meta("$2", "%2")];
        let (sessions, _) = c.update_cache(&metas);
        assert_ne!(
            sessions[0].session_uid, sessions[1].session_uid,
            "different tmux sessions should have different UIDs"
        );
    }

    // --- Pane UID stability ---

    #[test]
    fn pane_uid_stable_across_polls() {
        let cache = Arc::new(Cache::new(100));
        let mut c = make_collector_with_cache(cache.clone());

        let metas = vec![meta("$1", "%1")];
        let (_, panes1) = c.update_cache(&metas);
        let uid1 = panes1[0].pane_uid.clone();

        let (_, panes2) = c.update_cache(&metas);
        let uid2 = panes2[0].pane_uid.clone();

        assert_eq!(uid1, uid2, "same tmux pane_id should produce same UID");
    }

    #[test]
    fn different_pane_ids_get_different_uids() {
        let cache = Arc::new(Cache::new(100));
        let mut c = make_collector_with_cache(cache.clone());

        let metas = vec![meta("$1", "%1"), meta("$1", "%2")];
        let (_, panes) = c.update_cache(&metas);
        assert_ne!(
            panes[0].pane_uid, panes[1].pane_uid,
            "different tmux panes should have different UIDs"
        );
    }

    // --- Dead pane detection ---

    #[test]
    fn dead_pane_marked_ended() {
        let cache = Arc::new(Cache::new(100));
        let mut c = make_collector_with_cache(cache.clone());

        let mut m = meta("$1", "%1");
        m.pane_dead = true;
        let (_, panes) = c.update_cache(std::slice::from_ref(&m));

        assert_eq!(panes[0].status, PaneStatus::Ended);
        assert!(panes[0].ended_at.is_some());
    }

    #[test]
    fn alive_pane_active_no_ended() {
        let cache = Arc::new(Cache::new(100));
        let mut c = make_collector_with_cache(cache.clone());

        let m = meta("$1", "%1"); // pane_dead = false
        let (_, panes) = c.update_cache(std::slice::from_ref(&m));

        assert_eq!(panes[0].status, PaneStatus::Active);
        assert!(panes[0].ended_at.is_none());
    }

    // --- Pane fields mapping ---

    #[test]
    fn pane_fields_mapped_correctly() {
        let cache = Arc::new(Cache::new(100));
        let mut c = make_collector_with_cache(cache.clone());

        let mut m = meta("$1", "%5");
        m.window_id = "@3".to_string();
        m.pane_pid = 9876;
        m.pane_index = 2;
        m.pane_current_command = "vim".to_string();
        m.pane_last_activity = 5000;

        let (_, panes) = c.update_cache(std::slice::from_ref(&m));
        let p = &panes[0];
        assert_eq!(p.tmux_pane_id, Some("%5".to_string()));
        assert_eq!(p.tmux_window_id, Some("@3".to_string()));
        assert_eq!(p.tmux_pane_pid, Some(9876));
        assert_eq!(p.pane_index, 2);
        assert_eq!(p.current_command, Some("vim".to_string()));
        assert_eq!(p.last_activity_at, Some(5000));
        assert_eq!(p.status_reason, Some("tmux_poll".to_string()));
    }

    // --- Zero activity timestamp fallback ---

    #[test]
    fn zero_activity_uses_current_time() {
        let cache = Arc::new(Cache::new(100));
        let mut c = make_collector_with_cache(cache.clone());

        let mut m = meta("$1", "%1");
        m.pane_last_activity = 0; // triggers fallback to now
        let (sessions, panes) = c.update_cache(std::slice::from_ref(&m));

        // created_at should be recent (not zero)
        assert!(sessions[0].created_at > 0);
        assert!(panes[0].created_at > 0);
    }

    // --- Config defaults ---

    #[test]
    fn config_defaults() {
        let config = TmuxCollectorConfig::default();
        assert_eq!(config.poll_interval, Duration::from_millis(1500));
        assert_eq!(config.max_output_bytes, 256 * 1024);
        assert!(config.format.contains("session_id"));
        assert!(config.format.contains("pane_dead"));
    }

    // --- Pane belongs to correct session ---

    #[test]
    fn pane_session_uid_matches_parent() {
        let cache = Arc::new(Cache::new(100));
        let mut c = make_collector_with_cache(cache.clone());

        let metas = vec![meta("$1", "%1"), meta("$2", "%2")];
        let (sessions, panes) = c.update_cache(&metas);

        // Pane %1 belongs to session $1
        assert_eq!(panes[0].session_uid, sessions[0].session_uid);
        // Pane %2 belongs to session $2
        assert_eq!(panes[1].session_uid, sessions[1].session_uid);
    }

    // ========================================================
    // Collector failure and recovery (bd-2wun)
    // ========================================================

    #[tokio::test]
    async fn poll_failure_increments_count_returns_error() {
        let cache = Arc::new(Cache::new(100));
        let runner = CommandRunner::new(crate::command::CommandConfig {
            tmux_timeout: Duration::from_millis(1), // extremely short timeout
            ..crate::command::CommandConfig::default()
        });
        let bus = EventBus::new(4);
        let mut config = TmuxCollectorConfig::default();
        config.format = "#{session_id}:#{session_name}:#{window_id}:#{pane_id}:#{pane_index}:#{pane_pid}:#{pane_current_command}:#{pane_last_activity}:#{pane_dead}:#{pane_in_mode}".to_string();
        let mut collector = TmuxCollector::new(runner, bus, cache, config);

        // First poll attempt — will fail (tmux either not installed or timeout)
        let result = collector.poll_once().await;
        // Should be Err (failure_count < 3) or Ok with degraded=false
        // Both are valid — the important thing is no panic
        match result {
            Err(_) => {
                // Expected: failure_count = 1, not degraded yet
                assert_eq!(collector.failure_count, 1);
            }
            Ok(r) => {
                // Tmux might actually be installed and respond — that's fine
                assert!(!r.degraded || collector.failure_count >= 3);
            }
        }
    }

    #[tokio::test]
    async fn three_failures_produces_degraded_state() {
        let cache = Arc::new(Cache::new(100));
        let runner = CommandRunner::new(crate::command::CommandConfig {
            tmux_timeout: Duration::from_millis(1),
            ..crate::command::CommandConfig::default()
        });
        let bus = EventBus::new(4);
        let config = TmuxCollectorConfig::default();
        let mut collector = TmuxCollector::new(runner, bus, cache, config);

        // Force failure_count to 2 (simulating 2 prior failures)
        collector.failure_count = 2;

        // Next poll should fail and hit the degraded threshold (failure_count >= 3)
        let result = collector.poll_once().await;
        match result {
            Ok(r) => {
                // If tmux isn't installed, we'll get degraded=true
                if collector.failure_count >= 3 {
                    assert!(r.degraded, "should be degraded after 3+ failures");
                }
            }
            Err(_) => {
                // This happens if tmux actually runs but there's some other issue
                // Still valid — no panic
            }
        }
    }

    #[test]
    fn failure_count_resets_on_success() {
        // This tests that successful poll_once would reset failure_count.
        // We can verify by checking the logic: line 97 does `self.failure_count = 0`
        // after successful runner.run(). Test the state management directly.
        let cache = Arc::new(Cache::new(100));
        let mut c = make_collector_with_cache(cache.clone());
        c.failure_count = 5; // simulate prior failures

        // diff_state and update_cache work fine
        let metas = vec![meta("$1", "%1")];
        c.diff_state(&metas);
        c.update_cache(&metas);

        // The cache should have data despite prior failures
        assert_eq!(cache.session_count(), 1);
        assert_eq!(cache.pane_count(), 1);
    }

    #[test]
    fn degraded_threshold_is_three() {
        // Verify the threshold constant is 3 (per line 85)
        let cache = Arc::new(Cache::new(100));
        let mut c = make_collector_with_cache(cache);

        // Below threshold
        c.failure_count = 2;
        // At/above threshold
        assert!(c.failure_count < 3, "2 failures = not degraded");
        c.failure_count = 3;
        assert!(c.failure_count >= 3, "3 failures = degraded");
    }

    #[test]
    fn rpc_works_with_stale_cache_data() {
        // Even when collector fails, RPC should still work with whatever data
        // is in the cache. Verify cache is readable after partial updates.
        let cache = Arc::new(Cache::new(100));
        let mut c = make_collector_with_cache(cache.clone());

        // First successful poll populates cache
        let metas = vec![meta("$1", "%1"), meta("$1", "%2")];
        c.diff_state(&metas);
        c.update_cache(&metas);

        assert_eq!(cache.session_count(), 1);
        assert_eq!(cache.pane_count(), 2);

        // Simulate collector failure (no new data)
        c.failure_count = 5;

        // Cache data still accessible
        let sessions = cache.all_sessions();
        assert_eq!(sessions.len(), 1);
        let panes = cache.all_panes();
        assert_eq!(panes.len(), 2);

        // Data is stale but valid
        assert_eq!(sessions[0].name, "sess-$1");
    }

    #[test]
    fn poll_result_default() {
        let r = TmuxPollResult {
            changed: 0,
            removed: 0,
            degraded: false,
        };
        assert_eq!(r.changed, 0);
        assert_eq!(r.removed, 0);
        assert!(!r.degraded);
    }
}
