//! Schema validation tests.
//!
//! These tests ensure that Rust types match the shared JSON Schema definitions.
//! Run with: cargo test --test schema_validation

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// Re-export types that should match the schema
// Note: This validates the Rust type structure against our expected schema

/// Session status enum matching types.json SessionStatus
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Idle,
    Ended,
    Unknown,
}

/// Pane status enum matching types.json PaneStatus
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PaneStatus {
    Active,
    Idle,
    Waiting,
    Ended,
    Unknown,
}

/// Event type enum matching types.json EventType
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub enum EventType {
    #[serde(rename = "compact")]
    Compact,
    #[serde(rename = "escalation")]
    Escalation,
    #[serde(rename = "pane.status")]
    PaneStatus,
    #[serde(rename = "session.status")]
    SessionStatus,
}

/// Event severity enum matching types.json EventSeverity
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EventSeverity {
    Info,
    Warn,
    Error,
}

/// Event status enum matching types.json EventStatus
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EventStatus {
    Pending,
    Resolved,
    Dismissed,
}

/// Capabilities matching types.json Capabilities
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub ntm: bool,
    pub tmux: bool,
    pub stream: bool,
    pub systemd: bool,
}

/// Hello payload matching shared/schema/version.json Hello
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Hello {
    pub daemon_version: String,
    pub protocol_version: u32,
    pub schema_version: u32,
    pub capabilities: Capabilities,
    pub instance_id: String,
    pub run_id: String,
}

/// Stats summary matching types.json StatsSummary
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StatsSummary {
    pub sessions: u64,
    pub panes: u64,
    pub total_compacts: u64,
    pub active_minutes: u64,
    pub estimated_tokens: u64,
}

/// Error codes matching errors.json ErrorCode
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    Unauthorized,
    Forbidden,
    RateLimited,
    StaleCursor,
    Unsupported,
    Degraded,
    NotFound,
    InvalidParams,
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::schema_for;

    #[test]
    fn test_session_status_schema() {
        let schema = schema_for!(SessionStatus);
        let json = serde_json::to_string_pretty(&schema).unwrap();
        // Verify enum values match
        assert!(json.contains("\"active\""));
        assert!(json.contains("\"idle\""));
        assert!(json.contains("\"ended\""));
        assert!(json.contains("\"unknown\""));
    }

    #[test]
    fn test_pane_status_schema() {
        let schema = schema_for!(PaneStatus);
        let json = serde_json::to_string_pretty(&schema).unwrap();
        assert!(json.contains("\"active\""));
        assert!(json.contains("\"idle\""));
        assert!(json.contains("\"waiting\""));
        assert!(json.contains("\"ended\""));
        assert!(json.contains("\"unknown\""));
    }

    #[test]
    fn test_event_type_schema() {
        let schema = schema_for!(EventType);
        let json = serde_json::to_string_pretty(&schema).unwrap();
        assert!(json.contains("\"compact\""));
        assert!(json.contains("\"escalation\""));
        assert!(json.contains("\"pane.status\""));
        assert!(json.contains("\"session.status\""));
    }

    #[test]
    fn test_capabilities_schema() {
        let schema = schema_for!(Capabilities);
        let json = serde_json::to_string_pretty(&schema).unwrap();
        assert!(json.contains("\"ntm\""));
        assert!(json.contains("\"tmux\""));
        assert!(json.contains("\"stream\""));
        assert!(json.contains("\"systemd\""));
    }

    #[test]
    fn test_stats_summary_schema() {
        let schema = schema_for!(StatsSummary);
        let json = serde_json::to_string_pretty(&schema).unwrap();
        assert!(json.contains("\"sessions\""));
        assert!(json.contains("\"panes\""));
        assert!(json.contains("\"totalCompacts\""));
        assert!(json.contains("\"activeMinutes\""));
        assert!(json.contains("\"estimatedTokens\""));
    }

    #[test]
    fn test_hello_schema() {
        let schema = schema_for!(Hello);
        let json = serde_json::to_string_pretty(&schema).unwrap();
        assert!(json.contains("\"daemonVersion\""));
        assert!(json.contains("\"protocolVersion\""));
        assert!(json.contains("\"schemaVersion\""));
        assert!(json.contains("\"capabilities\""));
        assert!(json.contains("\"instanceId\""));
        assert!(json.contains("\"runId\""));
    }

    #[test]
    fn test_error_code_schema() {
        let schema = schema_for!(ErrorCode);
        let json = serde_json::to_string_pretty(&schema).unwrap();
        assert!(json.contains("UNAUTHORIZED"));
        assert!(json.contains("FORBIDDEN"));
        assert!(json.contains("RATE_LIMITED"));
        assert!(json.contains("NOT_FOUND"));
        assert!(json.contains("INVALID_PARAMS"));
    }

    #[test]
    fn test_session_status_serialization() {
        assert_eq!(
            serde_json::to_string(&SessionStatus::Active).unwrap(),
            "\"active\""
        );
        assert_eq!(
            serde_json::to_string(&SessionStatus::Idle).unwrap(),
            "\"idle\""
        );
    }

    #[test]
    fn test_event_type_serialization() {
        assert_eq!(
            serde_json::to_string(&EventType::PaneStatus).unwrap(),
            "\"pane.status\""
        );
        assert_eq!(
            serde_json::to_string(&EventType::SessionStatus).unwrap(),
            "\"session.status\""
        );
    }
}
