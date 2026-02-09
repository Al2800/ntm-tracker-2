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
