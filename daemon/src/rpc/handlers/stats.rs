use crate::cache::Cache;
use crate::rpc::{parse_params, RpcContext, RpcResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsSummary {
    pub sessions: usize,
    pub panes: usize,
    pub total_compacts: u64,
    pub active_minutes: u64,
    pub estimated_tokens: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct StatsRangeParams {
    session_id: Option<String>,
    start: Option<i64>,
    end: Option<i64>,
    limit: Option<usize>,
}

pub fn summary_payload(cache: &Cache) -> StatsSummary {
    let stats_today = cache.stats_today();
    StatsSummary {
        sessions: cache.all_sessions().len(),
        panes: cache.all_panes().len(),
        total_compacts: stats_today.total_compacts,
        active_minutes: stats_today.active_minutes,
        estimated_tokens: stats_today.estimated_tokens,
    }
}

pub fn summary(ctx: &RpcContext) -> RpcResult<Value> {
    Ok(json!({ "summary": summary_payload(ctx.cache.as_ref()) }))
}

pub fn hourly(_ctx: &RpcContext, params: Value) -> RpcResult<Value> {
    let _params: StatsRangeParams = if params.is_null() {
        StatsRangeParams {
            session_id: None,
            start: None,
            end: None,
            limit: None,
        }
    } else {
        parse_params(params)?
    };

    Ok(json!({ "hourly": [] }))
}

pub fn daily(_ctx: &RpcContext, params: Value) -> RpcResult<Value> {
    let _params: StatsRangeParams = if params.is_null() {
        StatsRangeParams {
            session_id: None,
            start: None,
            end: None,
            limit: None,
        }
    } else {
        parse_params(params)?
    };

    Ok(json!({ "daily": [] }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::{Cache, StatsAggregate};
    use crate::config::ConfigManager;
    use crate::models::pane::{Pane, PaneStatus};
    use crate::models::session::{Session, SessionStatus};
    use crate::rpc::{Capabilities, RpcContext};
    use std::sync::Arc;

    fn test_ctx() -> RpcContext {
        let cache = Arc::new(Cache::new(100));
        let config = ConfigManager::default();
        let caps = Capabilities { ntm: false, tmux: false, stream: false, systemd: false };
        RpcContext::with_capabilities(cache, config, caps)
    }

    #[test]
    fn summary_empty_cache() {
        let ctx = test_ctx();
        let result = summary(&ctx).unwrap();
        assert_eq!(result["summary"]["sessions"], 0);
        assert_eq!(result["summary"]["panes"], 0);
        assert_eq!(result["summary"]["totalCompacts"], 0);
    }

    #[test]
    fn summary_with_data() {
        let ctx = test_ctx();
        ctx.cache.upsert_session(Session {
            session_uid: "s1".to_string(), source_id: "tmux".to_string(),
            tmux_session_id: None, name: "alpha".to_string(),
            created_at: 1, last_seen_at: 1, ended_at: None,
            status: SessionStatus::Active, status_reason: None,
            pane_count: 0, metadata: None,
        });
        ctx.cache.upsert_pane(Pane {
            pane_uid: "p1".to_string(), session_uid: "s1".to_string(),
            pane_index: 0, tmux_pane_id: None, tmux_window_id: None,
            tmux_pane_pid: None, agent_type: None, created_at: 1,
            last_seen_at: 1, last_activity_at: None, current_command: None,
            ended_at: None, status: PaneStatus::Active, status_reason: None,
        });
        ctx.cache.set_stats_today(StatsAggregate {
            total_compacts: 10,
            active_minutes: 60,
            estimated_tokens: 5000,
        });
        let result = summary(&ctx).unwrap();
        assert_eq!(result["summary"]["sessions"], 1);
        assert_eq!(result["summary"]["panes"], 1);
        assert_eq!(result["summary"]["totalCompacts"], 10);
        assert_eq!(result["summary"]["activeMinutes"], 60);
        assert_eq!(result["summary"]["estimatedTokens"], 5000);
    }

    #[test]
    fn hourly_returns_empty_array() {
        let ctx = test_ctx();
        let result = hourly(&ctx, Value::Null).unwrap();
        assert!(result["hourly"].as_array().unwrap().is_empty());
    }

    #[test]
    fn daily_returns_empty_array() {
        let ctx = test_ctx();
        let result = daily(&ctx, Value::Null).unwrap();
        assert!(result["daily"].as_array().unwrap().is_empty());
    }

    #[test]
    fn hourly_with_params() {
        let ctx = test_ctx();
        let result = hourly(&ctx, json!({"sessionId": "s1", "limit": 24})).unwrap();
        assert!(result["hourly"].as_array().unwrap().is_empty());
    }

    #[test]
    fn summary_payload_counts_sessions_and_panes() {
        let ctx = test_ctx();
        ctx.cache.upsert_session(Session {
            session_uid: "s1".to_string(), source_id: "tmux".to_string(),
            tmux_session_id: None, name: "a".to_string(),
            created_at: 1, last_seen_at: 1, ended_at: None,
            status: SessionStatus::Active, status_reason: None,
            pane_count: 0, metadata: None,
        });
        ctx.cache.upsert_session(Session {
            session_uid: "s2".to_string(), source_id: "tmux".to_string(),
            tmux_session_id: None, name: "b".to_string(),
            created_at: 1, last_seen_at: 1, ended_at: None,
            status: SessionStatus::Active, status_reason: None,
            pane_count: 0, metadata: None,
        });
        let payload = summary_payload(ctx.cache.as_ref());
        assert_eq!(payload.sessions, 2);
        assert_eq!(payload.panes, 0);
    }
}
