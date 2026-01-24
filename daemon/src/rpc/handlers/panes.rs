use crate::cache::Cache;
use crate::models::pane::Pane;
use crate::redaction::default_redactor;
use crate::rpc::{parse_params, RpcContext, RpcError, RpcResult, CODE_NOT_FOUND};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaneView {
    pub pane_id: String,
    pub session_id: String,
    pub status: String,
    pub status_reason: Option<String>,
    pub pane_index: i32,
    pub agent_type: Option<String>,
    pub created_at: i64,
    pub last_seen_at: i64,
    pub last_activity_at: Option<i64>,
    pub current_command: Option<String>,
    pub ended_at: Option<i64>,
    pub tmux_pane_id: Option<String>,
    pub tmux_window_id: Option<String>,
    pub tmux_pane_pid: Option<i64>,
}

impl From<Pane> for PaneView {
    fn from(pane: Pane) -> Self {
        Self {
            pane_id: pane.pane_uid,
            session_id: pane.session_uid,
            status: pane.status.as_str().to_string(),
            status_reason: pane.status_reason,
            pane_index: pane.pane_index,
            agent_type: pane.agent_type,
            created_at: pane.created_at,
            last_seen_at: pane.last_seen_at,
            last_activity_at: pane.last_activity_at,
            current_command: pane.current_command,
            ended_at: pane.ended_at,
            tmux_pane_id: pane.tmux_pane_id,
            tmux_window_id: pane.tmux_window_id,
            tmux_pane_pid: pane.tmux_pane_pid,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PaneGetParams {
    pane_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PanePreviewParams {
    pane_id: String,
    max_lines: Option<usize>,
    max_chars: Option<usize>,
}

pub fn pane_views(cache: &Cache) -> Vec<PaneView> {
    cache
        .all_panes()
        .into_iter()
        .map(PaneView::from)
        .collect()
}

pub fn get(ctx: &RpcContext, params: Value) -> RpcResult<Value> {
    let params: PaneGetParams = parse_params(params)?;
    let pane = ctx
        .cache
        .get_pane(&params.pane_id)
        .ok_or_else(|| RpcError::new(CODE_NOT_FOUND, "Pane not found"))?;
    Ok(json!({ "pane": PaneView::from(pane) }))
}

pub fn output_preview(_ctx: &RpcContext, params: Value) -> RpcResult<Value> {
    let params: PanePreviewParams = parse_params(params)?;
    let preview = default_redactor().redact("");
    Ok(json!({
        "paneId": params.pane_id,
        "preview": preview,
        "redacted": true,
        "maxLines": params.max_lines.unwrap_or(0),
        "maxChars": params.max_chars.unwrap_or(0)
    }))
}
