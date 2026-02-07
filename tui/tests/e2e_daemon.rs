mod helpers;

use helpers::logging::TestLogger;
use ntm_tracker_tui::msg::{ConnState, Msg};
use ntm_tracker_tui::rpc::types::JsonRpcRequest;
use std::time::Duration;
use tokio::sync::Semaphore;

/// Only allow one daemon-spawning test at a time.
static DAEMON_SEM: Semaphore = Semaphore::const_new(1);

// ================================================================
// bd-38ag: E2E daemon spawn and hello handshake
// bd-cdwb: E2E snapshot request and response cycle
// bd-1n5z: E2E live notification streaming
//
// Consolidated into one daemon session to avoid parallel spawn contention.
// ================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_daemon_handshake_and_snapshots() {
    let _permit = DAEMON_SEM.acquire().await.unwrap();
    let mut harness = helpers::TestHarness::spawn("test_daemon_handshake_and_snapshots")
        .await
        .expect("Failed to spawn daemon");

    // --- bd-38ag: hello handshake ---

    harness.logger.step("Waiting for core.hello notification");
    let version = harness.wait_for_hello(Duration::from_secs(10)).await;

    match &version {
        Some(v) => {
            harness.logger.step_result(true, &format!("Received hello with version '{v}'"));
            assert!(!v.is_empty(), "Version should not be empty");
            assert!(v.contains('.'), "Version should be semver-like, got: {v}");
        }
        None => {
            harness.logger.step_result(false, "Timed out waiting for hello");
            panic!("Did not receive hello within timeout");
        }
    }

    harness.logger.step("Waiting for Connected state");
    let connected = harness
        .wait_for_connection_state(ConnState::Connected, Duration::from_secs(5))
        .await;
    harness.logger.step_result(connected, "Connection state reached Connected");
    assert!(connected, "Should transition to Connected after hello");

    // --- bd-cdwb: snapshot request ---

    harness.logger.step("Waiting for first snapshot notification");
    let snap = harness.wait_for_snapshot(Duration::from_secs(10)).await;

    match &snap {
        Some(s) => {
            harness.logger.step_result(
                true,
                &format!(
                    "Snapshot: {} sessions, {} panes, {} events",
                    s.sessions.len(),
                    s.panes.len(),
                    s.events.len()
                ),
            );

            // Validate session fields
            harness.logger.step("Validating session fields");
            for session in &s.sessions {
                assert!(!session.session_id.is_empty(), "Session ID should not be empty");
                assert!(!session.name.is_empty(), "Session name should not be empty");
                assert!(!session.status.is_empty(), "Session status should not be empty");
                assert!(session.created_at > 0, "Session created_at should be positive");
            }
            harness.logger.step_result(
                true,
                &format!("All {} sessions have valid fields", s.sessions.len()),
            );

            // Validate pane fields
            harness.logger.step("Validating pane fields");
            for pane in &s.panes {
                assert!(!pane.pane_id.is_empty(), "Pane ID should not be empty");
                assert!(!pane.session_id.is_empty(), "Pane session_id should not be empty");
                assert!(!pane.status.is_empty(), "Pane status should not be empty");
                assert!(pane.created_at > 0, "Pane created_at should be positive");
            }
            harness.logger.step_result(
                true,
                &format!("All {} panes have valid fields", s.panes.len()),
            );

            // Verify stats consistency
            harness.logger.step("Checking stats consistency");
            assert_eq!(
                s.stats.summary.sessions,
                s.sessions.len(),
                "stats.sessions should match sessions array length"
            );
            assert_eq!(
                s.stats.summary.panes,
                s.panes.len(),
                "stats.panes should match panes array length"
            );
            harness.logger.step_result(
                true,
                &format!(
                    "Stats consistent: {} sessions, {} panes",
                    s.stats.summary.sessions, s.stats.summary.panes
                ),
            );
        }
        None => {
            harness.logger.step_result(false, "Timed out waiting for snapshot");
            panic!("Did not receive snapshot within timeout");
        }
    }

    // --- bd-cdwb: explicit snapshot.get request ---

    harness.logger.step("Sending explicit snapshot.get request");
    let req = JsonRpcRequest::new(1, "snapshot.get", serde_json::Value::Null);
    let json = serde_json::to_string(&req).unwrap();
    harness.send_raw(&json).await.expect("send failed");
    harness.logger.step_result(true, "Request sent");

    // --- bd-1n5z: live streaming (second snapshot from daemon polling) ---

    harness.logger.step("Waiting for second snapshot (live polling ~2s)");
    let snap2 = harness.wait_for_snapshot(Duration::from_secs(10)).await;
    assert!(snap2.is_some(), "Second snapshot should arrive from periodic polling");
    harness.logger.step_result(true, "Second snapshot received — live streaming confirmed");

    harness.shutdown().await;
    harness.logger.finish(true);
}

// ================================================================
// bd-3i5y: E2E error handling paths
//
// Consolidated: invalid JSON, unknown method, and daemon resilience.
// ================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_daemon_error_handling() {
    let _permit = DAEMON_SEM.acquire().await.unwrap();
    let mut harness = helpers::TestHarness::spawn("test_daemon_error_handling")
        .await
        .expect("Failed to spawn daemon");

    let _ = harness.wait_for_hello(Duration::from_secs(10)).await;

    // Send invalid JSON
    harness.logger.step("Sending invalid JSON to daemon");
    harness.send_raw("this is not json").await.expect("send failed");
    harness.logger.step_result(true, "Invalid JSON sent");

    // Send malformed JSON-RPC
    harness.logger.step("Sending malformed JSON-RPC");
    harness.send_raw(r#"{"jsonrpc": "1.0"}"#).await.expect("send failed");
    harness.logger.step_result(true, "Malformed RPC sent");

    // Send unknown method
    harness.logger.step("Sending unknown RPC method");
    let req = JsonRpcRequest::new(99, "nonexistent.method", serde_json::Value::Null);
    let json = serde_json::to_string(&req).unwrap();
    harness.send_raw(&json).await.expect("send failed");
    harness.logger.step_result(true, "Unknown method sent");

    // Daemon should still be alive — wait for next snapshot
    harness.logger.step("Verifying daemon still alive after bad input");
    let snap = harness.wait_for_snapshot(Duration::from_secs(10)).await;
    let alive = snap.is_some();
    harness.logger.step_result(alive, "Daemon survived all invalid input");
    assert!(alive, "Daemon should survive invalid JSON, malformed RPC, and unknown methods");

    harness.shutdown().await;
    harness.logger.finish(true);
}

/// Verify spawning with invalid binary path gives clear error.
#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_daemon_binary() {
    let logger = TestLogger::new("test_invalid_daemon_binary");
    logger.step("Attempting to spawn non-existent daemon");

    use std::process::Stdio;
    let result = tokio::process::Command::new("non-existent-daemon-binary-xyz")
        .arg("start")
        .arg("--stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();

    let failed = result.is_err();
    logger.step_result(failed, "Spawn correctly failed for non-existent binary");
    assert!(failed, "Should fail to spawn non-existent binary");

    logger.finish(true);
}

// ================================================================
// bd-389w: E2E connection state lifecycle
// ================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_disconnection_on_daemon_kill() {
    let _permit = DAEMON_SEM.acquire().await.unwrap();
    let mut harness = helpers::TestHarness::spawn("test_disconnection_on_daemon_kill")
        .await
        .expect("Failed to spawn daemon");

    harness.logger.step("Establishing connection");
    let connected = harness
        .wait_for_connection_state(ConnState::Connected, Duration::from_secs(10))
        .await;
    assert!(connected, "Should connect first");
    harness.logger.step_result(true, "Connected");

    harness.logger.step("Killing daemon process");
    let _ = harness.child.kill().await;
    harness.logger.step_result(true, "Daemon killed");

    harness.logger.step("Waiting for Disconnected state");
    let disconnected = harness
        .wait_for_connection_state(ConnState::Disconnected, Duration::from_secs(5))
        .await;

    harness.logger.step_result(disconnected, "Disconnected state detected");
    assert!(disconnected, "Should detect disconnection after daemon dies");

    harness.logger.finish(true);
}

// ================================================================
// bd-17vb: E2E end-to-end data flow from daemon to app state
// ================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_data_flow_snapshot_to_app() {
    let _permit = DAEMON_SEM.acquire().await.unwrap();
    use ftui::Model;
    use ntm_tracker_tui::app::NtmApp;

    let mut harness = helpers::TestHarness::spawn("test_data_flow_snapshot_to_app")
        .await
        .expect("Failed to spawn daemon");

    let _ = harness.wait_for_hello(Duration::from_secs(10)).await;

    harness.logger.step("Waiting for live snapshot");
    let snap = harness.wait_for_snapshot(Duration::from_secs(10)).await;

    if let Some(s) = snap {
        harness.logger.step("Feeding snapshot into NtmApp::update()");
        let mut app = NtmApp::new();
        let cmd = app.update(Msg::SnapshotReceived(s.clone()));
        assert!(matches!(cmd, ftui::Cmd::None));

        harness.logger.step("Verifying app state matches snapshot");
        assert_eq!(app.sessions.len(), s.sessions.len(), "sessions match");
        assert_eq!(app.panes.len(), s.panes.len(), "panes match");
        assert_eq!(app.events.len(), s.events.len(), "events match");
        assert_eq!(app.stats.sessions, s.stats.summary.sessions, "stats.sessions match");
        assert_eq!(app.stats.panes, s.stats.summary.panes, "stats.panes match");
        assert_eq!(app.last_event_id, s.last_event_id, "last_event_id match");

        harness.logger.step_result(
            true,
            &format!("App state matches: {} sessions, {} panes", app.sessions.len(), app.panes.len()),
        );

        // Feed a second snapshot to verify replacement
        harness.logger.step("Waiting for second snapshot");
        if let Some(s2) = harness.wait_for_snapshot(Duration::from_secs(10)).await {
            app.update(Msg::SnapshotReceived(s2.clone()));
            assert_eq!(app.sessions.len(), s2.sessions.len());
            harness.logger.step_result(true, "Second snapshot replaced first");
        }
    }

    harness.shutdown().await;
    harness.logger.finish(true);
}

/// Verify hello -> connection state flows into app (no daemon needed).
#[test]
fn test_data_flow_hello_to_app() {
    use ftui::Model;
    use ntm_tracker_tui::app::NtmApp;

    let logger = TestLogger::new("test_data_flow_hello_to_app");

    logger.step("Creating NtmApp and simulating hello flow");
    let mut app = NtmApp::new();
    assert_eq!(app.conn_state, ConnState::Disconnected);
    assert!(app.daemon_version.is_empty());

    app.update(Msg::ConnectionChanged(ConnState::Connecting));
    assert_eq!(app.conn_state, ConnState::Connecting);
    logger.step_result(true, "Connecting state set");

    app.update(Msg::HelloReceived("0.1.0".to_string()));
    assert_eq!(app.daemon_version, "0.1.0");
    logger.step_result(true, "Version set from hello");

    app.update(Msg::ConnectionChanged(ConnState::Connected));
    assert_eq!(app.conn_state, ConnState::Connected);
    logger.step_result(true, "Connected state set");

    app.update(Msg::RpcError("timeout".to_string()));
    assert_eq!(app.conn_state, ConnState::Error("timeout".to_string()));
    logger.step_result(true, "Error state set from RpcError");

    logger.finish(true);
}

// ================================================================
// Test infrastructure (from bd-1i1d harness)
// ================================================================

#[test]
fn test_logger_structured_output() {
    let logger = TestLogger::new("test_logger");
    logger.step("First step");
    logger.step_result(true, "Completed first step");
    logger.step("Second step");
    logger.step_result(false, "Failed second step");
    logger.finish(false);

    let lines = logger.lines();
    assert!(lines.len() >= 5, "Expected at least 5 log lines, got {}", lines.len());
    assert!(lines[0].contains("=== START:"), "First line should be start marker");
    assert!(lines[1].contains("STEP 1:"), "Should have step 1");
    assert!(lines[2].contains("PASS"), "Should have pass result");
    assert!(lines[3].contains("STEP 2:"), "Should have step 2");
    assert!(lines[4].contains("FAIL"), "Should have fail result");
}

#[test]
fn test_fixture_builders() {
    let session = helpers::fixtures::session("s1", "test-project", "active");
    assert_eq!(session.session_id, "s1");
    assert_eq!(session.name, "test-project");
    assert_eq!(session.status, "active");

    let pane = helpers::fixtures::pane("p1", "s1", "active");
    assert_eq!(pane.pane_id, "p1");
    assert_eq!(pane.session_id, "s1");

    let event = helpers::fixtures::event(1, "escalation", "s1");
    assert_eq!(event.id, 1);
    assert_eq!(event.event_type, "escalation");

    let snap = helpers::fixtures::snapshot(vec![session], vec![pane], vec![event]);
    assert_eq!(snap.sessions.len(), 1);
    assert_eq!(snap.panes.len(), 1);
    assert_eq!(snap.events.len(), 1);
}

#[test]
fn test_notification_json_builders() {
    let hello = helpers::fixtures::hello_notification("1.0.0");
    let parsed: serde_json::Value = serde_json::from_str(&hello).unwrap();
    assert_eq!(parsed["method"], "core.hello");
    assert_eq!(parsed["params"]["daemonVersion"], "1.0.0");

    let snap_json = helpers::fixtures::snapshot_notification(&helpers::fixtures::snapshot(
        vec![], vec![], vec![],
    ));
    let parsed: serde_json::Value = serde_json::from_str(&snap_json).unwrap();
    assert_eq!(parsed["method"], "sessions.snapshot");

    let resp = helpers::fixtures::rpc_response(42, serde_json::json!({"ok": true}));
    let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(parsed["id"], 42);
    assert_eq!(parsed["result"]["ok"], true);

    let err = helpers::fixtures::rpc_error(1, -32600, "Invalid Request");
    let parsed: serde_json::Value = serde_json::from_str(&err).unwrap();
    assert_eq!(parsed["error"]["code"], -32600);
    assert_eq!(parsed["error"]["message"], "Invalid Request");
}
