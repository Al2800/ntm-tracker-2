use crate::cache::{Cache, EventRecord};
use crate::rpc::{parse_params, RpcContext, RpcError, RpcResult, CODE_UNSUPPORTED};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventView {
    id: i64,
    event_type: String,
    session_id: String,
    pane_id: String,
    detected_at: i64,
    severity: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EscalationView {
    id: i64,
    session_id: String,
    pane_id: String,
    detected_at: i64,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventsListParams {
    cursor: Option<i64>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubscribeParams {
    channels: Vec<String>,
    since_event_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EscalationDismissParams {
    escalation_id: i64,
}

fn to_event_view(record: EventRecord) -> EventView {
    EventView {
        id: record.event_id.unwrap_or(0),
        event_type: record.event_type,
        session_id: record.session_uid,
        pane_id: record.pane_uid,
        detected_at: record.detected_at,
        severity: record.severity,
        status: record.status,
    }
}

pub fn event_views(cache: &Cache, cursor: Option<i64>, limit: Option<usize>) -> Vec<EventView> {
    let mut records: Vec<EventView> = cache
        .recent_events()
        .into_iter()
        .filter(|record| cursor.map(|c| record.event_id.unwrap_or(0) > c).unwrap_or(true))
        .map(to_event_view)
        .collect();

    records.sort_by_key(|event| event.id);
    if let Some(limit) = limit {
        records.truncate(limit);
    }
    records
}

pub fn last_event_id(cache: &Cache) -> i64 {
    cache
        .recent_events()
        .iter()
        .filter_map(|event| event.event_id)
        .max()
        .unwrap_or(0)
}

pub fn list(ctx: &RpcContext, params: Value) -> RpcResult<Value> {
    let params: EventsListParams = if params.is_null() {
        EventsListParams {
            cursor: None,
            limit: None,
        }
    } else {
        parse_params(params)?
    };

    let events = event_views(ctx.cache.as_ref(), params.cursor, params.limit);
    let next_event_id = events.last().map(|event| event.id + 1).unwrap_or(0);

    Ok(json!({
        "events": events,
        "nextEventId": next_event_id
    }))
}

pub fn subscribe(ctx: &RpcContext, params: Value) -> RpcResult<Value> {
    let params: SubscribeParams = parse_params(params)?;
    let last_event_id = params
        .since_event_id
        .unwrap_or_else(|| last_event_id(ctx.cache.as_ref()));
    Ok(json!({
        "subscribed": true,
        "channels": params.channels,
        "lastEventId": last_event_id
    }))
}

pub fn escalations_list(ctx: &RpcContext) -> RpcResult<Value> {
    let escalations: Vec<EscalationView> = ctx
        .cache
        .recent_events()
        .into_iter()
        .filter(|event| event.event_type == "escalation")
        .map(|event| EscalationView {
            id: event.event_id.unwrap_or(0),
            session_id: event.session_uid,
            pane_id: event.pane_uid,
            detected_at: event.detected_at,
            status: event.status,
        })
        .collect();

    Ok(json!({ "escalations": escalations }))
}

pub fn escalations_dismiss(_ctx: &RpcContext, params: Value) -> RpcResult<Value> {
    let params: EscalationDismissParams = parse_params(params)?;
    Err(RpcError::new(
        CODE_UNSUPPORTED,
        format!(
            "escalations.dismiss not implemented for escalation {}",
            params.escalation_id
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::Cache;
    use crate::config::ConfigManager;
    use crate::rpc::{Capabilities, RpcContext};
    use std::sync::Arc;

    fn test_ctx() -> RpcContext {
        let cache = Arc::new(Cache::new(100));
        let config = ConfigManager::default();
        let caps = Capabilities { ntm: false, tmux: false, stream: false, systemd: false };
        RpcContext::with_capabilities(cache, config, caps)
    }

    fn test_ctx_with_events() -> RpcContext {
        let ctx = test_ctx();
        for i in 1..=5 {
            ctx.cache.record_event(EventRecord {
                event_id: Some(i),
                session_uid: format!("sess-{i}"),
                pane_uid: format!("pane-{i}"),
                event_type: if i == 3 { "escalation".to_string() } else { "compact".to_string() },
                detected_at: 1000 + i,
                severity: Some("info".to_string()),
                status: if i == 3 { Some("pending".to_string()) } else { None },
            });
        }
        ctx
    }

    #[test]
    fn events_list_empty_cache() {
        let ctx = test_ctx();
        let result = list(&ctx, Value::Null).unwrap();
        assert!(result["events"].as_array().unwrap().is_empty());
        assert_eq!(result["nextEventId"], 0);
    }

    #[test]
    fn events_list_returns_all_events() {
        let ctx = test_ctx_with_events();
        let result = list(&ctx, Value::Null).unwrap();
        let events = result["events"].as_array().unwrap();
        assert_eq!(events.len(), 5);
    }

    #[test]
    fn events_list_with_cursor_filters() {
        let ctx = test_ctx_with_events();
        let result = list(&ctx, serde_json::json!({"cursor": 3})).unwrap();
        let events = result["events"].as_array().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0]["id"], 4);
        assert_eq!(events[1]["id"], 5);
    }

    #[test]
    fn events_list_with_limit() {
        let ctx = test_ctx_with_events();
        let result = list(&ctx, serde_json::json!({"limit": 2})).unwrap();
        let events = result["events"].as_array().unwrap();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn events_list_cursor_and_limit_combined() {
        let ctx = test_ctx_with_events();
        let result = list(&ctx, serde_json::json!({"cursor": 2, "limit": 1})).unwrap();
        let events = result["events"].as_array().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["id"], 3);
    }

    #[test]
    fn events_list_next_event_id() {
        let ctx = test_ctx_with_events();
        let result = list(&ctx, Value::Null).unwrap();
        // Last event id is 5, so nextEventId should be 6
        assert_eq!(result["nextEventId"], 6);
    }

    #[test]
    fn escalations_list_filters_by_type() {
        let ctx = test_ctx_with_events();
        let result = escalations_list(&ctx).unwrap();
        let escalations = result["escalations"].as_array().unwrap();
        assert_eq!(escalations.len(), 1);
        assert_eq!(escalations[0]["id"], 3);
        assert_eq!(escalations[0]["status"], "pending");
    }

    #[test]
    fn escalations_list_empty_when_no_escalations() {
        let ctx = test_ctx();
        // Add a non-escalation event
        ctx.cache.record_event(EventRecord {
            event_id: Some(1),
            session_uid: "s".to_string(),
            pane_uid: "p".to_string(),
            event_type: "compact".to_string(),
            detected_at: 100,
            severity: None,
            status: None,
        });
        let result = escalations_list(&ctx).unwrap();
        assert!(result["escalations"].as_array().unwrap().is_empty());
    }

    #[test]
    fn escalations_dismiss_returns_unsupported() {
        let ctx = test_ctx();
        let result = escalations_dismiss(&ctx, serde_json::json!({"escalationId": 42}));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, CODE_UNSUPPORTED);
    }

    #[test]
    fn last_event_id_with_events() {
        let ctx = test_ctx_with_events();
        assert_eq!(last_event_id(ctx.cache.as_ref()), 5);
    }

    #[test]
    fn last_event_id_empty_cache() {
        let ctx = test_ctx();
        assert_eq!(last_event_id(ctx.cache.as_ref()), 0);
    }

    #[test]
    fn subscribe_returns_channels() {
        let ctx = test_ctx();
        let result = subscribe(&ctx, serde_json::json!({
            "channels": ["sessions", "events"]
        })).unwrap();
        assert_eq!(result["subscribed"], true);
        let channels = result["channels"].as_array().unwrap();
        assert_eq!(channels.len(), 2);
    }
}
