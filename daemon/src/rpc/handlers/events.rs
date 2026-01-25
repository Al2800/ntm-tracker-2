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
