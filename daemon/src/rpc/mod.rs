use crate::cache::Cache;
use crate::config::ConfigManager;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub mod handlers;

pub const CODE_UNAUTHORIZED: &str = "UNAUTHORIZED";
pub const CODE_FORBIDDEN: &str = "FORBIDDEN";
pub const CODE_RATE_LIMITED: &str = "RATE_LIMITED";
pub const CODE_STALE_CURSOR: &str = "STALE_CURSOR";
pub const CODE_UNSUPPORTED: &str = "UNSUPPORTED";
pub const CODE_DEGRADED: &str = "DEGRADED";
pub const CODE_NOT_FOUND: &str = "NOT_FOUND";
pub const CODE_INVALID_PARAMS: &str = "INVALID_PARAMS";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub ntm: bool,
    pub tmux: bool,
    pub stream: bool,
    pub systemd: bool,
}

impl Capabilities {
    /// Probe the system to determine actual capabilities.
    pub fn probe() -> Self {
        Self {
            ntm: probe_ntm_available(),
            tmux: probe_tmux_available(),
            stream: false,
            systemd: probe_systemd_available(),
        }
    }
}

/// Check if NTM is available by testing if the binary can be found.
fn probe_ntm_available() -> bool {
    std::process::Command::new("which")
        .arg("ntm")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Check if tmux is available.
fn probe_tmux_available() -> bool {
    std::process::Command::new("which")
        .arg("tmux")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Check if systemd is available.
fn probe_systemd_available() -> bool {
    std::path::Path::new("/run/systemd/system").exists()
}

#[derive(Clone)]
pub struct RpcContext {
    pub cache: Arc<Cache>,
    pub config: ConfigManager,
    pub instance_id: String,
    pub run_id: String,
    pub started_at: Instant,
    pub protocol_version: u32,
    pub schema_version: u32,
    pub capabilities: Capabilities,
    pub is_admin: bool,
}

impl RpcContext {
    /// Create a new RpcContext with probed capabilities.
    pub fn new(cache: Arc<Cache>, config: ConfigManager) -> Self {
        Self::with_capabilities(cache, config, Capabilities::probe())
    }

    /// Create a new RpcContext with explicit capabilities (for testing).
    pub fn with_capabilities(cache: Arc<Cache>, config: ConfigManager, capabilities: Capabilities) -> Self {
        Self {
            cache,
            config,
            instance_id: Uuid::now_v7().to_string(),
            run_id: Uuid::now_v7().to_string(),
            started_at: Instant::now(),
            protocol_version: 1,
            schema_version: 1,
            capabilities,
            is_admin: false,
        }
    }

    pub fn uptime_secs(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }

    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcError {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl RpcError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(code: &'static str, message: impl Into<String>, data: Value) -> Self {
        Self {
            code,
            message: message.into(),
            data: Some(data),
        }
    }
}

pub type RpcResult<T> = Result<T, RpcError>;

pub fn hello_payload(ctx: &RpcContext) -> Value {
    json!({
        "daemonVersion": crate::version(),
        "protocolVersion": ctx.protocol_version,
        "schemaVersion": ctx.schema_version,
        "capabilities": ctx.capabilities,
        "instanceId": ctx.instance_id,
        "runId": ctx.run_id,
    })
}

pub fn require_admin(ctx: &RpcContext) -> RpcResult<()> {
    if ctx.is_admin {
        Ok(())
    } else {
        Err(RpcError::new(
            CODE_FORBIDDEN,
            "Admin token required for this method",
        ))
    }
}

pub fn parse_params<T: for<'de> Deserialize<'de>>(params: Value) -> RpcResult<T> {
    serde_json::from_value(params).map_err(|err| {
        RpcError::with_data(
            CODE_INVALID_PARAMS,
            "Invalid params",
            Value::String(err.to_string()),
        )
    })
}

pub fn handle(method: &str, params: Value, ctx: &RpcContext) -> RpcResult<Value> {
    match method {
        "core.hello" => handlers::core::hello(ctx),
        "health.get" => handlers::core::health_get(ctx),
        "capabilities.get" => handlers::core::capabilities_get(ctx),
        "snapshot.get" => handlers::core::snapshot_get(ctx),
        "sessions.list" => handlers::sessions::list(ctx, params),
        "sessions.get" => handlers::sessions::get(ctx, params),
        "panes.get" => handlers::panes::get(ctx, params),
        "panes.outputPreview" => handlers::panes::output_preview(ctx, params),
        "events.list" => handlers::events::list(ctx, params),
        "subscribe" => handlers::events::subscribe(ctx, params),
        "escalations.list" => handlers::events::escalations_list(ctx),
        "escalations.dismiss" => handlers::events::escalations_dismiss(ctx, params),
        "stats.summary" => handlers::stats::summary(ctx),
        "stats.hourly" => handlers::stats::hourly(ctx, params),
        "stats.daily" => handlers::stats::daily(ctx, params),
        "config.get" => handlers::admin::config_get(ctx),
        "config.set" => handlers::admin::config_set(ctx, params),
        "config.reload" => handlers::admin::config_reload(ctx),
        "detectors.list" => handlers::admin::detectors_list(ctx),
        "detectors.reload" => handlers::admin::detectors_reload(ctx),
        "actions.sessionKill" => handlers::actions::session_kill(ctx, params),
        "actions.paneSend" => handlers::actions::pane_send(ctx, params),
        "attach.command" => handlers::actions::attach_command(ctx, params),
        "debug.diagnostics" => handlers::debug::diagnostics(ctx),
        "debug.selfTest" => handlers::debug::self_test(ctx),
        "debug.metrics" => handlers::debug::metrics(ctx),
        "debug.logTail" => handlers::debug::log_tail(ctx),
        _ => Err(RpcError::new(
            CODE_UNSUPPORTED,
            format!("Unsupported method: {method}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_capabilities() -> Capabilities {
        Capabilities {
            ntm: false,
            tmux: true,
            stream: false,
            systemd: false,
        }
    }

    #[test]
    fn capabilities_probe_runs_without_panic() {
        // Just verify probing doesn't crash
        let _ = Capabilities::probe();
    }

    #[test]
    fn capabilities_can_be_constructed_manually() {
        let caps = Capabilities {
            ntm: false,
            tmux: true,
            stream: true,
            systemd: false,
        };
        assert!(!caps.ntm);
        assert!(caps.tmux);
        assert!(caps.stream);
        assert!(!caps.systemd);
    }

    #[test]
    fn context_with_custom_capabilities() {
        let cache = Arc::new(Cache::new(100));
        let config = ConfigManager::default();
        let caps = test_capabilities();

        let ctx = RpcContext::with_capabilities(cache, config, caps);
        assert!(!ctx.capabilities.ntm);
        assert!(ctx.capabilities.tmux);
    }

    #[test]
    fn capabilities_reflected_in_health_get() {
        let cache = Arc::new(Cache::new(100));
        cache.set_health(crate::cache::HealthStatus {
            status: "ok".to_string(),
            last_error: None,
        });
        let config = ConfigManager::default();
        let caps = Capabilities {
            ntm: false,
            tmux: true,
            stream: false,
            systemd: false,
        };

        let ctx = RpcContext::with_capabilities(cache, config, caps);
        let result = handle("health.get", serde_json::json!(null), &ctx).unwrap();

        // Verify ntm capability is false
        assert_eq!(result["capabilities"]["ntm"], false);
        assert_eq!(result["capabilities"]["tmux"], true);
    }

    #[test]
    fn capabilities_reflected_in_capabilities_get() {
        let cache = Arc::new(Cache::new(100));
        let config = ConfigManager::default();
        let caps = Capabilities {
            ntm: false,
            tmux: true,
            stream: false,
            systemd: true,
        };

        let ctx = RpcContext::with_capabilities(cache, config, caps);
        let result = handle("capabilities.get", serde_json::json!(null), &ctx).unwrap();

        assert_eq!(result["capabilities"]["ntm"], false);
        assert_eq!(result["capabilities"]["tmux"], true);
        assert_eq!(result["capabilities"]["systemd"], true);
    }

    #[test]
    fn systemd_probe_checks_path() {
        // Just verify it runs
        let _ = probe_systemd_available();
    }

    #[test]
    fn ntm_probe_handles_missing_binary() {
        // Probing for a non-existent binary should return false
        // (which is the case when ntm is not installed)
        let _ = probe_ntm_available(); // Should not panic
    }

    #[test]
    fn tmux_probe_handles_missing_binary() {
        // Similar to ntm, should not panic
        let _ = probe_tmux_available();
    }
}
