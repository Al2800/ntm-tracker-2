use crate::command::{CommandCategory, CommandConfig, CommandError, CommandOutput, CommandRunner, CommandSpec};
use crate::models::session::SessionStatus;
use crate::rpc::{
    parse_params, RpcContext, RpcError, RpcResult, CODE_FORBIDDEN, CODE_INVALID_PARAMS,
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
struct PaneSendParams {
    pane_id: String,
    payload: String,
    /// If true, send Enter after the payload text.
    #[serde(default)]
    enter: bool,
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

/// Run a tmux command spec and map errors to RpcError.
fn run_tmux(spec: CommandSpec) -> RpcResult<CommandOutput> {
    let runner = CommandRunner::new(CommandConfig::default());
    let result = if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.block_on(runner.run(spec))
    } else {
        let runtime = tokio::runtime::Runtime::new().map_err(|err| {
            RpcError::new(CODE_UNSUPPORTED, format!("Runtime init failed: {err}"))
        })?;
        runtime.block_on(runner.run(spec))
    };

    result.map_err(|err| match err {
        CommandError::CircuitOpen => {
            RpcError::new(CODE_UNSUPPORTED, "tmux command circuit is open")
        }
        CommandError::Timeout => {
            RpcError::new(CODE_UNSUPPORTED, "tmux command timed out")
        }
        CommandError::ExitNonZero(code) => {
            RpcError::new(CODE_UNSUPPORTED, format!("tmux exited with code {code}"))
        }
        other => {
            RpcError::new(CODE_UNSUPPORTED, format!("tmux error: {other:?}"))
        }
    })
}

fn tmux_spec(args: Vec<String>) -> CommandSpec {
    CommandSpec {
        program: "tmux".to_string(),
        args,
        timeout: Duration::from_secs(2),
        max_output_bytes: 64 * 1024,
        category: CommandCategory::TmuxFast,
    }
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

    let spec = tmux_spec(vec![
        "kill-session".to_string(),
        "-t".to_string(),
        target.to_string(),
    ]);

    run_tmux(spec)?;

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

/// Send literal text (and optionally Enter) to a tmux pane via `tmux send-keys`.
///
/// Uses `-l` flag to send text literally (no key-name interpretation).
/// When `enter` is true, a separate `send-keys Enter` is sent after the text.
pub fn pane_send(ctx: &RpcContext, params: Value) -> RpcResult<Value> {
    require_admin_or_unsecured(ctx)?;
    let params: PaneSendParams = parse_params(params)?;

    if !is_valid_pane_id(&params.pane_id) {
        return Err(RpcError::new(
            CODE_INVALID_PARAMS,
            "Invalid pane_id format",
        ));
    }

    if params.payload.len() > 4096 {
        return Err(RpcError::new(
            CODE_INVALID_PARAMS,
            "Payload too large (max 4096 bytes)",
        ));
    }

    // Send literal text: tmux send-keys -t <pane> -l -- <payload>
    let spec = tmux_spec(vec![
        "send-keys".to_string(),
        "-t".to_string(),
        params.pane_id.clone(),
        "-l".to_string(),
        "--".to_string(),
        params.payload.clone(),
    ]);
    run_tmux(spec)?;

    // Optionally send Enter after the literal text
    if params.enter {
        let enter_spec = tmux_spec(vec![
            "send-keys".to_string(),
            "-t".to_string(),
            params.pane_id.clone(),
            "Enter".to_string(),
        ]);
        run_tmux(enter_spec)?;
    }

    Ok(json!({
        "sent": true,
        "paneId": params.pane_id,
        "bytes": params.payload.len(),
        "enter": params.enter,
    }))
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
    use crate::cache::Cache;
    use crate::config::ConfigManager;
    use std::sync::Arc;

    fn test_ctx() -> crate::rpc::RpcContext {
        let cache = Arc::new(Cache::new(100));
        let config = ConfigManager::default();
        crate::rpc::RpcContext::new(cache, config)
    }

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

    #[test]
    fn valid_session_targets() {
        assert!(is_valid_session_target("my-session"));
        assert!(is_valid_session_target("$0"));
        assert!(is_valid_session_target("project_a"));
    }

    #[test]
    fn invalid_session_targets() {
        assert!(!is_valid_session_target(""));
        assert!(!is_valid_session_target("; rm -rf /"));
        assert!(!is_valid_session_target("$(whoami)"));
        assert!(!is_valid_session_target(&"a".repeat(200)));
    }

    #[test]
    fn pane_send_params_deserialize_defaults() {
        let json = json!({"paneId": "%0", "payload": "hello"});
        let params: PaneSendParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.pane_id, "%0");
        assert_eq!(params.payload, "hello");
        assert!(!params.enter);
    }

    #[test]
    fn pane_send_params_deserialize_with_enter() {
        let json = json!({"paneId": "%0", "payload": "ls -la", "enter": true});
        let params: PaneSendParams = serde_json::from_value(json).unwrap();
        assert!(params.enter);
    }

    #[test]
    fn tmux_spec_creates_correct_command() {
        let spec = tmux_spec(vec!["send-keys".into(), "-t".into(), "%0".into()]);
        assert_eq!(spec.program, "tmux");
        assert_eq!(spec.args, vec!["send-keys", "-t", "%0"]);
        assert_eq!(spec.timeout, Duration::from_secs(2));
    }

    #[test]
    fn attach_command_valid_pane() {
        let json = json!({"paneId": "%0"});
        let result = attach_command(
            &test_ctx(),
            json,
        );
        let val = result.unwrap();
        assert_eq!(val["command"], "tmux attach -t %0");
    }

    #[test]
    fn attach_command_invalid_pane() {
        let json = json!({"paneId": "; rm -rf /"});
        let result = attach_command(
            &test_ctx(),
            json,
        );
        assert!(result.is_err());
    }
}
