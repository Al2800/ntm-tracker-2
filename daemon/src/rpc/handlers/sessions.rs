use crate::cache::Cache;
use crate::models::session::Session;
use crate::rpc::{parse_params, RpcContext, RpcError, RpcResult, CODE_NOT_FOUND};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    pub session_id: String,
    pub name: String,
    pub status: String,
    pub status_reason: Option<String>,
    pub pane_count: u32,
    pub created_at: i64,
    pub last_seen_at: i64,
    pub ended_at: Option<i64>,
    pub tmux_session_id: Option<String>,
    pub source_id: String,
    pub metadata: Option<Value>,
}

impl SessionView {
    pub fn from_session_with_pane_count(session: Session, pane_count: u32) -> Self {
        Self {
            session_id: session.session_uid,
            name: session.name,
            status: session.status.as_str().to_string(),
            status_reason: session.status_reason,
            pane_count,
            created_at: session.created_at,
            last_seen_at: session.last_seen_at,
            ended_at: session.ended_at,
            tmux_session_id: session.tmux_session_id,
            source_id: session.source_id,
            metadata: session.metadata,
        }
    }
}

impl From<Session> for SessionView {
    fn from(session: Session) -> Self {
        Self {
            session_id: session.session_uid,
            name: session.name,
            status: session.status.as_str().to_string(),
            status_reason: session.status_reason,
            pane_count: session.pane_count,
            created_at: session.created_at,
            last_seen_at: session.last_seen_at,
            ended_at: session.ended_at,
            tmux_session_id: session.tmux_session_id,
            source_id: session.source_id,
            metadata: session.metadata,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionsListParams {
    status: Option<String>,
    session_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionGetParams {
    session_id: String,
}

pub fn session_views(cache: &Cache) -> Vec<SessionView> {
    // Calculate pane counts per session from actual panes
    let mut pane_counts: HashMap<String, u32> = HashMap::new();
    for pane in cache.all_panes() {
        *pane_counts.entry(pane.session_uid).or_insert(0) += 1;
    }

    cache
        .all_sessions()
        .into_iter()
        .map(|session| {
            let count = pane_counts.get(&session.session_uid).copied().unwrap_or(0);
            SessionView::from_session_with_pane_count(session, count)
        })
        .collect()
}

pub fn list(ctx: &RpcContext, params: Value) -> RpcResult<Value> {
    let params: SessionsListParams = if params.is_null() {
        SessionsListParams {
            status: None,
            session_ids: None,
        }
    } else {
        parse_params(params)?
    };
    let mut sessions = session_views(ctx.cache.as_ref());

    if let Some(ref allowed) = params.session_ids {
        sessions.retain(|session| allowed.contains(&session.session_id));
    }
    if let Some(ref status) = params.status {
        sessions.retain(|session| session.status == *status);
    }

    Ok(json!({ "sessions": sessions }))
}

pub fn get(ctx: &RpcContext, params: Value) -> RpcResult<Value> {
    let params: SessionGetParams = parse_params(params)?;
    let session = ctx
        .cache
        .get_session(&params.session_id)
        .ok_or_else(|| RpcError::new(CODE_NOT_FOUND, "Session not found"))?;
    Ok(json!({ "session": SessionView::from(session) }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::Cache;
    use crate::config::ConfigManager;
    use crate::models::pane::{Pane, PaneStatus};
    use crate::models::session::{Session, SessionStatus};
    use crate::rpc::{Capabilities, RpcContext};
    use std::sync::Arc;

    fn test_ctx() -> RpcContext {
        let cache = Arc::new(Cache::new(100));
        let config = ConfigManager::default();
        let caps = Capabilities { ntm: false, tmux: false, stream: false, systemd: false };
        RpcContext::with_capabilities(cache, config, caps)
    }

    fn make_session(uid: &str, name: &str, status: SessionStatus) -> Session {
        Session {
            session_uid: uid.to_string(),
            source_id: "tmux".to_string(),
            tmux_session_id: None,
            name: name.to_string(),
            created_at: 1000,
            last_seen_at: 2000,
            ended_at: None,
            status,
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
            tmux_pane_id: None, tmux_window_id: None, tmux_pane_pid: None,
            agent_type: None, created_at: 1, last_seen_at: 1,
            last_activity_at: None, current_command: None, ended_at: None,
            status: PaneStatus::Active, status_reason: None,
        }
    }

    #[test]
    fn sessions_list_empty() {
        let ctx = test_ctx();
        let result = list(&ctx, Value::Null).unwrap();
        assert!(result["sessions"].as_array().unwrap().is_empty());
    }

    #[test]
    fn sessions_list_returns_all() {
        let ctx = test_ctx();
        ctx.cache.upsert_session(make_session("s1", "alpha", SessionStatus::Active));
        ctx.cache.upsert_session(make_session("s2", "beta", SessionStatus::Idle));
        let result = list(&ctx, Value::Null).unwrap();
        assert_eq!(result["sessions"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn sessions_list_filter_by_status() {
        let ctx = test_ctx();
        ctx.cache.upsert_session(make_session("s1", "alpha", SessionStatus::Active));
        ctx.cache.upsert_session(make_session("s2", "beta", SessionStatus::Idle));
        let result = list(&ctx, json!({"status": "active"})).unwrap();
        let sessions = result["sessions"].as_array().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0]["status"], "active");
    }

    #[test]
    fn sessions_list_filter_by_ids() {
        let ctx = test_ctx();
        ctx.cache.upsert_session(make_session("s1", "alpha", SessionStatus::Active));
        ctx.cache.upsert_session(make_session("s2", "beta", SessionStatus::Active));
        ctx.cache.upsert_session(make_session("s3", "gamma", SessionStatus::Active));
        let result = list(&ctx, json!({"sessionIds": ["s1", "s3"]})).unwrap();
        let sessions = result["sessions"].as_array().unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn sessions_get_found() {
        let ctx = test_ctx();
        ctx.cache.upsert_session(make_session("s1", "alpha", SessionStatus::Active));
        let result = get(&ctx, json!({"sessionId": "s1"})).unwrap();
        assert_eq!(result["session"]["sessionId"], "s1");
        assert_eq!(result["session"]["name"], "alpha");
        assert_eq!(result["session"]["status"], "active");
    }

    #[test]
    fn sessions_get_not_found() {
        let ctx = test_ctx();
        let result = get(&ctx, json!({"sessionId": "nonexistent"}));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, CODE_NOT_FOUND);
    }

    #[test]
    fn session_views_counts_panes() {
        let ctx = test_ctx();
        ctx.cache.upsert_session(make_session("s1", "alpha", SessionStatus::Active));
        ctx.cache.upsert_pane(make_pane("p1", "s1"));
        ctx.cache.upsert_pane(make_pane("p2", "s1"));
        let views = session_views(ctx.cache.as_ref());
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].pane_count, 2);
    }
}
