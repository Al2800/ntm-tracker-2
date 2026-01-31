//! Debug and diagnostics endpoints (admin only).

use crate::cache::PollingState;
use crate::metrics::METRICS;
use crate::rpc::{require_admin, RpcContext, RpcResult};
use serde::Serialize;
use serde_json::{json, Value};

/// GET debug.diagnostics - Internal state inspection.
pub fn diagnostics(ctx: &RpcContext) -> RpcResult<Value> {
    require_admin(ctx)?;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Diagnostics {
        version: &'static str,
        instance_id: String,
        run_id: String,
        uptime_secs: u64,
        protocol_version: u32,
        schema_version: u32,
        capabilities: crate::rpc::Capabilities,
        cache_stats: CacheStats,
        polling: PollingState,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct CacheStats {
        session_count: usize,
        pane_count: usize,
        event_count: usize,
    }

    let cache = &ctx.cache;
    let diagnostics = Diagnostics {
        version: crate::version(),
        instance_id: ctx.instance_id.clone(),
        run_id: ctx.run_id.clone(),
        uptime_secs: ctx.uptime_secs(),
        protocol_version: ctx.protocol_version,
        schema_version: ctx.schema_version,
        capabilities: ctx.capabilities.clone(),
        cache_stats: CacheStats {
            session_count: cache.session_count(),
            pane_count: cache.pane_count(),
            event_count: cache.event_count(),
        },
        polling: cache.polling_state(),
    };

    serde_json::to_value(diagnostics).map_err(|e| {
        crate::rpc::RpcError::new(crate::rpc::CODE_DEGRADED, e.to_string())
    })
}

/// GET debug.self-test - Validate daemon can reach dependencies.
pub fn self_test(ctx: &RpcContext) -> RpcResult<Value> {
    require_admin(ctx)?;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct TestResult {
        name: &'static str,
        ok: bool,
        detail: Option<String>,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct SelfTestResult {
        ok: bool,
        checks: Vec<TestResult>,
    }

    let mut checks = Vec::new();

    // Test tmux availability
    let tmux_result = std::process::Command::new("tmux")
        .arg("-V")
        .output();
    checks.push(match tmux_result {
        Ok(output) if output.status.success() => TestResult {
            name: "tmux",
            ok: true,
            detail: Some(String::from_utf8_lossy(&output.stdout).trim().to_string()),
        },
        Ok(output) => TestResult {
            name: "tmux",
            ok: false,
            detail: Some(format!("exit code: {:?}", output.status.code())),
        },
        Err(e) => TestResult {
            name: "tmux",
            ok: false,
            detail: Some(e.to_string()),
        },
    });

    // Test ntm availability
    let ntm_result = std::process::Command::new("ntm")
        .arg("--version")
        .output();
    checks.push(match ntm_result {
        Ok(output) if output.status.success() => TestResult {
            name: "ntm",
            ok: true,
            detail: Some(String::from_utf8_lossy(&output.stdout).trim().to_string()),
        },
        Ok(output) => TestResult {
            name: "ntm",
            ok: false,
            detail: Some(format!("exit code: {:?}", output.status.code())),
        },
        Err(e) => TestResult {
            name: "ntm",
            ok: false,
            detail: Some(format!("not available: {e}")),
        },
    });

    // Test cache read (always succeeds if we got here)
    checks.push(TestResult {
        name: "cache",
        ok: true,
        detail: Some(format!("{} sessions cached", ctx.cache.session_count())),
    });

    let all_ok = checks.iter().filter(|c| c.name != "ntm").all(|c| c.ok);

    let result = SelfTestResult {
        ok: all_ok,
        checks,
    };

    serde_json::to_value(result).map_err(|e| {
        crate::rpc::RpcError::new(crate::rpc::CODE_DEGRADED, e.to_string())
    })
}

/// GET debug.metrics - Performance metrics.
pub fn metrics(ctx: &RpcContext) -> RpcResult<Value> {
    require_admin(ctx)?;

    let summary = METRICS.summary();

    Ok(json!({
        "timings": {
            "tmuxCmd": histogram_json(&summary.tmux_cmd),
            "ntmCmd": histogram_json(&summary.ntm_cmd),
            "pollCycle": histogram_json(&summary.poll_cycle),
            "eventProcessing": histogram_json(&summary.event_processing),
            "dbWrite": histogram_json(&summary.db_write),
            "rpcRequest": histogram_json(&summary.rpc_request),
        },
        "counters": {
            "sessionCount": ctx.cache.session_count(),
            "paneCount": ctx.cache.pane_count(),
            "eventCount": ctx.cache.event_count(),
        }
    }))
}

fn histogram_json(stats: &crate::metrics::HistogramStats) -> Value {
    json!({
        "count": stats.count,
        "minUs": stats.min_us,
        "maxUs": stats.max_us,
        "avgUs": stats.avg_us,
        "sumUs": stats.sum_us,
    })
}

/// GET debug.log-tail - Recent log lines.
/// Note: This is a placeholder - actual log tailing requires ring buffer or file read.
pub fn log_tail(ctx: &RpcContext) -> RpcResult<Value> {
    require_admin(ctx)?;

    // For now, return a message indicating logs should be read from file
    Ok(json!({
        "message": "Log tailing not yet implemented. Check log file directly.",
        "configuredFile": ctx.config.current().logging.file,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::Cache;
    use crate::config::ConfigManager;
    use std::sync::Arc;

    fn admin_context() -> RpcContext {
        let cache = Arc::new(Cache::new(100));
        let config = ConfigManager::default();
        let mut ctx = RpcContext::new(cache, config);
        ctx.is_admin = true;
        ctx
    }

    fn non_admin_context() -> RpcContext {
        let cache = Arc::new(Cache::new(100));
        let config = ConfigManager::default();
        RpcContext::new(cache, config)
    }

    #[test]
    fn diagnostics_requires_admin() {
        let ctx = non_admin_context();
        let result = diagnostics(&ctx);
        assert!(result.is_err());
    }

    #[test]
    fn diagnostics_returns_info() {
        let ctx = admin_context();
        let result = diagnostics(&ctx).unwrap();
        assert!(result.get("version").is_some());
        assert!(result.get("instanceId").is_some());
        assert!(result.get("uptimeSecs").is_some());
    }

    #[test]
    fn self_test_requires_admin() {
        let ctx = non_admin_context();
        let result = self_test(&ctx);
        assert!(result.is_err());
    }

    #[test]
    fn metrics_requires_admin() {
        let ctx = non_admin_context();
        let result = metrics(&ctx);
        assert!(result.is_err());
    }

    #[test]
    fn metrics_returns_timings() {
        let ctx = admin_context();
        let result = metrics(&ctx).unwrap();
        assert!(result.get("timings").is_some());
        assert!(result.get("counters").is_some());
    }
}
