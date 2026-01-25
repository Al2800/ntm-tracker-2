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
