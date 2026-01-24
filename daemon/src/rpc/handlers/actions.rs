use crate::rpc::{parse_params, require_admin, RpcContext, RpcError, RpcResult, CODE_UNSUPPORTED};
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
    Ok(json!({
        "command": format!("tmux attach -t {}", params.pane_id)
    }))
}
