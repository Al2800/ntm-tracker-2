use crate::cache::Cache;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

#[derive(Clone)]
pub struct RpcContext {
    pub cache: Arc<Cache>,
    pub instance_id: String,
    pub run_id: String,
    pub started_at: Instant,
    pub protocol_version: u32,
    pub schema_version: u32,
    pub capabilities: Capabilities,
    pub is_admin: bool,
}

impl RpcContext {
    pub fn new(cache: Arc<Cache>) -> Self {
        Self {
            cache,
            instance_id: Uuid::now_v7().to_string(),
            run_id: Uuid::now_v7().to_string(),
            started_at: Instant::now(),
            protocol_version: 1,
            schema_version: 1,
            capabilities: Capabilities {
                ntm: true,
                tmux: true,
                stream: false,
                systemd: false,
            },
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
        _ => Err(RpcError::new(
            CODE_UNSUPPORTED,
            format!("Unsupported method: {method}"),
        )),
    }
}
