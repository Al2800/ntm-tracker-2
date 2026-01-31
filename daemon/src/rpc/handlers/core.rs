use crate::rpc::handlers::{events, panes, sessions, stats};
use crate::rpc::{RpcContext, RpcResult};
use serde_json::{json, Value};

pub fn hello(ctx: &RpcContext) -> RpcResult<Value> {
    Ok(crate::rpc::hello_payload(ctx))
}

pub fn health_get(ctx: &RpcContext) -> RpcResult<Value> {
    let health = ctx.cache.health();
    let last_event_id = events::last_event_id(ctx.cache.as_ref());

    Ok(json!({
        "status": health.status,
        "uptime": ctx.uptime_secs(),
        "version": crate::version(),
        "instanceId": ctx.instance_id,
        "runId": ctx.run_id,
        "schemaVersion": ctx.schema_version,
        "protocolVersion": ctx.protocol_version,
        "capabilities": ctx.capabilities,
        "lastEventId": last_event_id,
        "lastError": health.last_error,
    }))
}

pub fn capabilities_get(ctx: &RpcContext) -> RpcResult<Value> {
    Ok(json!({
        "protocolVersion": ctx.protocol_version,
        "schemaVersion": ctx.schema_version,
        "capabilities": ctx.capabilities,
    }))
}

pub fn snapshot_get(ctx: &RpcContext) -> RpcResult<Value> {
    let sessions = sessions::session_views(ctx.cache.as_ref());
    let panes = panes::pane_views(ctx.cache.as_ref());
    let events = events::event_views(ctx.cache.as_ref(), None, None);
    let stats_summary = stats::summary_payload(ctx.cache.as_ref());
    let last_event_id = events::last_event_id(ctx.cache.as_ref());

    Ok(json!({
        "sessions": sessions,
        "panes": panes,
        "events": events,
        "stats": {
            "summary": stats_summary,
            "hourly": [],
            "daily": [],
        },
        "lastEventId": last_event_id,
    }))
}
