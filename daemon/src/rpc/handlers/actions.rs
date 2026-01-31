use crate::command::{CommandCategory, CommandConfig, CommandError, CommandRunner, CommandSpec};
use crate::models::session::SessionStatus;
use crate::rpc::{
    parse_params, require_admin, RpcContext, RpcError, RpcResult, CODE_FORBIDDEN, CODE_INVALID_PARAMS,
    CODE_NOT_FOUND, CODE_UNSUPPORTED,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionKillParams {
    session_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct PaneSendParams {
    pane_id: String,
    payload: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AttachCommandParams {
    pane_id: String,
}

fn require_admin_or_unsecured(ctx: &RpcContext) -> RpcResult<()> {
    if ctx.is_admin || ctx.config.current().security.admin_token_path.is_none() {
        Ok(())
    } else {
        Err(RpcError::new(
            CODE_FORBIDDEN,
            "Admin token required for this method",
        ))
    }
}

/// Validates that a session target is safe for tmux commands.
/// Allows alphanumeric, %, @, :, ., -, _, $
fn is_valid_session_target(target: &str) -> bool {
    if target.is_empty() || target.len() > 128 {
        return false;
    }
    target
        .chars()
        .all(|c| c.is_alphanumeric() || matches!(c, '%' | '@' | ':' | '.' | '-' | '_' | '$'))
}

/// Validates that a pane_id is safe for shell use.
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

pub fn session_kill(ctx: &RpcContext, params: Value) -> RpcResult<Value> {
    require_admin_or_unsecured(ctx)?;
    let params: SessionKillParams = parse_params(params)?;
    let session = ctx
        .cache
        .get_session(&params.session_id)
        .ok_or_else(|| RpcError::new(CODE_NOT_FOUND, "Session not found"))?;

    let target = session
        .tmux_session_id
        .as_deref()
        .unwrap_or(session.name.as_str());

    if !is_valid_session_target(target) {
        return Err(RpcError::new(
            CODE_INVALID_PARAMS,
            "Invalid tmux session target",
        ));
    }

    let spec = CommandSpec {
        program: "tmux".to_string(),
        args: vec![
            "kill-session".to_string(),
            "-t".to_string(),
            target.to_string(),
        ],
        timeout: Duration::from_secs(2),
        max_output_bytes: 64 * 1024,
        category: CommandCategory::TmuxFast,
    };

    let runner = CommandRunner::new(CommandConfig::default());
    let output_result = if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.block_on(runner.run(spec))
    } else {
        let runtime = tokio::runtime::Runtime::new().map_err(|err| {
            RpcError::new(CODE_UNSUPPORTED, format!("Runtime init failed: {err}"))
        })?;
        runtime.block_on(runner.run(spec))
    };

    match output_result {
        Ok(_) => {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_else(|_| Duration::from_secs(0))
                .as_secs() as i64;
            let mut updated = session;
            updated.ended_at = Some(now);
            updated.status = SessionStatus::Ended;
            updated.status_reason = Some("killed".to_string());
            ctx.cache.upsert_session(updated);
            Ok(json!({ "killed": true, "sessionId": params.session_id }))
        }
        Err(CommandError::CircuitOpen) => Err(RpcError::new(
            CODE_UNSUPPORTED,
            "tmux command circuit is open",
        )),
        Err(CommandError::Timeout) => Err(RpcError::new(
            CODE_UNSUPPORTED,
            "tmux kill-session timed out",
        )),
        Err(CommandError::ExitNonZero(_)) => Err(RpcError::new(
            CODE_UNSUPPORTED,
            "tmux kill-session failed",
        )),
        Err(err) => Err(RpcError::new(
            CODE_UNSUPPORTED,
            format!("tmux kill-session error: {err:?}"),
        )),
    }
}

pub fn pane_send(ctx: &RpcContext, params: Value) -> RpcResult<Value> {
    require_admin(ctx)?;
    let params: PaneSendParams = parse_params(params)?;
    Err(RpcError::new(
        CODE_UNSUPPORTED,
        format!("paneSend not implemented for {}", params.pane_id),
    ))
}

pub fn attach_command(_ctx: &RpcContext, params: Value) -> RpcResult<Value> {
    let params: AttachCommandParams = parse_params(params)?;

    // Validate pane_id to prevent command injection
    if !is_valid_pane_id(&params.pane_id) {
        return Err(RpcError::new(
            CODE_INVALID_PARAMS,
            "Invalid pane_id format",
        ));
    }

    Ok(json!({
        "command": format!("tmux attach -t {}", params.pane_id)
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
