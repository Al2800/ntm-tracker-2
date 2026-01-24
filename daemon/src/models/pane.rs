use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PaneStatus {
    Unknown,
    Active,
    Waiting,
    Idle,
    Ended,
}

impl PaneStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PaneStatus::Unknown => "unknown",
            PaneStatus::Active => "active",
            PaneStatus::Waiting => "waiting",
            PaneStatus::Idle => "idle",
            PaneStatus::Ended => "ended",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pane {
    pub pane_uid: String,
    pub session_uid: String,
    pub tmux_pane_id: Option<String>,
    pub tmux_window_id: Option<String>,
    pub tmux_pane_pid: Option<i64>,
    pub pane_index: i32,
    pub agent_type: Option<String>,
    pub created_at: i64,
    pub last_seen_at: i64,
    pub last_activity_at: Option<i64>,
    pub current_command: Option<String>,
    pub ended_at: Option<i64>,
    pub status: PaneStatus,
    pub status_reason: Option<String>,
}

impl Pane {
    pub fn new(
        session_uid: impl Into<String>,
        pane_index: i32,
        now: i64,
        tmux_pane_id: Option<String>,
        tmux_window_id: Option<String>,
        tmux_pane_pid: Option<i64>,
    ) -> Self {
        Self {
            pane_uid: Uuid::now_v7().to_string(),
            session_uid: session_uid.into(),
            tmux_pane_id,
            tmux_window_id,
            tmux_pane_pid,
            pane_index,
            agent_type: None,
            created_at: now,
            last_seen_at: now,
            last_activity_at: None,
            current_command: None,
            ended_at: None,
            status: PaneStatus::Unknown,
            status_reason: None,
        }
    }
}
