use crate::cache::Cache;
use crate::models::pane::Pane;
use crate::command::{CommandCategory, CommandConfig, CommandRunner, CommandSpec, CommandError};
use crate::redaction::default_redactor;
use crate::rpc::{parse_params, RpcContext, RpcError, RpcResult, CODE_DEGRADED, CODE_INVALID_PARAMS, CODE_NOT_FOUND};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Validates that a pane_id is safe for use with tmux commands.
/// Valid tmux pane targets: %<digits>, @<digits>:<digits>, session:window.pane
/// Only allows alphanumeric, %, @, :, ., -, _
fn is_valid_pane_id(pane_id: &str) -> bool {
    if pane_id.is_empty() || pane_id.len() > 64 {
        return false;
    }
    pane_id
        .chars()
        .all(|c| c.is_alphanumeric() || matches!(c, '%' | '@' | ':' | '.' | '-' | '_'))
}

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

    // Validate pane_id to prevent command injection
    if !is_valid_pane_id(&params.pane_id) {
        return Err(RpcError::new(CODE_INVALID_PARAMS, "Invalid pane_id format"));
    }

    let max_lines = params.max_lines.unwrap_or(200).max(1);
    let max_chars = params.max_chars.unwrap_or(64 * 1024).max(1);
    let command = CommandSpec {
        program: "tmux".to_string(),
        args: vec![
            "capture-pane".to_string(),
            "-p".to_string(),
            "-t".to_string(),
            params.pane_id.clone(),
            "-S".to_string(),
            format!("-{}", max_lines),
        ],
        timeout: Duration::from_secs(2),
        max_output_bytes: max_chars,
        category: CommandCategory::TmuxFast,
    };

    let runner = CommandRunner::new(CommandConfig::default());
    let output_result = if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.block_on(runner.run(command))
    } else {
        let runtime =
            tokio::runtime::Runtime::new().map_err(|err| RpcError::new(CODE_DEGRADED, err.to_string()))?;
        runtime.block_on(runner.run(command))
    };

    let output = output_result.map_err(|err| {
        let message = match err {
            CommandError::Timeout => "tmux capture-pane timed out".to_string(),
            CommandError::CircuitOpen => "tmux command circuit is open".to_string(),
            other => format!("tmux capture-pane failed: {other:?}"),
        };
        RpcError::new(CODE_DEGRADED, message)
    })?;

    let raw = String::from_utf8_lossy(&output.stdout);
    let redacted = default_redactor().redact(&raw);
    let truncated = redacted.len() > max_chars;
    let content = if truncated {
        redacted.chars().take(max_chars).collect::<String>()
    } else {
        redacted
    };

    let captured_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);

    let line_count = content.lines().count();

    Ok(json!({
        "paneId": params.pane_id,
        "content": content,
        "lines": line_count,
        "bytes": output.stdout.len(),
        "truncated": truncated,
        "capturedAt": captured_at,
        "redacted": true
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_pane_ids() {
        assert!(is_valid_pane_id("%0"));
        assert!(is_valid_pane_id("%123"));
        assert!(is_valid_pane_id("@1:0"));
        assert!(is_valid_pane_id("session:window.0"));
        assert!(is_valid_pane_id("my-session:0.1"));
        assert!(is_valid_pane_id("my_session:0"));
    }

    #[test]
    fn invalid_pane_ids() {
        assert!(!is_valid_pane_id(""));
        assert!(!is_valid_pane_id("; rm -rf /"));
        assert!(!is_valid_pane_id("$(whoami)"));
        assert!(!is_valid_pane_id("`id`"));
        assert!(!is_valid_pane_id("foo\nbar"));
        assert!(!is_valid_pane_id("foo bar"));
        assert!(!is_valid_pane_id(&"a".repeat(100)));
    }
}
