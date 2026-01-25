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
