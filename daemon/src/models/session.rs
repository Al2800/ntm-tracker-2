use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SessionStatus {
    Unknown,
    Active,
    Idle,
    Ended,
}

impl SessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionStatus::Unknown => "unknown",
            SessionStatus::Active => "active",
            SessionStatus::Idle => "idle",
            SessionStatus::Ended => "ended",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub session_uid: String,
    pub source_id: String,
    pub tmux_session_id: Option<String>,
    pub name: String,
    pub created_at: i64,
    pub last_seen_at: i64,
    pub ended_at: Option<i64>,
    pub status: SessionStatus,
    pub status_reason: Option<String>,
    pub pane_count: u32,
    pub metadata: Option<serde_json::Value>,
}

impl Session {
    pub fn new(
        source_id: impl Into<String>,
        name: impl Into<String>,
        tmux_session_id: Option<String>,
        now: i64,
    ) -> Self {
        Self {
            session_uid: Uuid::now_v7().to_string(),
            source_id: source_id.into(),
            tmux_session_id,
            name: name.into(),
            created_at: now,
            last_seen_at: now,
            ended_at: None,
            status: SessionStatus::Unknown,
            status_reason: None,
            pane_count: 0,
            metadata: None,
        }
    }
}
