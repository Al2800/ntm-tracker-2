use crate::rpc::{require_admin, RpcContext, RpcResult};
use serde_json::{json, Value};

pub fn config_get(ctx: &RpcContext) -> RpcResult<Value> {
    Ok(json!({
        "config": {
            "transport": "stdio",
            "idleThresholdSecs": 300,
            "captureOutput": false,
            "adminMode": ctx.is_admin
        }
    }))
}

pub fn config_set(ctx: &RpcContext, params: Value) -> RpcResult<Value> {
    require_admin(ctx)?;
    Ok(json!({
        "applied": true,
        "config": params
    }))
}

pub fn config_reload(ctx: &RpcContext) -> RpcResult<Value> {
    require_admin(ctx)?;
    Ok(json!({ "reloaded": true }))
}

pub fn detectors_list(_ctx: &RpcContext) -> RpcResult<Value> {
    Ok(json!({
        "detectors": [
            { "name": "compact", "version": "1.0.0", "enabled": true },
            { "name": "escalation", "version": "1.0.0", "enabled": true },
            { "name": "status", "version": "1.0.0", "enabled": true }
        ]
    }))
}

pub fn detectors_reload(ctx: &RpcContext) -> RpcResult<Value> {
    require_admin(ctx)?;
    Ok(json!({ "reloaded": true }))
}
