//! Integration tests for the NTM tracker daemon.
//!
//! These tests verify end-to-end behavior with mocked ntm/tmux outputs.
//! Run with: cargo test --test integration

use ntm_tracker_daemon::cache::{Cache, EventRecord, HealthStatus, StatsAggregate};
use ntm_tracker_daemon::config::ConfigManager;
use ntm_tracker_daemon::models::pane::{Pane, PaneStatus};
use ntm_tracker_daemon::models::session::{Session, SessionStatus};
use ntm_tracker_daemon::rpc::{handle, RpcContext};
use serde_json::json;
use std::sync::Arc;

fn test_context() -> RpcContext {
    let cache = Arc::new(Cache::new(100));
    let config = ConfigManager::default();
    RpcContext::new(cache, config)
}

fn test_context_with_data() -> RpcContext {
    let cache = Arc::new(Cache::new(100));

    // Add test session
    let mut session = Session::new("ntm", "test-session", Some("$1".to_string()), 1000);
    session.status = SessionStatus::Active;
    session.status_reason = Some("recent_activity".to_string());
    session.pane_count = 2;
    cache.upsert_session(session.clone());

    // Add test panes
    let mut pane1 = Pane::new(&session.session_uid, 0, 1000, Some("%0".to_string()), None, None);
    pane1.status = PaneStatus::Active;
    pane1.current_command = Some("claude".to_string());
    pane1.agent_type = Some("claude".to_string());
    cache.upsert_pane(pane1);

    let mut pane2 = Pane::new(&session.session_uid, 1, 1001, Some("%1".to_string()), None, None);
    pane2.status = PaneStatus::Waiting;
    pane2.current_command = Some("bash".to_string());
    cache.upsert_pane(pane2);

    // Add test event
    cache.record_event(EventRecord {
        event_id: Some(1),
        session_uid: session.session_uid.clone(),
        pane_uid: "pane-1".to_string(),
        event_type: "compact".to_string(),
        detected_at: 1000,
        severity: Some("info".to_string()),
        status: Some("pending".to_string()),
    });

    // Set health
    cache.set_health(HealthStatus {
        status: "ok".to_string(),
        last_error: None,
    });

    // Set stats
    cache.set_stats_today(StatsAggregate {
        total_compacts: 5,
        active_minutes: 120,
        estimated_tokens: 50000,
    });

    let config = ConfigManager::default();
    RpcContext::new(cache, config)
}

// ============================================================================
// Startup and Health Tests
// ============================================================================

#[test]
fn health_get_returns_ok_status() {
    let ctx = test_context();
    ctx.cache.set_health(HealthStatus {
        status: "ok".to_string(),
        last_error: None,
    });

    let result = handle("health.get", json!(null), &ctx);
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response["status"], "ok");
    assert!(response["uptime"].is_number());
    assert!(response["instanceId"].is_string());
    assert!(response["version"].is_string());
}

#[test]
fn health_get_reports_degraded_status() {
    let ctx = test_context();
    ctx.cache.set_health(HealthStatus {
        status: "degraded".to_string(),
        last_error: Some("Connection timeout".to_string()),
    });

    let result = handle("health.get", json!(null), &ctx);
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response["status"], "degraded");
    assert_eq!(response["lastError"], "Connection timeout");
}

#[test]
fn capabilities_get_returns_feature_flags() {
    let ctx = test_context();

    let result = handle("capabilities.get", json!(null), &ctx);
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response["protocolVersion"], 1);
    assert_eq!(response["schemaVersion"], 1);
    assert!(response["capabilities"]["ntm"].is_boolean());
    assert!(response["capabilities"]["tmux"].is_boolean());
}

// ============================================================================
// Session Discovery Tests
// ============================================================================

#[test]
fn sessions_list_returns_empty_on_startup() {
    let ctx = test_context();

    let result = handle("sessions.list", json!(null), &ctx);
    assert!(result.is_ok());

    let response = result.unwrap();
    let sessions = response["sessions"].as_array().unwrap();
    assert!(sessions.is_empty());
}

#[test]
fn sessions_list_returns_discovered_sessions() {
    let ctx = test_context_with_data();

    let result = handle("sessions.list", json!(null), &ctx);
    assert!(result.is_ok());

    let response = result.unwrap();
    let sessions = response["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0]["name"], "test-session");
    assert_eq!(sessions[0]["status"], "active");
}

#[test]
fn sessions_list_filters_by_status() {
    let ctx = test_context_with_data();

    // Filter for idle sessions (none exist)
    let result = handle("sessions.list", json!({"status": "idle"}), &ctx);
    assert!(result.is_ok());
    let response = result.unwrap();
    let sessions = response["sessions"].as_array().unwrap();
    assert!(sessions.is_empty());

    // Filter for active sessions
    let result = handle("sessions.list", json!({"status": "active"}), &ctx);
    assert!(result.is_ok());
    let response = result.unwrap();
    let sessions = response["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 1);
}

#[test]
fn sessions_get_returns_session_by_id() {
    let ctx = test_context_with_data();

    // Get session IDs first
    let list_result = handle("sessions.list", json!(null), &ctx);
    let sessions = list_result.unwrap()["sessions"].as_array().unwrap().clone();
    let session_id = sessions[0]["sessionId"].as_str().unwrap();

    // Now get the specific session
    let result = handle("sessions.get", json!({"sessionId": session_id}), &ctx);
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response["session"]["name"], "test-session");
}

#[test]
fn sessions_get_returns_not_found() {
    let ctx = test_context();

    let result = handle("sessions.get", json!({"sessionId": "nonexistent"}), &ctx);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.code, "NOT_FOUND");
}

// ============================================================================
// Pane Tests
// ============================================================================

#[test]
fn panes_get_returns_pane_by_id() {
    let ctx = test_context_with_data();

    // Get pane IDs via snapshot
    let snapshot = handle("snapshot.get", json!(null), &ctx).unwrap();
    let panes = snapshot["panes"].as_array().unwrap();
    assert_eq!(panes.len(), 2);

    let pane_id = panes[0]["paneId"].as_str().unwrap();

    let result = handle("panes.get", json!({"paneId": pane_id}), &ctx);
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response["pane"]["status"].is_string());
}

#[test]
fn panes_get_returns_not_found() {
    let ctx = test_context();

    let result = handle("panes.get", json!({"paneId": "nonexistent"}), &ctx);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.code, "NOT_FOUND");
}

// ============================================================================
// Event Tests
// ============================================================================

#[test]
fn events_list_returns_recorded_events() {
    let ctx = test_context_with_data();

    let result = handle("events.list", json!(null), &ctx);
    assert!(result.is_ok());

    let response = result.unwrap();
    let events = response["events"].as_array().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["eventType"], "compact");
}

#[test]
fn events_list_empty_on_startup() {
    let ctx = test_context();

    let result = handle("events.list", json!(null), &ctx);
    assert!(result.is_ok());

    let response = result.unwrap();
    let events = response["events"].as_array().unwrap();
    assert!(events.is_empty());
}

// ============================================================================
// Stats Tests
// ============================================================================

#[test]
fn stats_summary_returns_aggregates() {
    let ctx = test_context_with_data();

    let result = handle("stats.summary", json!(null), &ctx);
    assert!(result.is_ok());

    let response = result.unwrap();
    let summary = &response["summary"];
    assert_eq!(summary["totalCompacts"], 5);
    assert_eq!(summary["activeMinutes"], 120);
    assert_eq!(summary["estimatedTokens"], 50000);
}

// ============================================================================
// Snapshot Tests
// ============================================================================

#[test]
fn snapshot_get_returns_full_state() {
    let ctx = test_context_with_data();

    let result = handle("snapshot.get", json!(null), &ctx);
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response["sessions"].is_array());
    assert!(response["panes"].is_array());
    assert!(response["events"].is_array());
    assert!(response["stats"].is_object());

    let sessions = response["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 1);

    let panes = response["panes"].as_array().unwrap();
    assert_eq!(panes.len(), 2);
}

// ============================================================================
// Admin Tests
// ============================================================================

#[test]
fn admin_methods_require_auth() {
    let ctx = test_context();
    // ctx.is_admin is false by default

    // config.set requires admin auth
    let result = handle("config.set", json!({"test": "value"}), &ctx);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.code, "FORBIDDEN");
}

#[test]
fn admin_methods_work_with_auth() {
    let mut ctx = test_context();
    ctx.is_admin = true;

    // config.set should work with admin auth
    let result = handle("config.set", json!({"test": "value"}), &ctx);
    assert!(result.is_ok());
}

#[test]
fn config_get_works_without_auth() {
    let ctx = test_context();
    // config.get doesn't require admin

    let result = handle("config.get", json!(null), &ctx);
    assert!(result.is_ok());
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn unsupported_method_returns_error() {
    let ctx = test_context();

    let result = handle("unknown.method", json!(null), &ctx);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.code, "UNSUPPORTED");
}

#[test]
fn invalid_params_returns_error() {
    let ctx = test_context();

    let result = handle("sessions.get", json!({"wrongParam": 123}), &ctx);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.code, "INVALID_PARAMS");
}

// ============================================================================
// Concurrent RPC Tests (bd-1bgf)
// ============================================================================

#[test]
fn concurrent_snapshot_and_health() {
    let ctx = Arc::new(test_context_with_data());
    let ctx1 = ctx.clone();
    let ctx2 = ctx.clone();

    let t1 = std::thread::spawn(move || handle("snapshot.get", json!(null), &ctx1));
    let t2 = std::thread::spawn(move || handle("health.get", json!(null), &ctx2));

    let r1 = t1.join().unwrap();
    let r2 = t2.join().unwrap();

    assert!(r1.is_ok(), "snapshot.get should succeed");
    assert!(r2.is_ok(), "health.get should succeed");

    let snap = r1.unwrap();
    assert!(snap["sessions"].is_array());
    assert!(snap["panes"].is_array());

    let health = r2.unwrap();
    assert!(health["status"].is_string());
}

#[test]
fn concurrent_sessions_and_events() {
    let ctx = Arc::new(test_context_with_data());
    let ctx1 = ctx.clone();
    let ctx2 = ctx.clone();

    let t1 = std::thread::spawn(move || handle("sessions.list", json!(null), &ctx1));
    let t2 = std::thread::spawn(move || handle("events.list", json!(null), &ctx2));

    let sessions_result = t1.join().unwrap().unwrap();
    let events_result = t2.join().unwrap().unwrap();

    let sessions = sessions_result["sessions"].as_array().unwrap();
    let events = events_result["events"].as_array().unwrap();

    assert_eq!(sessions.len(), 1);
    assert_eq!(events.len(), 1);
}

#[test]
fn ten_rapid_concurrent_requests() {
    let ctx = Arc::new(test_context_with_data());

    let methods = vec![
        "health.get",
        "sessions.list",
        "snapshot.get",
        "events.list",
        "stats.summary",
        "capabilities.get",
        "health.get",
        "sessions.list",
        "snapshot.get",
        "events.list",
    ];

    let handles: Vec<_> = methods
        .into_iter()
        .map(|method| {
            let ctx = ctx.clone();
            let method = method.to_string();
            std::thread::spawn(move || {
                let result = handle(&method, json!(null), &ctx);
                (method, result)
            })
        })
        .collect();

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // All 10 should succeed
    for (method, result) in &results {
        assert!(
            result.is_ok(),
            "method {method} should succeed, got: {:?}",
            result.as_ref().unwrap_err()
        );
    }
    assert_eq!(results.len(), 10);
}

#[test]
fn concurrent_reads_and_writes() {
    // Tests concurrent read (list) and write (config.set) operations
    let mut admin_ctx = test_context_with_data();
    admin_ctx.is_admin = true;
    let ctx = Arc::new(admin_ctx);

    let ctx_read = ctx.clone();
    let ctx_write = ctx.clone();

    let reader = std::thread::spawn(move || {
        let mut results = Vec::new();
        for _ in 0..5 {
            results.push(handle("sessions.list", json!(null), &ctx_read));
        }
        results
    });

    let writer = std::thread::spawn(move || {
        let mut results = Vec::new();
        for _ in 0..5 {
            results.push(handle("config.set", json!({"key": "value"}), &ctx_write));
        }
        results
    });

    let read_results = reader.join().unwrap();
    let write_results = writer.join().unwrap();

    // All reads should succeed
    for r in &read_results {
        assert!(r.is_ok());
    }
    // All writes should succeed (admin context)
    for r in &write_results {
        assert!(r.is_ok());
    }
}

#[test]
fn concurrent_valid_and_invalid_requests() {
    // Mix of valid and invalid requests — errors shouldn't affect valid ones
    let ctx = Arc::new(test_context_with_data());

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                if i % 2 == 0 {
                    // Valid request
                    ("health.get", handle("health.get", json!(null), &ctx))
                } else {
                    // Invalid request
                    (
                        "no.such.method",
                        handle("no.such.method", json!(null), &ctx),
                    )
                }
            })
        })
        .collect();

    let mut successes = 0;
    let mut errors = 0;
    for h in handles {
        let (method, result) = h.join().unwrap();
        match (method, &result) {
            ("health.get", Ok(_)) => successes += 1,
            ("no.such.method", Err(_)) => errors += 1,
            _ => panic!("unexpected result for {method}: {result:?}"),
        }
    }
    assert_eq!(successes, 5);
    assert_eq!(errors, 5);
}

// ============================================================================
// Large Dataset Performance Tests (bd-jxa7)
// ============================================================================

fn populate_large_dataset(cache: &Cache, num_sessions: usize, panes_per_session: usize, num_events: usize) {
    let now = 1_700_000_000_i64;
    for s in 0..num_sessions {
        let mut session = Session::new(
            "ntm",
            format!("session-{s}"),
            Some(format!("${s}")),
            now + s as i64,
        );
        session.status = if s % 3 == 0 {
            SessionStatus::Idle
        } else {
            SessionStatus::Active
        };
        session.pane_count = panes_per_session as u32;
        let session_uid = session.session_uid.clone();
        cache.upsert_session(session);

        for p in 0..panes_per_session {
            let mut pane = Pane::new(
                &session_uid,
                p as i32,
                now + p as i64,
                Some(format!("%{}", s * 100 + p)),
                None,
                None,
            );
            pane.status = if p % 2 == 0 {
                PaneStatus::Active
            } else {
                PaneStatus::Waiting
            };
            pane.current_command = Some(format!("cmd-{p}"));
            cache.upsert_pane(pane);
        }
    }

    for e in 0..num_events {
        cache.record_event(EventRecord {
            event_id: Some(e as i64),
            session_uid: format!("session-uid-{}", e % num_sessions),
            pane_uid: format!("pane-uid-{e}"),
            event_type: if e % 2 == 0 { "compact" } else { "escalation" }.to_string(),
            detected_at: now + e as i64,
            severity: Some("info".to_string()),
            status: Some("pending".to_string()),
        });
    }

    cache.set_health(HealthStatus {
        status: "ok".to_string(),
        last_error: None,
    });

    cache.set_stats_today(StatsAggregate {
        total_compacts: num_events as u64 / 2,
        active_minutes: 500,
        estimated_tokens: 250_000,
    });
}

#[test]
fn large_dataset_100_sessions_snapshot() {
    let cache = Arc::new(Cache::new(2000));
    populate_large_dataset(&cache, 100, 5, 500);
    let config = ConfigManager::default();
    let ctx = RpcContext::new(cache, config);

    let start = std::time::Instant::now();
    let result = handle("snapshot.get", json!(null), &ctx).unwrap();
    let elapsed = start.elapsed();

    let sessions = result["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 100);

    let panes = result["panes"].as_array().unwrap();
    assert_eq!(panes.len(), 500); // 100 sessions * 5 panes

    assert!(
        elapsed.as_millis() < 1000,
        "snapshot should complete within 1s, took {}ms",
        elapsed.as_millis()
    );
}

#[test]
fn large_dataset_events_list_all() {
    let cache = Arc::new(Cache::new(2000));
    populate_large_dataset(&cache, 10, 2, 1000);
    let config = ConfigManager::default();
    let ctx = RpcContext::new(cache, config);

    let start = std::time::Instant::now();
    let result = handle("events.list", json!(null), &ctx).unwrap();
    let elapsed = start.elapsed();

    let events = result["events"].as_array().unwrap();
    // Ring buffer is capped at cache capacity (2000), all 1000 fit
    assert_eq!(events.len(), 1000);

    assert!(
        elapsed.as_millis() < 1000,
        "events.list should complete within 1s, took {}ms",
        elapsed.as_millis()
    );
}

#[test]
fn large_dataset_sessions_filter_by_status() {
    let cache = Arc::new(Cache::new(2000));
    populate_large_dataset(&cache, 100, 2, 100);
    let config = ConfigManager::default();
    let ctx = RpcContext::new(cache, config);

    // Filter for idle sessions (every 3rd session: 0, 3, 6, ... = 34 idle sessions)
    let result = handle("sessions.list", json!({"status": "idle"}), &ctx).unwrap();
    let idle_sessions = result["sessions"].as_array().unwrap();
    assert_eq!(idle_sessions.len(), 34, "sessions 0,3,6,...,99 are idle");

    // Filter for active sessions
    let result = handle("sessions.list", json!({"status": "active"}), &ctx).unwrap();
    let active_sessions = result["sessions"].as_array().unwrap();
    assert_eq!(active_sessions.len(), 66, "remaining 66 sessions are active");
}

#[test]
fn large_dataset_stats_summary_correct() {
    let cache = Arc::new(Cache::new(2000));
    populate_large_dataset(&cache, 50, 10, 200);
    let config = ConfigManager::default();
    let ctx = RpcContext::new(cache, config);

    let result = handle("stats.summary", json!(null), &ctx).unwrap();
    let summary = &result["summary"];
    assert_eq!(summary["totalCompacts"], 100); // 200 events / 2
    assert_eq!(summary["activeMinutes"], 500);
    assert_eq!(summary["estimatedTokens"], 250_000);
}

#[test]
fn large_dataset_snapshot_json_size() {
    let cache = Arc::new(Cache::new(2000));
    populate_large_dataset(&cache, 100, 5, 500);
    let config = ConfigManager::default();
    let ctx = RpcContext::new(cache, config);

    let result = handle("snapshot.get", json!(null), &ctx).unwrap();
    let json_str = serde_json::to_string(&result).unwrap();
    let size_kb = json_str.len() / 1024;

    // Verify it's reasonable (should be under 1MB for this dataset)
    assert!(
        size_kb < 1024,
        "snapshot JSON should be under 1MB, got {}KB",
        size_kb
    );
    // But not empty
    assert!(size_kb > 10, "snapshot should have real data, got {}KB", size_kb);
}

#[test]
fn large_dataset_concurrent_operations() {
    let cache = Arc::new(Cache::new(2000));
    populate_large_dataset(&cache, 50, 5, 500);
    let config = ConfigManager::default();
    let ctx = Arc::new(RpcContext::new(cache, config));

    let methods = vec![
        "snapshot.get",
        "sessions.list",
        "events.list",
        "stats.summary",
        "health.get",
    ];

    let start = std::time::Instant::now();
    let handles: Vec<_> = methods
        .into_iter()
        .map(|m| {
            let ctx = ctx.clone();
            let method = m.to_string();
            std::thread::spawn(move || handle(&method, json!(null), &ctx))
        })
        .collect();

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let elapsed = start.elapsed();

    for r in &results {
        assert!(r.is_ok());
    }

    assert!(
        elapsed.as_millis() < 1000,
        "5 concurrent ops on large dataset should complete in 1s, took {}ms",
        elapsed.as_millis()
    );
}

// ============================================================================
// Admin Authentication Flow Tests (bd-13ar)
// ============================================================================

#[test]
fn admin_config_get_shows_all_sections() {
    let ctx = test_context();
    let result = handle("config.get", json!(null), &ctx).unwrap();
    // config.get works without admin
    assert!(result.is_object());
}

#[test]
fn admin_config_set_rejected_without_auth() {
    let ctx = test_context();
    let result = handle("config.set", json!({"polling": {"snapshot_interval_ms": 1000}}), &ctx);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "FORBIDDEN");
}

#[test]
fn admin_config_set_accepted_with_auth() {
    let mut ctx = test_context();
    ctx.is_admin = true;
    let result = handle("config.set", json!({"polling": {"snapshot_interval_ms": 1000}}), &ctx);
    assert!(result.is_ok());
}

#[test]
fn admin_config_reload_rejected_without_auth() {
    let ctx = test_context();
    let result = handle("config.reload", json!(null), &ctx);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "FORBIDDEN");
}

#[test]
fn admin_config_reload_accepted_with_auth() {
    let mut ctx = test_context();
    ctx.is_admin = true;
    let result = handle("config.reload", json!(null), &ctx);
    assert!(result.is_ok());
}

#[test]
fn admin_detectors_reload_rejected_without_auth() {
    let ctx = test_context();
    let result = handle("detectors.reload", json!(null), &ctx);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "FORBIDDEN");
}

#[test]
fn admin_detectors_list_works_without_auth() {
    let ctx = test_context();
    let result = handle("detectors.list", json!(null), &ctx);
    assert!(result.is_ok());
}

#[test]
fn admin_all_protected_methods_require_auth() {
    let ctx = test_context();
    let admin_methods = vec![
        ("config.set", json!({"key": "val"})),
        ("config.reload", json!(null)),
        ("detectors.reload", json!(null)),
    ];

    for (method, params) in admin_methods {
        let result = handle(method, params, &ctx);
        assert!(
            result.is_err(),
            "{method} should require admin auth"
        );
        assert_eq!(
            result.unwrap_err().code,
            "FORBIDDEN",
            "{method} should return FORBIDDEN"
        );
    }
}

#[test]
fn admin_all_protected_methods_succeed_with_auth() {
    let mut ctx = test_context();
    ctx.is_admin = true;
    let admin_methods = vec![
        ("config.set", json!({"key": "val"})),
        ("config.reload", json!(null)),
        ("detectors.reload", json!(null)),
    ];

    for (method, params) in admin_methods {
        let result = handle(method, params, &ctx);
        assert!(
            result.is_ok(),
            "{method} should succeed with admin auth, got: {:?}",
            result.unwrap_err()
        );
    }
}
