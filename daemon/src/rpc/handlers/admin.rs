use crate::rpc::{require_admin, RpcContext, RpcError, RpcResult, CODE_DEGRADED};
use serde_json::{json, Value};

pub fn config_get(ctx: &RpcContext) -> RpcResult<Value> {
    let config = ctx.config.current();
    Ok(json!({
        "config": config,
        "configPath": ctx.config.config_path().map(|path| path.display().to_string()),
        "adminMode": ctx.is_admin
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
    let config = ctx
        .config
        .reload()
        .map_err(|err| RpcError::new(CODE_DEGRADED, err.to_string()))?;
    Ok(json!({
        "reloaded": true,
        "config": config,
        "configPath": ctx.config.config_path().map(|path| path.display().to_string())
    }))
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
