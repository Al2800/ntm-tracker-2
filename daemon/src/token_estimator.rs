/// Token count estimation for context tracking.
///
/// Uses character-based heuristics to estimate token counts for AI context windows.
/// The estimations are approximate but useful for tracking context usage.

/// Ratio of characters to tokens for English text.
const CHARS_PER_TOKEN_TEXT: f64 = 4.0;

/// Ratio of characters to tokens for code.
const CHARS_PER_TOKEN_CODE: f64 = 3.5;

/// Default token limit for Claude models (200k context).
pub const DEFAULT_TOKEN_LIMIT: u64 = 200_000;

#[derive(Clone, Debug, Default)]
pub struct TokenEstimator {
    /// Cumulative bytes seen since last reset.
    cumulative_bytes: u64,
    /// Last estimated token count.
    last_estimate: u64,
    /// Ground truth from NTM (if available).
    ntm_reported: Option<u64>,
    /// Configured token limit for progress calculation.
    token_limit: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct TokenEstimate {
    /// Estimated token count.
    pub tokens: u64,
    /// Formatted display string (e.g., "~45k").
    pub display: String,
    /// Progress as fraction (0.0 to 1.0+).
    pub progress: Option<f64>,
    /// Severity level: "low", "medium", "high".
    pub severity: String,
}

impl TokenEstimator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_limit(limit: u64) -> Self {
        Self {
            token_limit: Some(limit),
            ..Default::default()
        }
    }

    /// Add bytes from pane output.
    pub fn add_bytes(&mut self, bytes: u64) {
        self.cumulative_bytes = self.cumulative_bytes.saturating_add(bytes);
        self.last_estimate = estimate_tokens_from_bytes(self.cumulative_bytes);
    }

    /// Add text from pane output.
    pub fn add_text(&mut self, text: &str) {
        self.add_bytes(text.len() as u64);
    }

    /// Set ground truth from NTM if available.
    pub fn set_ntm_reported(&mut self, tokens: u64) {
        self.ntm_reported = Some(tokens);
    }

    /// Reset after compact detection.
    pub fn reset(&mut self) {
        self.cumulative_bytes = 0;
        self.last_estimate = 0;
        self.ntm_reported = None;
    }

    /// Get current cumulative bytes.
    pub fn cumulative_bytes(&self) -> u64 {
        self.cumulative_bytes
    }

    /// Get the estimated token count.
    pub fn estimate(&self) -> TokenEstimate {
        // Prefer NTM-reported ground truth if available.
        let tokens = self.ntm_reported.unwrap_or(self.last_estimate);
        let display = format_token_count(tokens);
        let progress = self.token_limit.map(|limit| tokens as f64 / limit as f64);
        let severity = match progress {
            Some(p) if p >= 0.9 => "high".to_string(),
            Some(p) if p >= 0.7 => "medium".to_string(),
            _ => "low".to_string(),
        };

        TokenEstimate {
            tokens,
            display,
            progress,
            severity,
        }
    }

    /// Get the context before value for compact events.
    pub fn context_before(&self) -> u64 {
        self.ntm_reported.unwrap_or(self.last_estimate)
    }
}

/// Estimate tokens from byte count using character-based heuristic.
pub fn estimate_tokens_from_bytes(bytes: u64) -> u64 {
    // Use average of text and code ratios.
    let avg_ratio = (CHARS_PER_TOKEN_TEXT + CHARS_PER_TOKEN_CODE) / 2.0;
    let raw = bytes as f64 / avg_ratio;
    round_to_nearest(raw as u64, 100)
}

/// Estimate tokens from text, detecting if it's likely code.
pub fn estimate_tokens_from_text(text: &str) -> u64 {
    let chars = text.len() as u64;
    let is_code = detect_code_content(text);
    let ratio = if is_code {
        CHARS_PER_TOKEN_CODE
    } else {
        CHARS_PER_TOKEN_TEXT
    };
    let raw = chars as f64 / ratio;
    round_to_nearest(raw as u64, 100)
}

/// Detect if text is likely code based on common patterns.
fn detect_code_content(text: &str) -> bool {
    let code_indicators = [
        "fn ", "def ", "func ", "function ", "class ", "struct ", "impl ",
        "pub ", "private ", "public ", "const ", "let ", "var ",
        "if (", "for (", "while (", "switch (", "match ",
        "import ", "from ", "require(", "include ",
        "=> ", "->", "::", "&&", "||", "!= ", "==",
        "{ }", "[]", "()", "();",
    ];

    let sample = if text.len() > 1000 {
        &text[..1000]
    } else {
        text
    };

    let indicator_count = code_indicators
        .iter()
        .filter(|ind| sample.contains(*ind))
        .count();

    // If 3+ code indicators found, likely code.
    indicator_count >= 3
}

/// Round to nearest value (e.g., 100, 1000).
fn round_to_nearest(value: u64, nearest: u64) -> u64 {
    if nearest == 0 {
        return value;
    }
    ((value + nearest / 2) / nearest) * nearest
}

/// Format token count for display.
pub fn format_token_count(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("~{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("~{}k", tokens / 1_000)
    } else {
        format!("~{}", tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_tokens_from_text_basic() {
        let text = "Hello, world!"; // 13 chars
        let tokens = estimate_tokens_from_text(text);
        // Should be ~3-4 tokens, rounded to 100.
        assert!(tokens <= 100);
    }

    #[test]
    fn estimate_tokens_from_code() {
        let code = r#"
fn main() {
    let x = 42;
    if (x > 0) {
        println!("positive");
    }
    for i in 0..10 {
        let y = i * 2;
        println!("{}", y);
    }
    pub fn helper() {
        let z = 100;
    }
}
"#;
        let tokens = estimate_tokens_from_text(code);
        // Code has higher density, so more tokens per char.
        // ~200 chars of code should estimate to some tokens (rounded to 100).
        // Test that it doesn't panic and produces a sensible result.
        assert_eq!(tokens % 100, 0); // Verify rounding works.
    }

    #[test]
    fn detect_code_content_works() {
        let code = "fn main() { let x = 42; }";
        assert!(detect_code_content(code));

        let text = "Hello, this is just plain text.";
        assert!(!detect_code_content(text));
    }

    #[test]
    fn format_token_count_basic() {
        assert_eq!(format_token_count(500), "~500");
        assert_eq!(format_token_count(1500), "~1k");
        assert_eq!(format_token_count(45000), "~45k");
        assert_eq!(format_token_count(1500000), "~1.5M");
    }

    #[test]
    fn round_to_nearest_works() {
        assert_eq!(round_to_nearest(123, 100), 100);
        assert_eq!(round_to_nearest(150, 100), 200);
        assert_eq!(round_to_nearest(199, 100), 200);
        assert_eq!(round_to_nearest(1234, 1000), 1000);
        assert_eq!(round_to_nearest(1567, 1000), 2000);
    }

    #[test]
    fn estimator_cumulative_tracking() {
        let mut estimator = TokenEstimator::new();
        estimator.add_bytes(4000);
        estimator.add_bytes(4000);

        let estimate = estimator.estimate();
        // 8000 bytes / ~3.75 chars per token = ~2133 tokens, rounded.
        assert!(estimate.tokens >= 2000);
        assert!(estimate.tokens <= 3000);
    }

    #[test]
    fn estimator_reset_clears_state() {
        let mut estimator = TokenEstimator::new();
        estimator.add_bytes(10000);
        estimator.set_ntm_reported(5000);

        estimator.reset();

        let estimate = estimator.estimate();
        assert_eq!(estimate.tokens, 0);
        assert_eq!(estimator.cumulative_bytes(), 0);
    }

    #[test]
    fn estimator_ntm_overrides_heuristic() {
        let mut estimator = TokenEstimator::new();
        estimator.add_bytes(10000);
        estimator.set_ntm_reported(5000);

        let estimate = estimator.estimate();
        assert_eq!(estimate.tokens, 5000);
    }

    #[test]
    fn estimator_with_limit_shows_progress() {
        let mut estimator = TokenEstimator::with_limit(100_000);
        estimator.set_ntm_reported(50_000);

        let estimate = estimator.estimate();
        assert_eq!(estimate.progress, Some(0.5));
        assert_eq!(estimate.severity, "low");
    }

    #[test]
    fn estimator_high_severity_near_limit() {
        let mut estimator = TokenEstimator::with_limit(100_000);
        estimator.set_ntm_reported(95_000);

        let estimate = estimator.estimate();
        assert_eq!(estimate.severity, "high");
    }

    #[test]
    fn estimator_medium_severity() {
        let mut estimator = TokenEstimator::with_limit(100_000);
        estimator.set_ntm_reported(75_000);

        let estimate = estimator.estimate();
        assert_eq!(estimate.severity, "medium");
    }

    #[test]
    fn estimator_add_text() {
        let mut estimator = TokenEstimator::new();
        estimator.add_text("Hello, world!");

        let estimate = estimator.estimate();
        assert!(estimate.tokens <= 100);
    }

    #[test]
    fn estimator_context_before() {
        let mut estimator = TokenEstimator::new();
        estimator.add_bytes(40000);

        let context = estimator.context_before();
        assert!(context > 0);
    }

    #[test]
    fn estimator_context_before_with_ntm() {
        let mut estimator = TokenEstimator::new();
        estimator.add_bytes(40000);
        estimator.set_ntm_reported(12345);

        assert_eq!(estimator.context_before(), 12345);
    }

    #[test]
    fn format_edge_cases() {
        assert_eq!(format_token_count(0), "~0");
        assert_eq!(format_token_count(999), "~999");
        assert_eq!(format_token_count(1000), "~1k");
        assert_eq!(format_token_count(999999), "~999k");
        assert_eq!(format_token_count(1000000), "~1.0M");
    }

    #[test]
    fn round_to_zero_nearest() {
        assert_eq!(round_to_nearest(123, 0), 123);
    }

    #[test]
    fn large_byte_count() {
        let mut estimator = TokenEstimator::new();
        estimator.add_bytes(800_000); // 800KB

        let estimate = estimator.estimate();
        // Should be ~200k tokens.
        assert!(estimate.tokens >= 150_000);
        assert!(estimate.tokens <= 250_000);
    }

    #[test]
    fn display_format_is_correct() {
        let mut estimator = TokenEstimator::with_limit(200_000);
        estimator.set_ntm_reported(45_000);

        let estimate = estimator.estimate();
        assert_eq!(estimate.display, "~45k");
    }

    #[test]
    fn default_token_limit_constant() {
        assert_eq!(DEFAULT_TOKEN_LIMIT, 200_000);
    }
}
