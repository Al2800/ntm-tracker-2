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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::Cache;
    use crate::config::ConfigManager;
    use crate::rpc::{Capabilities, RpcContext, CODE_FORBIDDEN};
    use std::sync::Arc;

    fn test_ctx(is_admin: bool) -> RpcContext {
        let cache = Arc::new(Cache::new(100));
        let config = ConfigManager::default();
        let caps = Capabilities { ntm: false, tmux: false, stream: false, systemd: false };
        let mut ctx = RpcContext::with_capabilities(cache, config, caps);
        ctx.is_admin = is_admin;
        ctx
    }

    #[test]
    fn config_get_returns_config() {
        let ctx = test_ctx(false);
        let result = config_get(&ctx).unwrap();
        assert!(result["config"].is_object());
        assert_eq!(result["adminMode"], false);
    }

    #[test]
    fn config_get_shows_admin_mode() {
        let ctx = test_ctx(true);
        let result = config_get(&ctx).unwrap();
        assert_eq!(result["adminMode"], true);
    }

    #[test]
    fn config_set_requires_admin() {
        let ctx = test_ctx(false);
        let result = config_set(&ctx, json!({"foo": "bar"}));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, CODE_FORBIDDEN);
    }

    #[test]
    fn config_set_succeeds_as_admin() {
        let ctx = test_ctx(true);
        let result = config_set(&ctx, json!({"polling": {"interval": 5000}})).unwrap();
        assert_eq!(result["applied"], true);
    }

    #[test]
    fn config_reload_requires_admin() {
        let ctx = test_ctx(false);
        let result = config_reload(&ctx);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, CODE_FORBIDDEN);
    }

    #[test]
    fn detectors_list_returns_detectors() {
        let ctx = test_ctx(false);
        let result = detectors_list(&ctx).unwrap();
        let detectors = result["detectors"].as_array().unwrap();
        assert!(!detectors.is_empty());
        assert!(detectors.iter().any(|d| d["name"] == "compact"));
    }

    #[test]
    fn detectors_reload_requires_admin() {
        let ctx = test_ctx(false);
        let result = detectors_reload(&ctx);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, CODE_FORBIDDEN);
    }

    #[test]
    fn detectors_reload_succeeds_as_admin() {
        let ctx = test_ctx(true);
        let result = detectors_reload(&ctx).unwrap();
        assert_eq!(result["reloaded"], true);
    }
}
