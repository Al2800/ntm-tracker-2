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

    if lowered.contains("compacting") {
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
}
