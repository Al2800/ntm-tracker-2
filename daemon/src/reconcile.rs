use crate::cache::Cache;
use crate::models::pane::{Pane, PaneStatus};
use crate::models::session::{Session, SessionStatus};
use crate::parsers::ntm_markdown::{NtmMarkdown, NtmSession};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default)]
pub struct ReconcileResult {
    pub sessions: Vec<Session>,
    pub panes: Vec<Pane>,
    pub ended_sessions: usize,
}

impl ReconcileResult {
    pub fn change_count(&self) -> usize {
        self.sessions.len() + self.panes.len()
    }
}

pub fn reconcile_ntm_markdown(
    cache: &Cache,
    markdown: &NtmMarkdown,
    now: i64,
    session_uid_by_name: &mut HashMap<String, String>,
    pane_uid_by_key: &mut HashMap<String, String>,
) -> ReconcileResult {
    let existing_sessions = cache.all_sessions();
    let mut session_by_name = HashMap::new();
    let mut session_name_by_uid = HashMap::new();
    for session in &existing_sessions {
        session_by_name.insert(session.name.clone(), session.clone());
        session_name_by_uid.insert(session.session_uid.clone(), session.name.clone());
        session_uid_by_name
            .entry(session.name.clone())
            .or_insert_with(|| session.session_uid.clone());
    }

    if pane_uid_by_key.is_empty() {
        for pane in cache.all_panes() {
            if let Some(session_name) = session_name_by_uid.get(&pane.session_uid) {
                let key = format!("{}:{}", session_name, pane.pane_index);
                pane_uid_by_key.entry(key).or_insert(pane.pane_uid.clone());
            }
        }
    }

    let mut sessions_out: HashMap<String, Session> = HashMap::new();
    let mut panes_out: Vec<Pane> = Vec::new();
    let mut seen_sessions: HashSet<String> = HashSet::new();
    let mut pane_counts: HashMap<String, u32> = HashMap::new();

    for session in &markdown.sessions {
        let session = upsert_session(
            session,
            now,
            &session_by_name,
            session_uid_by_name,
        );
        seen_sessions.insert(session.name.clone());
        sessions_out.insert(session.name.clone(), session);
    }

    for pane in &markdown.panes {
        let session_name = pane.session.clone();
        let session_uid = session_uid_by_name
            .entry(session_name.clone())
            .or_insert_with(|| {
                session_by_name
                    .get(&session_name)
                    .map(|session| session.session_uid.clone())
                    .unwrap_or_else(|| Session::new("ntm", session_name.clone(), None, now).session_uid)
            })
            .clone();

        if !seen_sessions.contains(&session_name) {
            let fallback_session = NtmSession {
                name: session_name.clone(),
                status: None,
                metadata: HashMap::new(),
            };
            let session = upsert_session(
                &fallback_session,
                now,
                &session_by_name,
                session_uid_by_name,
            );
            seen_sessions.insert(session.name.clone());
            sessions_out.insert(session.name.clone(), session);
        }

        let pane_index = parse_pane_index(&pane.pane);
        let pane_key = format!("{}:{}", session_name, pane.pane);
        let numeric_key = format!("{}:{}", session_name, pane_index);
        let pane_uid = pane_uid_by_key
            .get(&pane_key)
            .or_else(|| pane_uid_by_key.get(&numeric_key))
            .cloned()
            .unwrap_or_else(|| uuid::Uuid::now_v7().to_string());
        pane_uid_by_key.entry(pane_key).or_insert(pane_uid.clone());
        pane_uid_by_key.entry(numeric_key).or_insert(pane_uid.clone());

        let mut pane_record = cache
            .get_pane(&pane_uid)
            .unwrap_or_else(|| Pane::new(session_uid.clone(), pane_index, now, None, None, None));
        pane_record.pane_uid = pane_uid.clone();
        pane_record.session_uid = session_uid.clone();
        pane_record.pane_index = pane_index;
        pane_record.last_seen_at = now;
        if let Some(agent_type) = pane.agent.clone() {
            if !agent_type.is_empty() {
                pane_record.agent_type = Some(agent_type);
            }
        }
        if let Some(status) = map_pane_status(&pane.status) {
            pane_record.status = status;
            pane_record.status_reason = Some("ntm_status".to_string());
        }
        if let Some(command) = extract_metadata(&pane.metadata, &["command", "cmd", "current_command"]) {
            pane_record.current_command = Some(command);
        }

        panes_out.push(pane_record);
        *pane_counts.entry(session_uid).or_insert(0) += 1;
    }

    let mut ended_sessions = 0;
    for (session_name, _session_uid) in session_uid_by_name.iter() {
        if seen_sessions.contains(session_name) {
            continue;
        }
        let Some(mut session) = session_by_name.get(session_name).cloned() else {
            continue;
        };
        if session.source_id != "ntm" || session.ended_at.is_some() {
            continue;
        }
        session.ended_at = Some(now);
        session.status = SessionStatus::Ended;
        session.status_reason = Some("ntm_missing".to_string());
        sessions_out.insert(session_name.clone(), session);
        ended_sessions += 1;
    }

    for session in sessions_out.values_mut() {
        if let Some(count) = pane_counts.get(&session.session_uid) {
            session.pane_count = *count;
        }
    }

    ReconcileResult {
        sessions: sessions_out.into_values().collect(),
        panes: panes_out,
        ended_sessions,
    }
}

fn upsert_session(
    session: &NtmSession,
    now: i64,
    session_by_name: &HashMap<String, Session>,
    session_uid_by_name: &mut HashMap<String, String>,
) -> Session {
    let session_uid = session_uid_by_name
        .entry(session.name.clone())
        .or_insert_with(|| {
            session_by_name
                .get(&session.name)
                .map(|session| session.session_uid.clone())
                .unwrap_or_else(|| uuid::Uuid::now_v7().to_string())
        })
        .clone();
    let mut record = session_by_name
        .get(&session.name)
        .cloned()
        .unwrap_or_else(|| Session::new("ntm", session.name.clone(), None, now));
    record.session_uid = session_uid;
    record.last_seen_at = now;
    record.ended_at = None;
    if let Some(status) = map_session_status(&session.status) {
        record.status = status;
        record.status_reason = Some("ntm_status".to_string());
    }
    if !session.metadata.is_empty() {
        record.metadata = Some(metadata_to_value(&session.metadata));
    }
    record
}

fn parse_pane_index(value: &str) -> i32 {
    value.trim().parse::<i32>().unwrap_or(0)
}

fn map_session_status(status: &Option<String>) -> Option<SessionStatus> {
    let value = status.as_deref()?.trim().to_lowercase();
    match value.as_str() {
        "active" | "running" | "working" => Some(SessionStatus::Active),
        "idle" => Some(SessionStatus::Idle),
        "ended" | "stopped" | "dead" => Some(SessionStatus::Ended),
        _ => None,
    }
}

fn map_pane_status(status: &Option<String>) -> Option<PaneStatus> {
    let value = status.as_deref()?.trim().to_lowercase();
    match value.as_str() {
        "active" | "running" | "working" => Some(PaneStatus::Active),
        "waiting" => Some(PaneStatus::Waiting),
        "idle" => Some(PaneStatus::Idle),
        "ended" | "stopped" | "dead" => Some(PaneStatus::Ended),
        _ => None,
    }
}

fn metadata_to_value(metadata: &HashMap<String, String>) -> Value {
    let mut map = Map::new();
    for (key, value) in metadata {
        map.insert(key.clone(), Value::String(value.clone()));
    }
    Value::Object(map)
}

fn extract_metadata(metadata: &HashMap<String, String>, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = metadata.get(&key.to_lowercase()) {
            if !value.is_empty() {
                return Some(value.clone());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::Cache;
    use crate::models::pane::Pane;
    use crate::models::session::{Session, SessionStatus};
    use crate::parsers::ntm_markdown::{NtmMarkdown, NtmPane, NtmSession};
    use std::collections::HashMap;

    #[test]
    fn marks_missing_ntm_sessions_as_ended() {
        let cache = Cache::new(128);
        let now = 1_700_000_000;

        let mut alpha = Session::new("ntm", "alpha", None, now - 60);
        alpha.session_uid = "alpha_uid".to_string();
        cache.upsert_session(alpha);

        let mut beta = Session::new("ntm", "beta", None, now - 60);
        beta.session_uid = "beta_uid".to_string();
        cache.upsert_session(beta);

        let markdown = NtmMarkdown {
            sessions: vec![NtmSession {
                name: "alpha".to_string(),
                status: Some("active".to_string()),
                metadata: HashMap::new(),
            }],
            panes: vec![NtmPane {
                session: "alpha".to_string(),
                pane: "0".to_string(),
                status: Some("active".to_string()),
                agent: None,
                metadata: HashMap::new(),
            }],
        };

        let mut session_uid_by_name = HashMap::new();
        let mut pane_uid_by_key = HashMap::new();
        let result = reconcile_ntm_markdown(
            &cache,
            &markdown,
            now,
            &mut session_uid_by_name,
            &mut pane_uid_by_key,
        );

        assert_eq!(result.ended_sessions, 1);
        let beta_out = result
            .sessions
            .iter()
            .find(|session| session.name == "beta")
            .expect("beta session present");
        assert_eq!(beta_out.status, SessionStatus::Ended);
        assert_eq!(beta_out.ended_at, Some(now));
        assert_eq!(beta_out.status_reason.as_deref(), Some("ntm_missing"));
    }

    #[test]
    fn preserves_pane_uids_using_lookup_tables() {
        let cache = Cache::new(128);
        let now = 1_700_000_000;

        let mut session_uid_by_name = HashMap::new();
        session_uid_by_name.insert("alpha".to_string(), "alpha_uid".to_string());

        let mut pane_uid_by_key = HashMap::new();
        pane_uid_by_key.insert("alpha:0".to_string(), "pane_uid".to_string());

        let mut existing = Pane::new("alpha_uid".to_string(), 0, now - 60, None, None, None);
        existing.pane_uid = "pane_uid".to_string();
        cache.upsert_pane(existing);

        let markdown = NtmMarkdown {
            sessions: vec![],
            panes: vec![NtmPane {
                session: "alpha".to_string(),
                pane: "0".to_string(),
                status: Some("active".to_string()),
                agent: None,
                metadata: HashMap::new(),
            }],
        };

        let result = reconcile_ntm_markdown(
            &cache,
            &markdown,
            now,
            &mut session_uid_by_name,
            &mut pane_uid_by_key,
        );

        assert_eq!(result.panes.len(), 1);
        assert_eq!(result.panes[0].pane_uid, "pane_uid");
    }

    // --- Helpers ---

    fn empty_markdown() -> NtmMarkdown {
        NtmMarkdown {
            sessions: vec![],
            panes: vec![],
        }
    }

    fn make_ntm_session(name: &str, status: Option<&str>) -> NtmSession {
        NtmSession {
            name: name.to_string(),
            status: status.map(|s| s.to_string()),
            metadata: HashMap::new(),
        }
    }

    fn make_ntm_pane(session: &str, pane: &str, status: Option<&str>, agent: Option<&str>) -> NtmPane {
        NtmPane {
            session: session.to_string(),
            pane: pane.to_string(),
            status: status.map(|s| s.to_string()),
            agent: agent.map(|a| a.to_string()),
            metadata: HashMap::new(),
        }
    }

    fn reconcile(cache: &Cache, markdown: &NtmMarkdown, now: i64) -> ReconcileResult {
        let mut session_uid_by_name = HashMap::new();
        let mut pane_uid_by_key = HashMap::new();
        reconcile_ntm_markdown(cache, markdown, now, &mut session_uid_by_name, &mut pane_uid_by_key)
    }

    // --- Empty input ---

    #[test]
    fn empty_markdown_produces_empty_result() {
        let cache = Cache::new(128);
        let result = reconcile(&cache, &empty_markdown(), 1000);
        assert_eq!(result.sessions.len(), 0);
        assert_eq!(result.panes.len(), 0);
        assert_eq!(result.ended_sessions, 0);
        assert_eq!(result.change_count(), 0);
    }

    // --- New session discovery ---

    #[test]
    fn new_session_created_from_markdown() {
        let cache = Cache::new(128);
        let now = 1_700_000_000;
        let md = NtmMarkdown {
            sessions: vec![make_ntm_session("new-sess", Some("active"))],
            panes: vec![],
        };
        let result = reconcile(&cache, &md, now);

        assert_eq!(result.sessions.len(), 1);
        let sess = &result.sessions[0];
        assert_eq!(sess.name, "new-sess");
        assert_eq!(sess.status, SessionStatus::Active);
        assert_eq!(sess.last_seen_at, now);
        assert!(sess.ended_at.is_none());
    }

    // --- Session matching by name ---

    #[test]
    fn existing_session_matched_by_name() {
        let cache = Cache::new(128);
        let now = 1_700_000_000;

        let mut existing = Session::new("ntm", "alpha", None, now - 100);
        existing.session_uid = "alpha-uid".to_string();
        cache.upsert_session(existing);

        let md = NtmMarkdown {
            sessions: vec![make_ntm_session("alpha", Some("active"))],
            panes: vec![],
        };

        let mut session_uid_by_name = HashMap::new();
        let mut pane_uid_by_key = HashMap::new();
        let result = reconcile_ntm_markdown(
            &cache, &md, now, &mut session_uid_by_name, &mut pane_uid_by_key,
        );

        assert_eq!(result.sessions.len(), 1);
        assert_eq!(result.sessions[0].session_uid, "alpha-uid", "should reuse existing UID");
        assert_eq!(result.sessions[0].last_seen_at, now);
    }

    // --- Session status mapping ---

    #[test]
    fn session_status_maps_correctly() {
        assert_eq!(map_session_status(&Some("active".to_string())), Some(SessionStatus::Active));
        assert_eq!(map_session_status(&Some("running".to_string())), Some(SessionStatus::Active));
        assert_eq!(map_session_status(&Some("working".to_string())), Some(SessionStatus::Active));
        assert_eq!(map_session_status(&Some("idle".to_string())), Some(SessionStatus::Idle));
        assert_eq!(map_session_status(&Some("ended".to_string())), Some(SessionStatus::Ended));
        assert_eq!(map_session_status(&Some("stopped".to_string())), Some(SessionStatus::Ended));
        assert_eq!(map_session_status(&Some("dead".to_string())), Some(SessionStatus::Ended));
        assert_eq!(map_session_status(&Some("unknown_status".to_string())), None);
        assert_eq!(map_session_status(&None), None);
    }

    // --- Pane status mapping ---

    #[test]
    fn pane_status_maps_correctly() {
        assert_eq!(map_pane_status(&Some("active".to_string())), Some(PaneStatus::Active));
        assert_eq!(map_pane_status(&Some("waiting".to_string())), Some(PaneStatus::Waiting));
        assert_eq!(map_pane_status(&Some("idle".to_string())), Some(PaneStatus::Idle));
        assert_eq!(map_pane_status(&Some("ended".to_string())), Some(PaneStatus::Ended));
        assert_eq!(map_pane_status(&None), None);
    }

    // --- Pane creation ---

    #[test]
    fn pane_created_with_correct_fields() {
        let cache = Cache::new(128);
        let now = 1_700_000_000;
        let md = NtmMarkdown {
            sessions: vec![make_ntm_session("sess", Some("active"))],
            panes: vec![make_ntm_pane("sess", "2", Some("active"), Some("claude"))],
        };
        let result = reconcile(&cache, &md, now);

        assert_eq!(result.panes.len(), 1);
        let p = &result.panes[0];
        assert_eq!(p.pane_index, 2);
        assert_eq!(p.status, PaneStatus::Active);
        assert_eq!(p.agent_type, Some("claude".to_string()));
        assert_eq!(p.last_seen_at, now);
        assert_eq!(p.status_reason, Some("ntm_status".to_string()));
    }

    // --- Pane UID stability ---

    #[test]
    fn pane_uid_stable_across_reconcile_calls() {
        let cache = Cache::new(128);
        let now = 1_700_000_000;
        let md = NtmMarkdown {
            sessions: vec![make_ntm_session("sess", Some("active"))],
            panes: vec![make_ntm_pane("sess", "0", Some("active"), None)],
        };

        let mut session_uid_by_name = HashMap::new();
        let mut pane_uid_by_key = HashMap::new();

        let r1 = reconcile_ntm_markdown(
            &cache, &md, now, &mut session_uid_by_name, &mut pane_uid_by_key,
        );
        let uid1 = r1.panes[0].pane_uid.clone();

        let r2 = reconcile_ntm_markdown(
            &cache, &md, now + 10, &mut session_uid_by_name, &mut pane_uid_by_key,
        );
        let uid2 = r2.panes[0].pane_uid.clone();

        assert_eq!(uid1, uid2, "same pane key should produce same UID");
    }

    // --- Pane count reconciliation ---

    #[test]
    fn session_pane_count_updated() {
        let cache = Cache::new(128);
        let now = 1_700_000_000;
        let md = NtmMarkdown {
            sessions: vec![make_ntm_session("sess", Some("active"))],
            panes: vec![
                make_ntm_pane("sess", "0", Some("active"), None),
                make_ntm_pane("sess", "1", Some("active"), None),
                make_ntm_pane("sess", "2", Some("idle"), None),
            ],
        };
        let result = reconcile(&cache, &md, now);

        let sess = result.sessions.iter().find(|s| s.name == "sess").unwrap();
        assert_eq!(sess.pane_count, 3);
    }

    // --- Ended session detection ---

    #[test]
    fn ended_session_only_marks_ntm_source() {
        let cache = Cache::new(128);
        let now = 1_700_000_000;

        // Add a tmux-source session that is missing from markdown
        let mut tmux_sess = Session::new("tmux", "tmux-only", None, now - 60);
        tmux_sess.session_uid = "tmux-uid".to_string();
        cache.upsert_session(tmux_sess);

        let mut session_uid_by_name = HashMap::new();
        session_uid_by_name.insert("tmux-only".to_string(), "tmux-uid".to_string());

        let mut pane_uid_by_key = HashMap::new();
        let result = reconcile_ntm_markdown(
            &cache, &empty_markdown(), now, &mut session_uid_by_name, &mut pane_uid_by_key,
        );

        // tmux-sourced session should NOT be ended by NTM reconcile
        assert_eq!(result.ended_sessions, 0);
    }

    // --- Already-ended session not re-ended ---

    #[test]
    fn already_ended_session_not_double_ended() {
        let cache = Cache::new(128);
        let now = 1_700_000_000;

        let mut ended_sess = Session::new("ntm", "old", None, now - 200);
        ended_sess.session_uid = "old-uid".to_string();
        ended_sess.ended_at = Some(now - 100);
        ended_sess.status = SessionStatus::Ended;
        cache.upsert_session(ended_sess);

        let mut session_uid_by_name = HashMap::new();
        session_uid_by_name.insert("old".to_string(), "old-uid".to_string());
        let mut pane_uid_by_key = HashMap::new();

        let result = reconcile_ntm_markdown(
            &cache, &empty_markdown(), now, &mut session_uid_by_name, &mut pane_uid_by_key,
        );
        assert_eq!(result.ended_sessions, 0, "already ended should not count again");
    }

    // --- Metadata merging ---

    #[test]
    fn session_metadata_merged() {
        let cache = Cache::new(128);
        let now = 1_700_000_000;
        let mut meta = HashMap::new();
        meta.insert("version".to_string(), "1.0".to_string());
        meta.insert("env".to_string(), "prod".to_string());

        let md = NtmMarkdown {
            sessions: vec![NtmSession {
                name: "sess".to_string(),
                status: Some("active".to_string()),
                metadata: meta,
            }],
            panes: vec![],
        };
        let result = reconcile(&cache, &md, now);
        let sess = &result.sessions[0];
        let metadata = sess.metadata.as_ref().expect("should have metadata");
        assert_eq!(metadata["version"], "1.0");
        assert_eq!(metadata["env"], "prod");
    }

    // --- Pane metadata extract ---

    #[test]
    fn pane_command_extracted_from_metadata() {
        let cache = Cache::new(128);
        let now = 1_700_000_000;
        let mut meta = HashMap::new();
        meta.insert("command".to_string(), "vim".to_string());

        let md = NtmMarkdown {
            sessions: vec![make_ntm_session("sess", Some("active"))],
            panes: vec![NtmPane {
                session: "sess".to_string(),
                pane: "0".to_string(),
                status: Some("active".to_string()),
                agent: None,
                metadata: meta,
            }],
        };
        let result = reconcile(&cache, &md, now);
        assert_eq!(result.panes[0].current_command, Some("vim".to_string()));
    }

    // --- parse_pane_index ---

    #[test]
    fn parse_pane_index_valid() {
        assert_eq!(parse_pane_index("0"), 0);
        assert_eq!(parse_pane_index("3"), 3);
        assert_eq!(parse_pane_index("  7  "), 7);
    }

    #[test]
    fn parse_pane_index_invalid_returns_zero() {
        assert_eq!(parse_pane_index("abc"), 0);
        assert_eq!(parse_pane_index(""), 0);
    }

    // --- extract_metadata ---

    #[test]
    fn extract_metadata_tries_multiple_keys() {
        let mut meta = HashMap::new();
        meta.insert("cmd".to_string(), "bash".to_string());

        assert_eq!(
            extract_metadata(&meta, &["command", "cmd"]),
            Some("bash".to_string()),
        );
    }

    #[test]
    fn extract_metadata_skips_empty_values() {
        let mut meta = HashMap::new();
        meta.insert("command".to_string(), "".to_string());
        meta.insert("cmd".to_string(), "zsh".to_string());

        assert_eq!(
            extract_metadata(&meta, &["command", "cmd"]),
            Some("zsh".to_string()),
        );
    }

    #[test]
    fn extract_metadata_returns_none_when_missing() {
        let meta = HashMap::new();
        assert_eq!(extract_metadata(&meta, &["command", "cmd"]), None);
    }

    // --- Pane created for session not in sessions list ---

    #[test]
    fn pane_with_unseen_session_creates_fallback_session() {
        let cache = Cache::new(128);
        let now = 1_700_000_000;
        let md = NtmMarkdown {
            sessions: vec![], // no sessions declared
            panes: vec![make_ntm_pane("orphan-sess", "0", Some("active"), None)],
        };
        let result = reconcile(&cache, &md, now);

        // A fallback session should be created for the pane's session reference
        let sess = result.sessions.iter().find(|s| s.name == "orphan-sess");
        assert!(sess.is_some(), "fallback session should be created");
        assert_eq!(result.panes.len(), 1);
        assert_eq!(result.panes[0].session_uid, sess.unwrap().session_uid);
    }

    // --- change_count helper ---

    #[test]
    fn change_count_is_sessions_plus_panes() {
        let r = ReconcileResult {
            sessions: vec![Session::new("ntm", "a", None, 1)],
            panes: vec![Pane::new("uid".to_string(), 0, 1, None, None, None)],
            ended_sessions: 0,
        };
        assert_eq!(r.change_count(), 2);
    }

    // --- metadata_to_value ---

    #[test]
    fn metadata_to_value_converts_hashmap() {
        let mut meta = HashMap::new();
        meta.insert("key".to_string(), "value".to_string());
        let val = metadata_to_value(&meta);
        assert_eq!(val["key"], "value");
    }

    // --- Empty agent ignored ---

    #[test]
    fn empty_agent_string_not_set() {
        let cache = Cache::new(128);
        let now = 1_700_000_000;
        let md = NtmMarkdown {
            sessions: vec![make_ntm_session("sess", Some("active"))],
            panes: vec![make_ntm_pane("sess", "0", Some("active"), Some(""))],
        };
        let result = reconcile(&cache, &md, now);
        assert!(result.panes[0].agent_type.is_none(), "empty agent should be ignored");
    }
}
