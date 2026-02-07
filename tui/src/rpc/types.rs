use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Full snapshot from `snapshot.get` or `sessions.snapshot` notification.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    #[serde(default)]
    pub sessions: Vec<SessionView>,
    #[serde(default)]
    pub panes: Vec<PaneView>,
    #[serde(default)]
    pub events: Vec<EventView>,
    #[serde(default)]
    pub stats: StatsEnvelope,
    #[serde(default)]
    pub last_event_id: i64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsEnvelope {
    #[serde(default)]
    pub summary: StatsSummary,
    #[serde(default)]
    pub hourly: Vec<Value>,
    #[serde(default)]
    pub daily: Vec<Value>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsSummary {
    #[serde(default)]
    pub sessions: usize,
    #[serde(default)]
    pub panes: usize,
    #[serde(default)]
    pub total_compacts: u64,
    #[serde(default)]
    pub active_minutes: u64,
    #[serde(default)]
    pub estimated_tokens: u64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub status: String,
    pub status_reason: Option<String>,
    #[serde(default)]
    pub pane_count: u32,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub last_seen_at: i64,
    pub ended_at: Option<i64>,
    pub tmux_session_id: Option<String>,
    #[serde(default)]
    pub source_id: String,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaneView {
    #[serde(default)]
    pub pane_id: String,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub status: String,
    pub status_reason: Option<String>,
    #[serde(default)]
    pub pane_index: i32,
    pub agent_type: Option<String>,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub last_seen_at: i64,
    pub last_activity_at: Option<i64>,
    pub current_command: Option<String>,
    pub ended_at: Option<i64>,
    pub tmux_pane_id: Option<String>,
    pub tmux_window_id: Option<String>,
    pub tmux_pane_pid: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventView {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub event_type: String,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub pane_id: String,
    #[serde(default)]
    pub detected_at: i64,
    pub severity: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EscalationView {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub pane_id: String,
    #[serde(default)]
    pub detected_at: i64,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthData {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub uptime: u64,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub instance_id: String,
    #[serde(default)]
    pub run_id: String,
    #[serde(default)]
    pub schema_version: String,
    #[serde(default)]
    pub protocol_version: String,
    pub last_error: Option<String>,
    #[serde(default)]
    pub last_event_id: i64,
}

/// A JSON-RPC 2.0 request we send.
#[derive(Debug, serde::Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: &'static str,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Value::is_null")]
    pub params: Value,
}

impl JsonRpcRequest {
    pub fn new(id: u64, method: impl Into<String>, params: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            method: method.into(),
            params,
        }
    }
}

/// A JSON-RPC 2.0 response/notification we receive.
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcMessage {
    #[serde(default)]
    pub jsonrpc: String,
    /// Present for responses, absent for notifications.
    pub id: Option<Value>,
    /// Present for notifications.
    pub method: Option<String>,
    /// Success result.
    pub result: Option<Value>,
    /// Error result.
    pub error: Option<JsonRpcError>,
    /// Notification params.
    pub params: Option<Value>,
}

impl JsonRpcMessage {
    pub fn is_notification(&self) -> bool {
        self.id.is_none() && self.method.is_some()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_view_deserialize() {
        let json = r#"{
            "sessionId": "sess-1",
            "name": "my-project",
            "status": "active",
            "paneCount": 3,
            "createdAt": 1700000000,
            "lastSeenAt": 1700001000,
            "sourceId": "tmux"
        }"#;
        let sv: SessionView = serde_json::from_str(json).unwrap();
        assert_eq!(sv.session_id, "sess-1");
        assert_eq!(sv.name, "my-project");
        assert_eq!(sv.status, "active");
        assert_eq!(sv.pane_count, 3);
        assert_eq!(sv.source_id, "tmux");
    }

    #[test]
    fn test_session_view_minimal() {
        let json = r#"{}"#;
        let sv: SessionView = serde_json::from_str(json).unwrap();
        assert_eq!(sv.session_id, "");
        assert_eq!(sv.pane_count, 0);
        assert_eq!(sv.status_reason, None);
        assert_eq!(sv.ended_at, None);
    }

    #[test]
    fn test_pane_view_deserialize() {
        let json = r#"{
            "paneId": "pane-1",
            "sessionId": "sess-1",
            "status": "active",
            "paneIndex": 0,
            "agentType": "claude-code",
            "createdAt": 1700000000
        }"#;
        let pv: PaneView = serde_json::from_str(json).unwrap();
        assert_eq!(pv.pane_id, "pane-1");
        assert_eq!(pv.session_id, "sess-1");
        assert_eq!(pv.agent_type, Some("claude-code".to_string()));
    }

    #[test]
    fn test_pane_view_optional_fields() {
        let json = r#"{"paneId": "p1"}"#;
        let pv: PaneView = serde_json::from_str(json).unwrap();
        assert_eq!(pv.agent_type, None);
        assert_eq!(pv.current_command, None);
        assert_eq!(pv.ended_at, None);
        assert_eq!(pv.tmux_pane_id, None);
    }

    #[test]
    fn test_event_view_deserialize() {
        let json = r#"{
            "id": 42,
            "eventType": "escalation",
            "sessionId": "sess-1",
            "paneId": "pane-0",
            "detectedAt": 1700000500,
            "severity": "high",
            "status": "pending"
        }"#;
        let ev: EventView = serde_json::from_str(json).unwrap();
        assert_eq!(ev.id, 42);
        assert_eq!(ev.event_type, "escalation");
        assert_eq!(ev.detected_at, 1700000500);
    }

    #[test]
    fn test_stats_summary_default() {
        let s = StatsSummary::default();
        assert_eq!(s.sessions, 0);
        assert_eq!(s.panes, 0);
        assert_eq!(s.total_compacts, 0);
        assert_eq!(s.active_minutes, 0);
        assert_eq!(s.estimated_tokens, 0);
    }

    #[test]
    fn test_stats_summary_deserialize() {
        let json = r#"{
            "sessions": 3,
            "panes": 7,
            "totalCompacts": 5,
            "activeMinutes": 120,
            "estimatedTokens": 50000
        }"#;
        let s: StatsSummary = serde_json::from_str(json).unwrap();
        assert_eq!(s.sessions, 3);
        assert_eq!(s.panes, 7);
        assert_eq!(s.total_compacts, 5);
        assert_eq!(s.active_minutes, 120);
        assert_eq!(s.estimated_tokens, 50000);
    }

    #[test]
    fn test_snapshot_deserialize() {
        let json = r#"{
            "sessions": [{"sessionId": "s1", "name": "test", "status": "active"}],
            "panes": [{"paneId": "p1", "sessionId": "s1"}],
            "events": [],
            "stats": {"summary": {"sessions": 1}},
            "lastEventId": 99
        }"#;
        let snap: Snapshot = serde_json::from_str(json).unwrap();
        assert_eq!(snap.sessions.len(), 1);
        assert_eq!(snap.panes.len(), 1);
        assert_eq!(snap.events.len(), 0);
        assert_eq!(snap.stats.summary.sessions, 1);
        assert_eq!(snap.last_event_id, 99);
    }

    #[test]
    fn test_snapshot_empty_arrays() {
        let json = r#"{}"#;
        let snap: Snapshot = serde_json::from_str(json).unwrap();
        assert!(snap.sessions.is_empty());
        assert!(snap.panes.is_empty());
        assert!(snap.events.is_empty());
        assert_eq!(snap.last_event_id, 0);
    }

    #[test]
    fn test_jsonrpc_message_is_notification() {
        let json = r#"{"method": "core.hello", "params": {}}"#;
        let msg: JsonRpcMessage = serde_json::from_str(json).unwrap();
        assert!(msg.is_notification());
    }

    #[test]
    fn test_jsonrpc_message_is_response() {
        let json = r#"{"id": 1, "result": null}"#;
        let msg: JsonRpcMessage = serde_json::from_str(json).unwrap();
        assert!(!msg.is_notification());
    }

    #[test]
    fn test_jsonrpc_request_serialization() {
        let req = JsonRpcRequest::new(1, "snapshot.get", Value::Null);
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"method\":\"snapshot.get\""));
        // params is null and skip_serializing_if, so should not appear
        assert!(!json.contains("\"params\""));
    }

    #[test]
    fn test_camel_case_mapping() {
        let json = r#"{"sessionId": "x", "paneCount": 5, "sourceId": "tmux", "lastSeenAt": 100}"#;
        let sv: SessionView = serde_json::from_str(json).unwrap();
        assert_eq!(sv.session_id, "x");
        assert_eq!(sv.pane_count, 5);
        assert_eq!(sv.source_id, "tmux");
        assert_eq!(sv.last_seen_at, 100);
    }
}
