use crate::models::pane::PaneStatus;

#[derive(Clone, Debug)]
pub struct StatusInput<'a> {
    pub now: i64,
    pub pane_last_activity: Option<i64>,
    pub pane_dead: bool,
    pub pane_current_command: Option<&'a str>,
    pub output_line: Option<&'a str>,
}

#[derive(Clone, Debug)]
pub struct StatusConfig {
    pub idle_threshold_secs: i64,
}

impl Default for StatusConfig {
    fn default() -> Self {
        Self {
            idle_threshold_secs: 300,
        }
    }
}

#[derive(Clone, Debug)]
pub struct StatusResult {
    pub status: PaneStatus,
    pub reason: String,
}

pub fn detect_status(input: StatusInput<'_>, config: StatusConfig) -> StatusResult {
    if input.pane_dead {
        return StatusResult {
            status: PaneStatus::Ended,
            reason: "pane_dead".to_string(),
        };
    }

    let recent_activity = input
        .pane_last_activity
        .map(|last| input.now.saturating_sub(last) <= config.idle_threshold_secs)
        .unwrap_or(false);

    let output = input.output_line.map(strip_ansi);
    if recent_activity && output.as_deref().map(is_waiting_pattern).unwrap_or(false) {
        return StatusResult {
            status: PaneStatus::Waiting,
            reason: "waiting_pattern".to_string(),
        };
    }

    if output.as_deref().map(is_active_pattern).unwrap_or(false) {
        return StatusResult {
            status: PaneStatus::Active,
            reason: "active_pattern".to_string(),
        };
    }

    if !recent_activity {
        return StatusResult {
            status: PaneStatus::Idle,
            reason: "idle_timeout".to_string(),
        };
    }

    let command_hint = input
        .pane_current_command
        .unwrap_or("unknown")
        .to_string();

    StatusResult {
        status: PaneStatus::Active,
        reason: format!("recent_activity:{command_hint}"),
    }
}

fn strip_ansi(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if matches!(chars.peek(), Some('[')) {
                chars.next();
                while let Some(next) = chars.next() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
                continue;
            }
        }
        output.push(ch);
    }
    output
}

fn is_waiting_pattern(input: &str) -> bool {
    let lowered = input.to_lowercase();
    if lowered.contains("waiting for input") {
        return true;
    }
    if lowered.contains("(y/n)") || lowered.contains("press enter") {
        return true;
    }
    if input.trim_end().ends_with('>') {
        return true;
    }
    false
}

fn is_active_pattern(input: &str) -> bool {
    let lowered = input.to_lowercase();
    if lowered.contains("thinking...") || lowered.contains("processing") {
        return true;
    }
    if lowered.contains("reading") && lowered.contains("file") {
        return true;
    }
    if lowered.contains("executing") {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waiting_pattern_takes_priority() {
        let input = StatusInput {
            now: 100,
            pane_last_activity: Some(95),
            pane_dead: false,
            pane_current_command: Some("bash"),
            output_line: Some("Waiting for input (y/n)"),
        };
        let result = detect_status(input, StatusConfig { idle_threshold_secs: 10 });
        assert_eq!(result.status, PaneStatus::Waiting);
        assert_eq!(result.reason, "waiting_pattern");
    }

    #[test]
    fn idle_when_no_recent_activity() {
        let input = StatusInput {
            now: 100,
            pane_last_activity: Some(10),
            pane_dead: false,
            pane_current_command: Some("bash"),
            output_line: None,
        };
        let result = detect_status(input, StatusConfig { idle_threshold_secs: 10 });
        assert_eq!(result.status, PaneStatus::Idle);
    }

    #[test]
    fn ended_when_pane_dead() {
        let input = StatusInput {
            now: 100,
            pane_last_activity: Some(90),
            pane_dead: true,
            pane_current_command: Some("bash"),
            output_line: None,
        };
        let result = detect_status(input, StatusConfig::default());
        assert_eq!(result.status, PaneStatus::Ended);
    }

    #[test]
    fn active_when_thinking_pattern() {
        let input = StatusInput {
            now: 100,
            pane_last_activity: Some(5), // Stale but active pattern
            pane_dead: false,
            pane_current_command: Some("claude"),
            output_line: Some("thinking..."),
        };
        let result = detect_status(input, StatusConfig { idle_threshold_secs: 10 });
        assert_eq!(result.status, PaneStatus::Active);
        assert_eq!(result.reason, "active_pattern");
    }

    #[test]
    fn active_when_processing_pattern() {
        let input = StatusInput {
            now: 100,
            pane_last_activity: Some(5),
            pane_dead: false,
            pane_current_command: Some("claude"),
            output_line: Some("Processing your request..."),
        };
        let result = detect_status(input, StatusConfig { idle_threshold_secs: 10 });
        assert_eq!(result.status, PaneStatus::Active);
        assert_eq!(result.reason, "active_pattern");
    }

    #[test]
    fn active_when_reading_file_pattern() {
        let input = StatusInput {
            now: 100,
            pane_last_activity: Some(5),
            pane_dead: false,
            pane_current_command: Some("claude"),
            output_line: Some("Reading file contents..."),
        };
        let result = detect_status(input, StatusConfig { idle_threshold_secs: 10 });
        assert_eq!(result.status, PaneStatus::Active);
        assert_eq!(result.reason, "active_pattern");
    }

    #[test]
    fn active_when_executing_pattern() {
        let input = StatusInput {
            now: 100,
            pane_last_activity: Some(5),
            pane_dead: false,
            pane_current_command: Some("claude"),
            output_line: Some("Executing command..."),
        };
        let result = detect_status(input, StatusConfig { idle_threshold_secs: 10 });
        assert_eq!(result.status, PaneStatus::Active);
        assert_eq!(result.reason, "active_pattern");
    }

    #[test]
    fn waiting_when_prompt_ends_with_chevron() {
        let input = StatusInput {
            now: 100,
            pane_last_activity: Some(99),
            pane_dead: false,
            pane_current_command: Some("bash"),
            output_line: Some("Enter your choice >"),
        };
        let result = detect_status(input, StatusConfig { idle_threshold_secs: 10 });
        assert_eq!(result.status, PaneStatus::Waiting);
    }

    #[test]
    fn waiting_when_press_enter_pattern() {
        let input = StatusInput {
            now: 100,
            pane_last_activity: Some(99),
            pane_dead: false,
            pane_current_command: Some("bash"),
            output_line: Some("Press enter to continue"),
        };
        let result = detect_status(input, StatusConfig { idle_threshold_secs: 10 });
        assert_eq!(result.status, PaneStatus::Waiting);
    }

    #[test]
    fn active_with_recent_activity() {
        let input = StatusInput {
            now: 100,
            pane_last_activity: Some(99),
            pane_dead: false,
            pane_current_command: Some("vim"),
            output_line: None,
        };
        let result = detect_status(input, StatusConfig { idle_threshold_secs: 10 });
        assert_eq!(result.status, PaneStatus::Active);
        assert!(result.reason.starts_with("recent_activity:"));
    }

    #[test]
    fn includes_command_in_active_reason() {
        let input = StatusInput {
            now: 100,
            pane_last_activity: Some(99),
            pane_dead: false,
            pane_current_command: Some("python"),
            output_line: None,
        };
        let result = detect_status(input, StatusConfig { idle_threshold_secs: 10 });
        assert_eq!(result.reason, "recent_activity:python");
    }

    #[test]
    fn strips_ansi_codes_for_pattern_matching() {
        let input = StatusInput {
            now: 100,
            pane_last_activity: Some(99),
            pane_dead: false,
            pane_current_command: Some("claude"),
            output_line: Some("\x1b[32mthinking...\x1b[0m"),
        };
        let result = detect_status(input, StatusConfig { idle_threshold_secs: 10 });
        assert_eq!(result.status, PaneStatus::Active);
        assert_eq!(result.reason, "active_pattern");
    }

    #[test]
    fn ended_takes_priority_over_everything() {
        let input = StatusInput {
            now: 100,
            pane_last_activity: Some(99),
            pane_dead: true,
            pane_current_command: Some("bash"),
            output_line: Some("thinking..."), // Active pattern but dead
        };
        let result = detect_status(input, StatusConfig { idle_threshold_secs: 10 });
        assert_eq!(result.status, PaneStatus::Ended);
        assert_eq!(result.reason, "pane_dead");
    }

    #[test]
    fn waiting_requires_recent_activity() {
        let input = StatusInput {
            now: 100,
            pane_last_activity: Some(50), // Not recent
            pane_dead: false,
            pane_current_command: Some("bash"),
            output_line: Some("Waiting for input (y/n)"),
        };
        let result = detect_status(input, StatusConfig { idle_threshold_secs: 10 });
        // Waiting pattern but stale activity - should fall through
        assert_eq!(result.status, PaneStatus::Idle);
    }

    #[test]
    fn idle_when_no_activity_timestamp() {
        let input = StatusInput {
            now: 100,
            pane_last_activity: None,
            pane_dead: false,
            pane_current_command: Some("bash"),
            output_line: None,
        };
        let result = detect_status(input, StatusConfig { idle_threshold_secs: 10 });
        assert_eq!(result.status, PaneStatus::Idle);
        assert_eq!(result.reason, "idle_timeout");
    }

    #[test]
    fn unknown_command_defaults_to_unknown() {
        let input = StatusInput {
            now: 100,
            pane_last_activity: Some(99),
            pane_dead: false,
            pane_current_command: None,
            output_line: None,
        };
        let result = detect_status(input, StatusConfig { idle_threshold_secs: 10 });
        assert_eq!(result.status, PaneStatus::Active);
        assert_eq!(result.reason, "recent_activity:unknown");
    }

    #[test]
    fn default_config_idle_threshold() {
        let config = StatusConfig::default();
        assert_eq!(config.idle_threshold_secs, 300);
    }

    #[test]
    fn case_insensitive_active_patterns() {
        let input = StatusInput {
            now: 100,
            pane_last_activity: Some(5),
            pane_dead: false,
            pane_current_command: Some("claude"),
            output_line: Some("THINKING... please wait"),
        };
        let result = detect_status(input, StatusConfig { idle_threshold_secs: 10 });
        assert_eq!(result.status, PaneStatus::Active);
    }

    #[test]
    fn case_insensitive_waiting_patterns() {
        let input = StatusInput {
            now: 100,
            pane_last_activity: Some(99),
            pane_dead: false,
            pane_current_command: Some("bash"),
            output_line: Some("WAITING FOR INPUT"),
        };
        let result = detect_status(input, StatusConfig { idle_threshold_secs: 10 });
        assert_eq!(result.status, PaneStatus::Waiting);
    }
}
