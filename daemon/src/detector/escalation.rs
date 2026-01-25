use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct EscalationInput<'a> {
    pub now: i64,
    pub pane_uid: &'a str,
    pub line: &'a str,
    pub pane_last_activity: Option<i64>,
    pub waiting_hint: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EscalationStatus {
    Pending,
    Resolved,
    Dismissed,
}

#[derive(Clone, Debug)]
pub struct EscalationEvent {
    pub pane_uid: String,
    pub detected_at: i64,
    pub severity: String,
    pub status: EscalationStatus,
    pub message: String,
    pub confidence: f32,
}

#[derive(Clone, Debug)]
pub struct EscalationConfig {
    pub debounce_secs: i64,
    pub activity_window_secs: i64,
}

impl Default for EscalationConfig {
    fn default() -> Self {
        Self {
            debounce_secs: 30,
            activity_window_secs: 300,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct EscalationDetector {
    last_detected_at: Option<i64>,
    active: HashMap<String, EscalationEvent>,
    config: EscalationConfig,
}

impl EscalationDetector {
    pub fn new(config: EscalationConfig) -> Self {
        Self {
            last_detected_at: None,
            active: HashMap::new(),
            config,
        }
    }

    pub fn detect(&mut self, input: EscalationInput<'_>) -> Option<EscalationEvent> {
        if self.is_debounced(input.now) {
            return None;
        }

        if !self.is_recent_activity(input.now, input.pane_last_activity) {
            return None;
        }

        if !input.waiting_hint && !looks_like_prompt(input.line) {
            return None;
        }

        let lowered = input.line.to_lowercase();
        let (severity, confidence, message) = escalation_match(&lowered)?;

        let event = EscalationEvent {
            pane_uid: input.pane_uid.to_string(),
            detected_at: input.now,
            severity: severity.to_string(),
            status: EscalationStatus::Pending,
            message,
            confidence,
        };

        self.last_detected_at = Some(input.now);
        self.active.insert(input.pane_uid.to_string(), event.clone());
        Some(event)
    }

    pub fn active_for_pane(&self, pane_uid: &str) -> Option<EscalationEvent> {
        self.active.get(pane_uid).cloned()
    }

    pub fn resolve_on_activity(&mut self, pane_uid: &str, now: i64) -> Option<EscalationEvent> {
        if let Some(mut event) = self.active.remove(pane_uid) {
            if now.saturating_sub(event.detected_at) <= self.config.activity_window_secs {
                event.status = EscalationStatus::Resolved;
                return Some(event);
            }
            self.active.insert(pane_uid.to_string(), event);
        }
        None
    }

    pub fn dismiss(&mut self, pane_uid: &str) -> Option<EscalationEvent> {
        if let Some(mut event) = self.active.remove(pane_uid) {
            event.status = EscalationStatus::Dismissed;
            return Some(event);
        }
        None
    }

    fn is_debounced(&self, now: i64) -> bool {
        self.last_detected_at
            .map(|last| now.saturating_sub(last) < self.config.debounce_secs)
            .unwrap_or(false)
    }

    fn is_recent_activity(&self, now: i64, last_activity: Option<i64>) -> bool {
        last_activity
            .map(|last| now.saturating_sub(last) <= self.config.activity_window_secs)
            .unwrap_or(false)
    }
}

fn escalation_match(line: &str) -> Option<(&'static str, f32, String)> {
    if line.contains("fatal error") || line.contains("cannot proceed") {
        return Some(("error", 0.9, line.to_string()));
    }

    if line.contains("permission denied") && line.contains("continue") {
        return Some(("error", 0.85, line.to_string()));
    }

    if line.contains("need") && line.contains("human") && line.contains("input") {
        return Some(("warn", 0.75, line.to_string()));
    }

    if line.contains("escalating to user") || line.contains("requires manual intervention") {
        return Some(("warn", 0.7, line.to_string()));
    }

    if line.contains("please confirm")
        && (line.contains("delete")
            || line.contains("remove")
            || line.contains("overwrite")
            || line.contains("proceed")
            || line.contains("continue"))
    {
        return Some(("warn", 0.8, line.to_string()));
    }

    None
}

fn looks_like_prompt(line: &str) -> bool {
    let trimmed = line.trim_end();
    if trimmed.ends_with('>') || trimmed.ends_with('$') {
        return true;
    }
    let lowered = line.to_lowercase();
    if lowered.contains("(y/n)") || lowered.contains("press enter") {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_escalation_pattern() {
        let mut detector = EscalationDetector::default();
        let input = EscalationInput {
            now: 100,
            pane_uid: "pane-1",
            line: "Please confirm delete (y/n)",
            pane_last_activity: Some(95),
            waiting_hint: true,
        };
        let event = detector.detect(input).expect("detect");
        assert_eq!(event.status, EscalationStatus::Pending);
        assert_eq!(event.severity, "warn");
    }

    #[test]
    fn debounce_prevents_repeat() {
        let mut detector = EscalationDetector::default();
        let input = EscalationInput {
            now: 100,
            pane_uid: "pane-1",
            line: "fatal error",
            pane_last_activity: Some(95),
            waiting_hint: true,
        };
        assert!(detector.detect(input).is_some());
        let second = EscalationInput {
            now: 110,
            pane_uid: "pane-1",
            line: "fatal error",
            pane_last_activity: Some(95),
            waiting_hint: true,
        };
        assert!(detector.detect(second).is_none());
    }

    #[test]
    fn resolves_on_activity() {
        let mut detector = EscalationDetector::default();
        let input = EscalationInput {
            now: 100,
            pane_uid: "pane-2",
            line: "need human input (y/n)",
            pane_last_activity: Some(99),
            waiting_hint: true,
        };
        assert!(detector.detect(input).is_some());
        let resolved = detector.resolve_on_activity("pane-2", 120).expect("resolved");
        assert_eq!(resolved.status, EscalationStatus::Resolved);
    }

    #[test]
    fn detects_fatal_error() {
        let mut detector = EscalationDetector::default();
        let input = EscalationInput {
            now: 100,
            pane_uid: "pane-1",
            line: "FATAL ERROR: cannot continue",
            pane_last_activity: Some(99),
            waiting_hint: true,
        };
        let event = detector.detect(input).expect("detect");
        assert_eq!(event.severity, "error");
        assert!(event.confidence >= 0.9);
    }

    #[test]
    fn detects_cannot_proceed() {
        let mut detector = EscalationDetector::default();
        let input = EscalationInput {
            now: 100,
            pane_uid: "pane-1",
            line: "cannot proceed without authorization",
            pane_last_activity: Some(99),
            waiting_hint: true,
        };
        let event = detector.detect(input).expect("detect");
        assert_eq!(event.severity, "error");
    }

    #[test]
    fn detects_permission_denied_continue() {
        let mut detector = EscalationDetector::default();
        let input = EscalationInput {
            now: 100,
            pane_uid: "pane-1",
            line: "Permission denied. Press enter to continue or abort.",
            pane_last_activity: Some(99),
            waiting_hint: true,
        };
        let event = detector.detect(input).expect("detect");
        assert_eq!(event.severity, "error");
    }

    #[test]
    fn detects_manual_intervention() {
        let mut detector = EscalationDetector::default();
        let input = EscalationInput {
            now: 100,
            pane_uid: "pane-1",
            line: "This requires manual intervention from the user",
            pane_last_activity: Some(99),
            waiting_hint: true,
        };
        let event = detector.detect(input).expect("detect");
        assert_eq!(event.severity, "warn");
    }

    #[test]
    fn detects_escalating_to_user() {
        let mut detector = EscalationDetector::default();
        let input = EscalationInput {
            now: 100,
            pane_uid: "pane-1",
            line: "Escalating to user for decision",
            pane_last_activity: Some(99),
            waiting_hint: true,
        };
        let event = detector.detect(input).expect("detect");
        assert_eq!(event.severity, "warn");
    }

    #[test]
    fn detects_confirm_remove() {
        let mut detector = EscalationDetector::default();
        let input = EscalationInput {
            now: 100,
            pane_uid: "pane-1",
            line: "please confirm remove operation >",
            pane_last_activity: Some(99),
            waiting_hint: true,
        };
        let event = detector.detect(input).expect("detect");
        assert_eq!(event.severity, "warn");
        assert!(event.confidence >= 0.8);
    }

    #[test]
    fn detects_confirm_overwrite() {
        let mut detector = EscalationDetector::default();
        let input = EscalationInput {
            now: 100,
            pane_uid: "pane-1",
            line: "please confirm overwrite $",
            pane_last_activity: Some(99),
            waiting_hint: true,
        };
        let event = detector.detect(input).expect("detect");
        assert_eq!(event.severity, "warn");
    }

    #[test]
    fn ignores_stale_activity() {
        let config = EscalationConfig {
            debounce_secs: 30,
            activity_window_secs: 60,
        };
        let mut detector = EscalationDetector::new(config);
        // Last activity was too long ago
        let input = EscalationInput {
            now: 200,
            pane_uid: "pane-1",
            line: "fatal error",
            pane_last_activity: Some(100), // 100 seconds ago, outside 60s window
            waiting_hint: true,
        };
        assert!(detector.detect(input).is_none());
    }

    #[test]
    fn ignores_non_prompt_without_waiting_hint() {
        let mut detector = EscalationDetector::default();
        // Line doesn't look like a prompt and waiting_hint is false
        let input = EscalationInput {
            now: 100,
            pane_uid: "pane-1",
            line: "fatal error occurred during processing",
            pane_last_activity: Some(99),
            waiting_hint: false,
        };
        assert!(detector.detect(input).is_none());
    }

    #[test]
    fn detects_prompt_ending_with_chevron() {
        let mut detector = EscalationDetector::default();
        let input = EscalationInput {
            now: 100,
            pane_uid: "pane-1",
            line: "fatal error - what do you want to do >",
            pane_last_activity: Some(99),
            waiting_hint: false, // No hint but line ends with >
        };
        let event = detector.detect(input).expect("detect");
        assert_eq!(event.severity, "error");
    }

    #[test]
    fn detects_prompt_ending_with_dollar() {
        let mut detector = EscalationDetector::default();
        let input = EscalationInput {
            now: 100,
            pane_uid: "pane-1",
            line: "need human input here $",
            pane_last_activity: Some(99),
            waiting_hint: false,
        };
        let event = detector.detect(input).expect("detect");
        assert_eq!(event.severity, "warn");
    }

    #[test]
    fn detects_y_n_prompt() {
        let mut detector = EscalationDetector::default();
        let input = EscalationInput {
            now: 100,
            pane_uid: "pane-1",
            line: "Are you sure you want to proceed? (Y/N)",
            pane_last_activity: Some(99),
            waiting_hint: false,
        };
        // This should match because of (y/n) pattern in looks_like_prompt
        // But it needs an escalation pattern too
        // Without escalation pattern, it won't detect
        let result = detector.detect(input);
        // This line has (Y/N) which looks_like_prompt but no escalation keywords
        assert!(result.is_none());
    }

    #[test]
    fn dismiss_removes_active_escalation() {
        let mut detector = EscalationDetector::default();
        let input = EscalationInput {
            now: 100,
            pane_uid: "pane-3",
            line: "fatal error",
            pane_last_activity: Some(99),
            waiting_hint: true,
        };
        assert!(detector.detect(input).is_some());
        assert!(detector.active_for_pane("pane-3").is_some());

        let dismissed = detector.dismiss("pane-3").expect("dismissed");
        assert_eq!(dismissed.status, EscalationStatus::Dismissed);
        assert!(detector.active_for_pane("pane-3").is_none());
    }

    #[test]
    fn dismiss_returns_none_for_unknown_pane() {
        let mut detector = EscalationDetector::default();
        assert!(detector.dismiss("unknown-pane").is_none());
    }

    #[test]
    fn resolve_outside_window_keeps_active() {
        let config = EscalationConfig {
            debounce_secs: 30,
            activity_window_secs: 60,
        };
        let mut detector = EscalationDetector::new(config);
        let input = EscalationInput {
            now: 100,
            pane_uid: "pane-4",
            line: "fatal error",
            pane_last_activity: Some(99),
            waiting_hint: true,
        };
        assert!(detector.detect(input).is_some());

        // Try to resolve outside the activity window
        let result = detector.resolve_on_activity("pane-4", 500);
        assert!(result.is_none());
        // Escalation should still be active
        assert!(detector.active_for_pane("pane-4").is_some());
    }

    #[test]
    fn active_for_pane_returns_none_for_unknown() {
        let detector = EscalationDetector::default();
        assert!(detector.active_for_pane("unknown").is_none());
    }

    #[test]
    fn multiple_panes_tracked_independently() {
        let mut detector = EscalationDetector::default();

        let input1 = EscalationInput {
            now: 100,
            pane_uid: "pane-a",
            line: "fatal error",
            pane_last_activity: Some(99),
            waiting_hint: true,
        };
        assert!(detector.detect(input1).is_some());

        let input2 = EscalationInput {
            now: 150, // After debounce for different pane
            pane_uid: "pane-b",
            line: "cannot proceed",
            pane_last_activity: Some(149),
            waiting_hint: true,
        };
        assert!(detector.detect(input2).is_some());

        assert!(detector.active_for_pane("pane-a").is_some());
        assert!(detector.active_for_pane("pane-b").is_some());

        // Resolve pane-a
        assert!(detector.resolve_on_activity("pane-a", 150).is_some());
        assert!(detector.active_for_pane("pane-a").is_none());
        assert!(detector.active_for_pane("pane-b").is_some());
    }

    #[test]
    fn requires_recent_activity() {
        let mut detector = EscalationDetector::default();
        let input = EscalationInput {
            now: 100,
            pane_uid: "pane-1",
            line: "fatal error",
            pane_last_activity: None, // No activity timestamp
            waiting_hint: true,
        };
        assert!(detector.detect(input).is_none());
    }
}
