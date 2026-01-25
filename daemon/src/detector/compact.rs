#[derive(Clone, Debug)]
pub struct CompactInput<'a> {
    pub now: i64,
    pub line: &'a str,
    pub ntm_compact_count: Option<u64>,
    pub context_tokens: Option<u64>,
    pub previous_tokens: Option<u64>,
}

#[derive(Clone, Debug, Default)]
pub struct CompactDetection {
    pub confidence: f32,
    pub trigger: String,
    pub context_before: Option<u64>,
    pub reason: String,
    pub matched_text: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CompactConfig {
    pub debounce_secs: i64,
    pub drop_min_tokens: u64,
    pub drop_ratio: f32,
}

impl Default for CompactConfig {
    fn default() -> Self {
        Self {
            debounce_secs: 60,
            drop_min_tokens: 20_000,
            drop_ratio: 0.75,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CompactDetector {
    last_detected_at: Option<i64>,
    last_compact_count: Option<u64>,
    config: CompactConfig,
}

impl CompactDetector {
    pub fn new(config: CompactConfig) -> Self {
        Self {
            last_detected_at: None,
            last_compact_count: None,
            config,
        }
    }

    pub fn detect(&mut self, input: CompactInput<'_>) -> Option<CompactDetection> {
        if self.is_debounced(input.now) {
            return None;
        }

        let stripped = strip_ansi(input.line);
        if let Some(reason) = hard_match_reason(&stripped) {
            return Some(self.mark_detected(
                input.now,
                CompactDetection {
                    confidence: 1.0,
                    trigger: "auto".to_string(),
                    context_before: input.context_tokens,
                    reason,
                    matched_text: Some(stripped),
                },
            ));
        }

        if let Some(reason) = warning_match_reason(&stripped) {
            return Some(self.mark_detected(
                input.now,
                CompactDetection {
                    confidence: 0.4,
                    trigger: "auto".to_string(),
                    context_before: input.context_tokens,
                    reason,
                    matched_text: Some(stripped),
                },
            ));
        }

        if let Some(count) = input.ntm_compact_count {
            if self.last_compact_count.map(|prev| count > prev).unwrap_or(true) {
                self.last_compact_count = Some(count);
                return Some(self.mark_detected(
                    input.now,
                    CompactDetection {
                        confidence: 0.8,
                        trigger: "auto".to_string(),
                        context_before: input.context_tokens,
                        reason: "ntm_counter".to_string(),
                        matched_text: None,
                    },
                ));
            }
        }

        if let (Some(previous), Some(current)) = (input.previous_tokens, input.context_tokens) {
            if previous >= self.config.drop_min_tokens {
                let drop_ratio = 1.0 - (current as f32 / previous as f32);
                if drop_ratio >= self.config.drop_ratio {
                    return Some(self.mark_detected(
                        input.now,
                        CompactDetection {
                            confidence: 0.6,
                            trigger: "auto".to_string(),
                            context_before: Some(previous),
                            reason: "context_drop".to_string(),
                            matched_text: None,
                        },
                    ));
                }
            }
        }

        None
    }

    fn is_debounced(&self, now: i64) -> bool {
        self.last_detected_at
            .map(|last| now.saturating_sub(last) < self.config.debounce_secs)
            .unwrap_or(false)
    }

    fn mark_detected(&mut self, now: i64, detection: CompactDetection) -> CompactDetection {
        self.last_detected_at = Some(now);
        detection
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

fn hard_match_reason(input: &str) -> Option<String> {
    let lowered = input.to_lowercase();
    let matches = [
        ("auto-compacting conversation", "auto_compacting"),
        ("conversation compacted", "conversation_compacted"),
        ("context limit reached", "context_limit"),
        ("starting fresh context", "fresh_context"),
    ];
    for (needle, reason) in matches {
        if lowered.contains(needle) {
            return Some(reason.to_string());
        }
    }

    if lowered.contains("compacting") && !lowered.contains("approaching") && !lowered.contains("soon") {
        return Some("compacting".to_string());
    }

    if lowered.contains("context") && lowered.contains("reset") {
        return Some("context_reset".to_string());
    }

    None
}

fn warning_match_reason(input: &str) -> Option<String> {
    let lowered = input.to_lowercase();
    if lowered.contains("approaching context limit") {
        return Some("approaching_limit".to_string());
    }

    if lowered.contains("context") && lowered.contains("limit") && (lowered.contains("near") || lowered.contains("close")) {
        return Some("context_near_limit".to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_hard_pattern() {
        let mut detector = CompactDetector::default();
        let input = CompactInput {
            now: 100,
            line: "Auto-compacting conversation due to context limit.",
            ntm_compact_count: None,
            context_tokens: Some(90000),
            previous_tokens: Some(90000),
        };
        let result = detector.detect(input).expect("detect");
        assert_eq!(result.reason, "auto_compacting");
        assert!(result.confidence >= 1.0);
    }

    #[test]
    fn debounce_blocks_repeat() {
        let mut detector = CompactDetector::default();
        let input = CompactInput {
            now: 100,
            line: "Context limit reached",
            ntm_compact_count: None,
            context_tokens: Some(80000),
            previous_tokens: Some(80000),
        };
        assert!(detector.detect(input).is_some());
        let second = CompactInput {
            now: 120,
            line: "Context limit reached",
            ntm_compact_count: None,
            context_tokens: Some(80000),
            previous_tokens: Some(80000),
        };
        assert!(detector.detect(second).is_none());
    }

    #[test]
    fn detects_counter_increase() {
        let mut detector = CompactDetector::default();
        let input = CompactInput {
            now: 100,
            line: "",
            ntm_compact_count: Some(1),
            context_tokens: Some(1000),
            previous_tokens: Some(1000),
        };
        assert!(detector.detect(input).is_some());
    }

    #[test]
    fn detects_context_drop() {
        let mut detector = CompactDetector::default();
        let input = CompactInput {
            now: 100,
            line: "",
            ntm_compact_count: None,
            context_tokens: Some(10000),
            previous_tokens: Some(50000),
        };
        let result = detector.detect(input).expect("detect");
        assert_eq!(result.reason, "context_drop");
    }

    #[test]
    fn detects_conversation_compacted() {
        let mut detector = CompactDetector::default();
        let input = CompactInput {
            now: 100,
            line: "Conversation compacted to fit context window",
            ntm_compact_count: None,
            context_tokens: None,
            previous_tokens: None,
        };
        let result = detector.detect(input).expect("detect");
        assert_eq!(result.reason, "conversation_compacted");
        assert!(result.confidence >= 1.0);
    }

    #[test]
    fn detects_fresh_context() {
        let mut detector = CompactDetector::default();
        let input = CompactInput {
            now: 100,
            line: "Starting fresh context after limit exceeded",
            ntm_compact_count: None,
            context_tokens: None,
            previous_tokens: None,
        };
        let result = detector.detect(input).expect("detect");
        assert_eq!(result.reason, "fresh_context");
    }

    #[test]
    fn detects_context_reset() {
        let mut detector = CompactDetector::default();
        let input = CompactInput {
            now: 100,
            line: "Context has been reset for this session",
            ntm_compact_count: None,
            context_tokens: None,
            previous_tokens: None,
        };
        let result = detector.detect(input).expect("detect");
        assert_eq!(result.reason, "context_reset");
    }

    #[test]
    fn detects_compacting_without_false_positives() {
        let mut detector = CompactDetector::default();
        let input = CompactInput {
            now: 100,
            line: "Compacting the data now",
            ntm_compact_count: None,
            context_tokens: None,
            previous_tokens: None,
        };
        let result = detector.detect(input).expect("detect");
        assert_eq!(result.reason, "compacting");
    }

    #[test]
    fn ignores_approaching_compacting() {
        let mut detector = CompactDetector::default();
        // "approaching compacting soon" should not trigger hard match
        let input = CompactInput {
            now: 100,
            line: "approaching compacting soon",
            ntm_compact_count: None,
            context_tokens: None,
            previous_tokens: None,
        };
        let result = detector.detect(input);
        // Should be None or low confidence warning, not hard match
        assert!(result.is_none() || result.as_ref().unwrap().confidence < 1.0);
    }

    #[test]
    fn detects_warning_approaching_limit() {
        let mut detector = CompactDetector::default();
        let input = CompactInput {
            now: 100,
            line: "Warning: approaching context limit",
            ntm_compact_count: None,
            context_tokens: None,
            previous_tokens: None,
        };
        let result = detector.detect(input).expect("detect");
        assert_eq!(result.reason, "approaching_limit");
        assert!(result.confidence < 1.0); // Should be warning level
    }

    #[test]
    fn detects_context_near_limit() {
        let mut detector = CompactDetector::default();
        let input = CompactInput {
            now: 100,
            line: "context is near the limit",
            ntm_compact_count: None,
            context_tokens: None,
            previous_tokens: None,
        };
        let result = detector.detect(input).expect("detect");
        assert_eq!(result.reason, "context_near_limit");
    }

    #[test]
    fn strips_ansi_codes_before_matching() {
        let mut detector = CompactDetector::default();
        // Input with ANSI escape codes
        let input = CompactInput {
            now: 100,
            line: "\x1b[31mAuto-compacting conversation\x1b[0m",
            ntm_compact_count: None,
            context_tokens: None,
            previous_tokens: None,
        };
        let result = detector.detect(input).expect("detect");
        assert_eq!(result.reason, "auto_compacting");
    }

    #[test]
    fn counter_increase_after_initial() {
        let mut detector = CompactDetector::default();
        // First detection with counter 1
        let input1 = CompactInput {
            now: 100,
            line: "",
            ntm_compact_count: Some(1),
            context_tokens: None,
            previous_tokens: None,
        };
        assert!(detector.detect(input1).is_some());

        // After debounce, same counter shouldn't trigger again
        let input2 = CompactInput {
            now: 200,
            line: "",
            ntm_compact_count: Some(1),
            context_tokens: None,
            previous_tokens: None,
        };
        assert!(detector.detect(input2).is_none());

        // But increased counter should trigger
        let input3 = CompactInput {
            now: 300,
            line: "",
            ntm_compact_count: Some(2),
            context_tokens: None,
            previous_tokens: None,
        };
        assert!(detector.detect(input3).is_some());
    }

    #[test]
    fn context_drop_needs_minimum_tokens() {
        let mut detector = CompactDetector::default();
        // Previous tokens below threshold shouldn't trigger
        let input = CompactInput {
            now: 100,
            line: "",
            ntm_compact_count: None,
            context_tokens: Some(1000),
            previous_tokens: Some(5000), // Below 20_000 threshold
        };
        assert!(detector.detect(input).is_none());
    }

    #[test]
    fn context_drop_needs_ratio() {
        let mut detector = CompactDetector::default();
        // Small drop shouldn't trigger
        let input = CompactInput {
            now: 100,
            line: "",
            ntm_compact_count: None,
            context_tokens: Some(45000),
            previous_tokens: Some(50000), // Only 10% drop
        };
        assert!(detector.detect(input).is_none());
    }

    #[test]
    fn allows_detection_after_debounce_window() {
        let config = CompactConfig {
            debounce_secs: 60,
            ..Default::default()
        };
        let mut detector = CompactDetector::new(config);

        let input1 = CompactInput {
            now: 100,
            line: "Context limit reached",
            ntm_compact_count: None,
            context_tokens: None,
            previous_tokens: None,
        };
        assert!(detector.detect(input1).is_some());

        // After debounce window passes
        let input2 = CompactInput {
            now: 170, // 70 seconds later, past 60s debounce
            line: "Context limit reached again",
            ntm_compact_count: None,
            context_tokens: None,
            previous_tokens: None,
        };
        assert!(detector.detect(input2).is_some());
    }

    #[test]
    fn no_detection_for_empty_line() {
        let mut detector = CompactDetector::default();
        let input = CompactInput {
            now: 100,
            line: "",
            ntm_compact_count: None,
            context_tokens: None,
            previous_tokens: None,
        };
        assert!(detector.detect(input).is_none());
    }

    #[test]
    fn case_insensitive_matching() {
        let mut detector = CompactDetector::default();
        let input = CompactInput {
            now: 100,
            line: "AUTO-COMPACTING CONVERSATION NOW",
            ntm_compact_count: None,
            context_tokens: None,
            previous_tokens: None,
        };
        let result = detector.detect(input).expect("detect");
        assert_eq!(result.reason, "auto_compacting");
    }
}
