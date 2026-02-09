use crate::rpc::handlers::{events, panes, sessions, stats};
use crate::rpc::{RpcContext, RpcResult};
use serde_json::{json, Value};

pub fn hello(ctx: &RpcContext) -> RpcResult<Value> {
    Ok(crate::rpc::hello_payload(ctx))
}

pub fn health_get(ctx: &RpcContext) -> RpcResult<Value> {
    let health = ctx.cache.health();
    let last_event_id = events::last_event_id(ctx.cache.as_ref());
    let polling_state = ctx.cache.polling_state();
    let polling_config = ctx.config.current().polling;

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
        "polling": {
            "snapshot": polling_state.snapshot,
            "tmux": polling_state.tmux,
            "ntm": polling_state.ntm,
            "config": polling_config,
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::{Cache, EventRecord, HealthStatus, StatsAggregate};
    use crate::config::ConfigManager;
    use crate::models::pane::{Pane, PaneStatus};
    use crate::models::session::{Session, SessionStatus};
    use crate::rpc::{Capabilities, RpcContext};
    use std::sync::Arc;

    fn test_caps() -> Capabilities {
        Capabilities { ntm: false, tmux: true, stream: false, systemd: false }
    }

    fn test_ctx() -> RpcContext {
        let cache = Arc::new(Cache::new(100));
        let config = ConfigManager::default();
        RpcContext::with_capabilities(cache, config, test_caps())
    }

    fn make_session(uid: &str, name: &str) -> Session {
        Session {
            session_uid: uid.to_string(),
            source_id: "tmux".to_string(),
            tmux_session_id: None,
            name: name.to_string(),
            created_at: 1000,
            last_seen_at: 2000,
            ended_at: None,
            status: SessionStatus::Active,
            status_reason: None,
            pane_count: 0,
            metadata: None,
        }
    }

    fn make_pane(uid: &str, session_uid: &str) -> Pane {
        Pane {
            pane_uid: uid.to_string(),
            session_uid: session_uid.to_string(),
            pane_index: 0,
            tmux_pane_id: None,
            tmux_window_id: None,
            tmux_pane_pid: None,
            agent_type: None,
            created_at: 1000,
            last_seen_at: 2000,
            last_activity_at: Some(1500),
            current_command: None,
            ended_at: None,
            status: PaneStatus::Active,
            status_reason: None,
        }
    }

    #[test]
    fn hello_returns_daemon_version() {
        let ctx = test_ctx();
        let result = hello(&ctx).unwrap();
        assert!(result["daemonVersion"].is_string());
        assert_eq!(result["protocolVersion"], 1);
        assert_eq!(result["schemaVersion"], 1);
        assert!(result["instanceId"].is_string());
        assert!(result["runId"].is_string());
        assert_eq!(result["capabilities"]["tmux"], true);
    }

    #[test]
    fn health_get_returns_status() {
        let ctx = test_ctx();
        ctx.cache.set_health(HealthStatus {
            status: "ok".to_string(),
            last_error: None,
        });
        let result = health_get(&ctx).unwrap();
        assert_eq!(result["status"], "ok");
        assert!(result["uptime"].is_number());
        assert!(result["version"].is_string());
        assert!(result["instanceId"].is_string());
        assert_eq!(result["lastError"], Value::Null);
    }

    #[test]
    fn health_get_includes_error() {
        let ctx = test_ctx();
        ctx.cache.set_health(HealthStatus {
            status: "degraded".to_string(),
            last_error: Some("tmux timeout".to_string()),
        });
        let result = health_get(&ctx).unwrap();
        assert_eq!(result["status"], "degraded");
        assert_eq!(result["lastError"], "tmux timeout");
    }

    #[test]
    fn health_get_includes_capabilities() {
        let ctx = test_ctx();
        let result = health_get(&ctx).unwrap();
        assert_eq!(result["capabilities"]["tmux"], true);
        assert_eq!(result["capabilities"]["ntm"], false);
    }

    #[test]
    fn capabilities_get_returns_caps() {
        let ctx = test_ctx();
        let result = capabilities_get(&ctx).unwrap();
        assert_eq!(result["protocolVersion"], 1);
        assert_eq!(result["schemaVersion"], 1);
        assert_eq!(result["capabilities"]["tmux"], true);
        assert_eq!(result["capabilities"]["ntm"], false);
    }

    #[test]
    fn snapshot_get_empty_cache() {
        let ctx = test_ctx();
        let result = snapshot_get(&ctx).unwrap();
        assert!(result["sessions"].as_array().unwrap().is_empty());
        assert!(result["panes"].as_array().unwrap().is_empty());
        assert!(result["events"].as_array().unwrap().is_empty());
        assert_eq!(result["lastEventId"], 0);
        assert!(result["stats"]["summary"].is_object());
    }

    #[test]
    fn snapshot_get_with_data() {
        let ctx = test_ctx();
        ctx.cache.upsert_session(make_session("s1", "alpha"));
        ctx.cache.upsert_pane(make_pane("p1", "s1"));
        ctx.cache.record_event(EventRecord {
            event_id: Some(1),
            session_uid: "s1".to_string(),
            pane_uid: "p1".to_string(),
            event_type: "compact".to_string(),
            detected_at: 100,
            severity: None,
            status: None,
        });
        ctx.cache.set_stats_today(StatsAggregate {
            total_compacts: 5,
            active_minutes: 30,
            estimated_tokens: 10000,
        });

        let result = snapshot_get(&ctx).unwrap();
        assert_eq!(result["sessions"].as_array().unwrap().len(), 1);
        assert_eq!(result["panes"].as_array().unwrap().len(), 1);
        assert_eq!(result["events"].as_array().unwrap().len(), 1);
        assert_eq!(result["lastEventId"], 1);
        assert_eq!(result["stats"]["summary"]["totalCompacts"], 5);
    }
}
