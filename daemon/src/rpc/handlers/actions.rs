use crate::rpc::{parse_params, require_admin, RpcContext, RpcError, RpcResult, CODE_INVALID_PARAMS, CODE_UNSUPPORTED};
use serde::Deserialize;
use serde_json::{json, Value};

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
    require_admin(ctx)?;
    let params: SessionKillParams = parse_params(params)?;
    Err(RpcError::new(
        CODE_UNSUPPORTED,
        format!("sessionKill not implemented for {}", params.session_id),
    ))
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
