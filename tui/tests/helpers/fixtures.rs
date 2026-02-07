use ntm_tracker_tui::rpc::types::*;

/// Build a SessionView with required fields.
pub fn session(id: &str, name: &str, status: &str) -> SessionView {
    SessionView {
        session_id: id.to_string(),
        name: name.to_string(),
        status: status.to_string(),
        pane_count: 0,
        source_id: "tmux".to_string(),
        created_at: 1700000000,
        last_seen_at: 1700000000,
        ..Default::default()
    }
}

/// Build a PaneView with required fields.
pub fn pane(pane_id: &str, session_id: &str, status: &str) -> PaneView {
    PaneView {
        pane_id: pane_id.to_string(),
        session_id: session_id.to_string(),
        status: status.to_string(),
        created_at: 1700000000,
        last_seen_at: 1700000000,
        ..Default::default()
    }
}

/// Build an EventView with required fields.
pub fn event(id: i64, event_type: &str, session_id: &str) -> EventView {
    EventView {
        id,
        event_type: event_type.to_string(),
        session_id: session_id.to_string(),
        pane_id: "pane-0".to_string(),
        detected_at: 1700000000 + id,
        severity: None,
        status: None,
    }
}

/// Build a Snapshot with the given components.
pub fn snapshot(
    sessions: Vec<SessionView>,
    panes: Vec<PaneView>,
    events: Vec<EventView>,
) -> Snapshot {
    Snapshot {
        sessions,
        panes,
        events,
        stats: StatsEnvelope::default(),
        last_event_id: 0,
    }
}

/// Build a minimal core.hello JSON-RPC notification string.
pub fn hello_notification(version: &str) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "core.hello",
        "params": {
            "daemonVersion": version,
            "protocolVersion": 1,
            "schemaVersion": 1,
            "instanceId": "test-instance",
            "runId": "test-run",
            "capabilities": {}
        }
    })
    .to_string()
}

/// Build a sessions.snapshot JSON-RPC notification string.
pub fn snapshot_notification(snap: &Snapshot) -> String {
    let params = serde_json::to_value(snap).unwrap_or_default();
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "sessions.snapshot",
        "params": params
    })
    .to_string()
}

/// Build a JSON-RPC response string.
pub fn rpc_response(id: u64, result: serde_json::Value) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
    .to_string()
}

/// Build a JSON-RPC error response string.
pub fn rpc_error(id: u64, code: i64, message: &str) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
    .to_string()
}
