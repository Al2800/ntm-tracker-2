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
