use crate::bus::{EventBus, StateChange};
use crate::cache::Cache;
use crate::command::{CommandCategory, CommandRunner, CommandSpec};
use crate::models::pane::{Pane, PaneStatus};
use crate::models::session::{Session, SessionStatus};
use crate::parsers::tmux_panes::{parse_tmux_panes, TmuxPaneMeta};
use std::collections::HashMap;
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
            format: "#{session_id}:#{window_id}:#{pane_id}:#{pane_index}:#{pane_pid}:#{pane_current_command}:#{pane_last_activity}:#{pane_dead}:#{pane_in_mode}".to_string(),
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
    cache: Cache,
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
        cache: Cache,
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

            let session = Session {
                session_uid: session_uid.clone(),
                source_id: "tmux".to_string(),
                tmux_session_id: Some(meta.session_id.clone()),
                name: meta.session_id.clone(),
                created_at: meta.pane_last_activity,
                last_seen_at: meta.pane_last_activity,
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
                created_at: meta.pane_last_activity,
                last_seen_at: meta.pane_last_activity,
                last_activity_at: Some(meta.pane_last_activity),
                current_command: Some(meta.pane_current_command.clone()),
                ended_at: if meta.pane_dead { Some(meta.pane_last_activity) } else { None },
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
        let cache = Cache::new(100);
        let mut collector = TmuxCollector::new(runner, bus, cache, TmuxCollectorConfig::default());

        let meta = TmuxPaneMeta {
            session_id: "$1".to_string(),
            window_id: "@1".to_string(),
            pane_id: "%1".to_string(),
            pane_index: 0,
            pane_pid: 42,
            pane_current_command: "bash".to_string(),
            pane_last_activity: 1,
            pane_dead: false,
            pane_in_mode: false,
        };

        let (changed, removed) = collector.diff_state(&[meta.clone()]);
        assert_eq!(changed, 1);
        assert_eq!(removed, 0);

        let (changed_again, removed_again) = collector.diff_state(&[]);
        assert_eq!(changed_again, 0);
        assert_eq!(removed_again, 1);
    }
}
