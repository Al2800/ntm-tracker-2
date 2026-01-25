use regex::Regex;
use std::io::{Read, Result as IoResult};
use std::sync::OnceLock;

#[derive(Clone, Debug)]
pub struct RedactionConfig {
    pub patterns: Vec<String>,
    pub replacement: String,
    pub max_scan_bytes: usize,
}

impl Default for RedactionConfig {
    fn default() -> Self {
        Self {
            patterns: default_patterns(),
            replacement: "[REDACTED]".to_string(),
            max_scan_bytes: 32 * 1024,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Redactor {
    patterns: Vec<Regex>,
    replacement: String,
    max_scan_bytes: usize,
}

impl Redactor {
    pub fn from_config(config: RedactionConfig) -> Result<Self, String> {
        let mut compiled = Vec::with_capacity(config.patterns.len());
        for pattern in &config.patterns {
            let regex = Regex::new(pattern)
                .map_err(|err| format!("Invalid redaction pattern '{pattern}': {err}"))?;
            compiled.push(regex);
        }
        Ok(Self {
            patterns: compiled,
            replacement: config.replacement,
            max_scan_bytes: config.max_scan_bytes.max(1),
        })
    }

    pub fn redact(&self, input: &str) -> String {
        let truncated = if input.len() > self.max_scan_bytes {
            // Find a valid UTF-8 boundary to avoid panicking on multi-byte characters
            let mut end = self.max_scan_bytes;
            while end > 0 && !input.is_char_boundary(end) {
                end -= 1;
            }
            &input[..end]
        } else {
            input
        };
        let mut output = truncated.to_string();
        for pattern in &self.patterns {
            output = pattern.replace_all(&output, self.replacement.as_str()).to_string();
        }
        output
    }

    pub fn redact_streaming<R: Read>(&self, mut reader: R) -> IoResult<String> {
        let mut buf = vec![0; self.max_scan_bytes];
        let bytes = reader.read(&mut buf)?;
        let input = String::from_utf8_lossy(&buf[..bytes]);
        Ok(self.redact(&input))
    }
}

impl Default for Redactor {
    fn default() -> Self {
        Redactor::from_config(RedactionConfig::default()).unwrap_or_else(|_| Redactor {
            patterns: Vec::new(),
            replacement: "[REDACTED]".to_string(),
            max_scan_bytes: 32 * 1024,
        })
    }
}

static DEFAULT_REDACTOR: OnceLock<Redactor> = OnceLock::new();

pub fn default_redactor() -> &'static Redactor {
    DEFAULT_REDACTOR.get_or_init(Redactor::default)
}

fn default_patterns() -> Vec<String> {
    vec![
        r"AKIA[0-9A-Z]{16}".to_string(),
        r"-----BEGIN PRIVATE KEY-----[\s\S]+?-----END PRIVATE KEY-----".to_string(),
        r"Bearer\s+[A-Za-z0-9\-_\.]+".to_string(),
        r"/home/[^/]+/".to_string(),
        r"(?i)(password|secret|token)=\S+".to_string(),
    ]
}

pub fn build_redactor_with_custom_patterns(
    custom_patterns: &[String],
    replacement: Option<String>,
    max_scan_bytes: Option<usize>,
) -> Result<Redactor, String> {
    let mut patterns = default_patterns();
    for pattern in custom_patterns {
        patterns.push(pattern.clone());
    }
    Redactor::from_config(RedactionConfig {
        patterns,
        replacement: replacement.unwrap_or_else(|| "[REDACTED]".to_string()),
        max_scan_bytes: max_scan_bytes.unwrap_or(32 * 1024),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_handles_multibyte_utf8_at_boundary() {
        // Create a redactor with a max_scan_bytes that would fall in the middle
        // of a multi-byte UTF-8 character
        let config = RedactionConfig {
            patterns: vec![],
            replacement: "[REDACTED]".to_string(),
            max_scan_bytes: 5, // Will fall in middle of emoji if not handled
        };
        let redactor = Redactor::from_config(config).unwrap();

        // "Hello🎉" - emoji is 4 bytes, so bytes are: H(1) e(2) l(3) l(4) o(5) 🎉(6-9)
        // If max_scan_bytes=5, we want "Hello" not a panic
        let input = "Hello🎉World";
        let result = redactor.redact(input);
        assert_eq!(result, "Hello");

        // Test with emoji at the start
        let input2 = "🎉Hello";
        let config2 = RedactionConfig {
            patterns: vec![],
            replacement: "[REDACTED]".to_string(),
            max_scan_bytes: 2, // Falls in middle of emoji
        };
        let redactor2 = Redactor::from_config(config2).unwrap();
        // Should not panic, and should return empty or the valid prefix
        let result2 = redactor2.redact(input2);
        assert!(result2.is_empty() || result2.chars().all(|c| c.len_utf8() <= 2));
    }

    #[test]
    fn redact_basic_patterns() {
        let redactor = Redactor::default();

        // AWS key pattern
        let input = "key=AKIAIOSFODNN7EXAMPLE";
        let result = redactor.redact(input);
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("AKIAIOSFODNN7EXAMPLE"));

        // Bearer token
        let input2 = "Authorization: Bearer abc123.xyz789";
        let result2 = redactor.redact(input2);
        assert!(result2.contains("[REDACTED]"));
    }

    #[test]
    fn redact_password_pattern() {
        let redactor = Redactor::default();
        let input = "password=secret123";
        let result = redactor.redact(input);
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("secret123"));
    }
}
