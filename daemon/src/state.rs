use crate::models::pane::{Pane, PaneStatus};
use crate::models::session::{Session, SessionStatus};

#[derive(Clone, Copy, Debug)]
pub struct StateConfig {
    pub idle_threshold_secs: i64,
}

impl Default for StateConfig {
    fn default() -> Self {
        Self {
            idle_threshold_secs: 300,
        }
    }
}

pub fn update_session_status(
    session: &mut Session,
    now: i64,
    config: StateConfig,
) -> bool {
    let new_status = if session.ended_at.is_some() {
        SessionStatus::Ended
    } else if now.saturating_sub(session.last_seen_at) <= config.idle_threshold_secs {
        SessionStatus::Active
    } else {
        SessionStatus::Idle
    };

    let new_reason = match new_status {
        SessionStatus::Ended => Some("ended".to_string()),
        SessionStatus::Active => Some("recent_activity".to_string()),
        SessionStatus::Idle => Some("idle_timeout".to_string()),
        SessionStatus::Unknown => None,
    };

    let changed = session.status != new_status || session.status_reason != new_reason;
    session.status = new_status;
    session.status_reason = new_reason;
    changed
}

pub fn update_pane_status(
    pane: &mut Pane,
    now: i64,
    config: StateConfig,
    waiting: bool,
) -> bool {
    let new_status = if pane.ended_at.is_some() {
        PaneStatus::Ended
    } else if waiting {
        PaneStatus::Waiting
    } else if pane
        .last_activity_at
        .map(|last| now.saturating_sub(last) <= config.idle_threshold_secs)
        .unwrap_or(false)
    {
        PaneStatus::Active
    } else {
        PaneStatus::Idle
    };

    let new_reason = match new_status {
        PaneStatus::Ended => Some("ended".to_string()),
        PaneStatus::Waiting => Some("waiting_prompt".to_string()),
        PaneStatus::Active => Some("recent_activity".to_string()),
        PaneStatus::Idle => Some("idle_timeout".to_string()),
        PaneStatus::Unknown => None,
    };

    let changed = pane.status != new_status || pane.status_reason != new_reason;
    pane.status = new_status;
    pane.status_reason = new_reason;
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_transitions_to_idle_after_threshold() {
        let config = StateConfig {
            idle_threshold_secs: 10,
        };
        let mut session = Session::new("source", "name", None, 0);
        session.last_seen_at = 0;
        let changed = update_session_status(&mut session, 15, config);
        assert!(changed);
        assert_eq!(session.status, SessionStatus::Idle);
        assert_eq!(session.status_reason.as_deref(), Some("idle_timeout"));
    }

    #[test]
    fn pane_waiting_overrides_activity() {
        let config = StateConfig {
            idle_threshold_secs: 10,
        };
        let mut pane = Pane::new("sess", 0, 0, None, None, None);
        pane.last_activity_at = Some(5);
        let changed = update_pane_status(&mut pane, 6, config, true);
        assert!(changed);
        assert_eq!(pane.status, PaneStatus::Waiting);
        assert_eq!(pane.status_reason.as_deref(), Some("waiting_prompt"));
    }

    #[test]
    fn pane_transitions_to_ended() {
        let config = StateConfig::default();
        let mut pane = Pane::new("sess", 0, 0, None, None, None);
        pane.ended_at = Some(1);
        let changed = update_pane_status(&mut pane, 2, config, false);
        assert!(changed);
        assert_eq!(pane.status, PaneStatus::Ended);
        assert_eq!(pane.status_reason.as_deref(), Some("ended"));
    }

    #[test]
    fn session_transitions_to_active_with_recent_activity() {
        let config = StateConfig {
            idle_threshold_secs: 10,
        };
        let mut session = Session::new("source", "name", None, 0);
        session.last_seen_at = 95;
        let changed = update_session_status(&mut session, 100, config);
        assert!(changed);
        assert_eq!(session.status, SessionStatus::Active);
        assert_eq!(session.status_reason.as_deref(), Some("recent_activity"));
    }

    #[test]
    fn session_ended_takes_priority() {
        let config = StateConfig {
            idle_threshold_secs: 10,
        };
        let mut session = Session::new("source", "name", None, 0);
        session.last_seen_at = 99; // Recent activity
        session.ended_at = Some(100);
        let changed = update_session_status(&mut session, 100, config);
        assert!(changed);
        assert_eq!(session.status, SessionStatus::Ended);
        assert_eq!(session.status_reason.as_deref(), Some("ended"));
    }

    #[test]
    fn session_no_change_returns_false() {
        let config = StateConfig {
            idle_threshold_secs: 10,
        };
        let mut session = Session::new("source", "name", None, 0);
        session.last_seen_at = 95;
        session.status = SessionStatus::Active;
        session.status_reason = Some("recent_activity".to_string());
        let changed = update_session_status(&mut session, 100, config);
        assert!(!changed);
    }

    #[test]
    fn pane_active_with_recent_activity() {
        let config = StateConfig {
            idle_threshold_secs: 10,
        };
        let mut pane = Pane::new("sess", 0, 0, None, None, None);
        pane.last_activity_at = Some(95);
        let changed = update_pane_status(&mut pane, 100, config, false);
        assert!(changed);
        assert_eq!(pane.status, PaneStatus::Active);
        assert_eq!(pane.status_reason.as_deref(), Some("recent_activity"));
    }

    #[test]
    fn pane_idle_without_recent_activity() {
        let config = StateConfig {
            idle_threshold_secs: 10,
        };
        let mut pane = Pane::new("sess", 0, 0, None, None, None);
        pane.last_activity_at = Some(50);
        let changed = update_pane_status(&mut pane, 100, config, false);
        assert!(changed);
        assert_eq!(pane.status, PaneStatus::Idle);
        assert_eq!(pane.status_reason.as_deref(), Some("idle_timeout"));
    }

    #[test]
    fn pane_idle_without_activity_timestamp() {
        let config = StateConfig {
            idle_threshold_secs: 10,
        };
        let mut pane = Pane::new("sess", 0, 0, None, None, None);
        pane.last_activity_at = None;
        let changed = update_pane_status(&mut pane, 100, config, false);
        assert!(changed);
        assert_eq!(pane.status, PaneStatus::Idle);
    }

    #[test]
    fn pane_ended_takes_priority_over_waiting() {
        let config = StateConfig {
            idle_threshold_secs: 10,
        };
        let mut pane = Pane::new("sess", 0, 0, None, None, None);
        pane.last_activity_at = Some(99);
        pane.ended_at = Some(100);
        let changed = update_pane_status(&mut pane, 100, config, true); // waiting=true
        assert!(changed);
        assert_eq!(pane.status, PaneStatus::Ended);
        assert_eq!(pane.status_reason.as_deref(), Some("ended"));
    }

    #[test]
    fn pane_no_change_returns_false() {
        let config = StateConfig {
            idle_threshold_secs: 10,
        };
        let mut pane = Pane::new("sess", 0, 0, None, None, None);
        pane.last_activity_at = Some(99);
        pane.status = PaneStatus::Active;
        pane.status_reason = Some("recent_activity".to_string());
        let changed = update_pane_status(&mut pane, 100, config, false);
        assert!(!changed);
    }

    #[test]
    fn default_config() {
        let config = StateConfig::default();
        assert_eq!(config.idle_threshold_secs, 300);
    }

    #[test]
    fn session_exactly_at_threshold_is_active() {
        let config = StateConfig {
            idle_threshold_secs: 10,
        };
        let mut session = Session::new("source", "name", None, 0);
        session.last_seen_at = 90;
        let changed = update_session_status(&mut session, 100, config);
        assert!(changed);
        assert_eq!(session.status, SessionStatus::Active);
    }

    #[test]
    fn session_one_past_threshold_is_idle() {
        let config = StateConfig {
            idle_threshold_secs: 10,
        };
        let mut session = Session::new("source", "name", None, 0);
        session.last_seen_at = 89;
        let changed = update_session_status(&mut session, 100, config);
        assert!(changed);
        assert_eq!(session.status, SessionStatus::Idle);
    }

    #[test]
    fn pane_exactly_at_threshold_is_active() {
        let config = StateConfig {
            idle_threshold_secs: 10,
        };
        let mut pane = Pane::new("sess", 0, 0, None, None, None);
        pane.last_activity_at = Some(90);
        let changed = update_pane_status(&mut pane, 100, config, false);
        assert!(changed);
        assert_eq!(pane.status, PaneStatus::Active);
    }
}
