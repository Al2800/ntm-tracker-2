//! Detector pack loader for configurable detection patterns.
//!
//! Loads patterns from:
//! 1. Default embedded pack (compiled in)
//! 2. Custom pack from ~/.config/ntm-tracker/detectors.toml (optional override)

use regex::Regex;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Default detector pack embedded at compile time.
const DEFAULT_PACK: &str = include_str!("../../data/default_detectors.toml");

/// A loaded detector pack with compiled regex patterns.
#[derive(Clone, Debug)]
pub struct DetectorPack {
    pub version: String,
    pub min_daemon_version: String,
    pub description: String,
    pub compact_patterns: Vec<CompiledCompactPattern>,
    pub escalation_patterns: Vec<CompiledEscalationPattern>,
    pub prompt_patterns: Vec<Regex>,
    pub source_path: Option<PathBuf>,
}

/// Compiled compact detection pattern.
#[derive(Clone, Debug)]
pub struct CompiledCompactPattern {
    pub regex: Regex,
    pub confidence: f32,
    pub category: String,
    pub reason: String,
    pub source: String,
    pub original_pattern: String,
}

/// Compiled escalation detection pattern.
#[derive(Clone, Debug)]
pub struct CompiledEscalationPattern {
    pub regex: Regex,
    pub severity: String,
    pub confidence: f32,
    pub requires_prompt: bool,
    pub source: String,
    pub original_pattern: String,
}

/// Raw pack format as parsed from TOML.
#[derive(Debug, Deserialize)]
struct RawPack {
    pack: PackMeta,
    #[serde(default)]
    compact_patterns: Vec<RawCompactPattern>,
    #[serde(default)]
    escalation_patterns: Vec<RawEscalationPattern>,
    #[serde(default)]
    prompt_patterns: Vec<RawPromptPattern>,
}

#[derive(Debug, Deserialize)]
struct PackMeta {
    version: String,
    #[serde(default)]
    min_daemon_version: String,
    #[serde(default)]
    description: String,
}

#[derive(Debug, Deserialize)]
struct RawCompactPattern {
    pattern: String,
    #[serde(default)]
    flags: String,
    #[serde(default = "default_confidence")]
    confidence: f32,
    #[serde(default = "default_category")]
    category: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    source: String,
}

#[derive(Debug, Deserialize)]
struct RawEscalationPattern {
    pattern: String,
    #[serde(default)]
    flags: String,
    #[serde(default = "default_severity")]
    severity: String,
    #[serde(default = "default_confidence")]
    confidence: f32,
    #[serde(default)]
    requires_prompt: bool,
    #[serde(default)]
    source: String,
}

#[derive(Debug, Deserialize)]
struct RawPromptPattern {
    pattern: String,
    #[serde(default)]
    flags: String,
    #[allow(dead_code)]
    #[serde(default)]
    description: String,
}

fn default_confidence() -> f32 {
    0.8
}

fn default_category() -> String {
    "hard".to_string()
}

fn default_severity() -> String {
    "warn".to_string()
}

/// Errors that can occur when loading a detector pack.
#[derive(Debug)]
pub enum LoadError {
    Parse(toml::de::Error),
    Io(std::io::Error),
    InvalidPattern { pattern: String, error: String },
    VersionMismatch { required: String, current: String },
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "Failed to parse detector pack: {e}"),
            Self::Io(e) => write!(f, "Failed to read detector pack: {e}"),
            Self::InvalidPattern { pattern, error } => {
                write!(f, "Invalid pattern '{pattern}': {error}")
            }
            Self::VersionMismatch { required, current } => {
                write!(f, "Pack requires daemon {required}, current is {current}")
            }
        }
    }
}

impl std::error::Error for LoadError {}

impl From<toml::de::Error> for LoadError {
    fn from(e: toml::de::Error) -> Self {
        Self::Parse(e)
    }
}

impl From<std::io::Error> for LoadError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl DetectorPack {
    /// Load the default embedded detector pack.
    pub fn load_default() -> Result<Self, LoadError> {
        Self::from_toml(DEFAULT_PACK, None)
    }

    /// Load a detector pack from a file path.
    pub fn load_from_file(path: &PathBuf) -> Result<Self, LoadError> {
        let content = std::fs::read_to_string(path)?;
        Self::from_toml(&content, Some(path.clone()))
    }

    /// Load from config directory if available, otherwise use default.
    ///
    /// Looks for ~/.config/ntm-tracker/detectors.toml
    pub fn load_with_override() -> Result<Self, LoadError> {
        if let Some(config_dir) = config_dir() {
            let custom_path = config_dir.join("detectors.toml");
            if custom_path.exists() {
                info!(path = %custom_path.display(), "Loading custom detector pack");
                return Self::load_from_file(&custom_path);
            }
        }

        debug!("Using default detector pack");
        Self::load_default()
    }

    /// Parse a detector pack from TOML content.
    fn from_toml(content: &str, source_path: Option<PathBuf>) -> Result<Self, LoadError> {
        let raw: RawPack = toml::from_str(content)?;

        // Validate version compatibility
        if !raw.pack.min_daemon_version.is_empty() {
            let current = crate::version();
            if !version_compatible(&raw.pack.min_daemon_version, current) {
                return Err(LoadError::VersionMismatch {
                    required: raw.pack.min_daemon_version,
                    current: current.to_string(),
                });
            }
        }

        // Compile compact patterns
        let mut compact_patterns = Vec::with_capacity(raw.compact_patterns.len());
        for raw_pattern in raw.compact_patterns {
            match compile_pattern(&raw_pattern.pattern, &raw_pattern.flags) {
                Ok(regex) => {
                    compact_patterns.push(CompiledCompactPattern {
                        regex,
                        confidence: raw_pattern.confidence,
                        category: raw_pattern.category,
                        reason: raw_pattern.reason,
                        source: raw_pattern.source,
                        original_pattern: raw_pattern.pattern,
                    });
                }
                Err(e) => {
                    warn!(
                        pattern = %raw_pattern.pattern,
                        error = %e,
                        "Skipping invalid compact pattern"
                    );
                }
            }
        }

        // Compile escalation patterns
        let mut escalation_patterns = Vec::with_capacity(raw.escalation_patterns.len());
        for raw_pattern in raw.escalation_patterns {
            match compile_pattern(&raw_pattern.pattern, &raw_pattern.flags) {
                Ok(regex) => {
                    escalation_patterns.push(CompiledEscalationPattern {
                        regex,
                        severity: raw_pattern.severity,
                        confidence: raw_pattern.confidence,
                        requires_prompt: raw_pattern.requires_prompt,
                        source: raw_pattern.source,
                        original_pattern: raw_pattern.pattern,
                    });
                }
                Err(e) => {
                    warn!(
                        pattern = %raw_pattern.pattern,
                        error = %e,
                        "Skipping invalid escalation pattern"
                    );
                }
            }
        }

        // Compile prompt patterns
        let mut prompt_patterns = Vec::with_capacity(raw.prompt_patterns.len());
        for raw_pattern in raw.prompt_patterns {
            match compile_pattern(&raw_pattern.pattern, &raw_pattern.flags) {
                Ok(regex) => {
                    prompt_patterns.push(regex);
                }
                Err(e) => {
                    warn!(
                        pattern = %raw_pattern.pattern,
                        error = %e,
                        "Skipping invalid prompt pattern"
                    );
                }
            }
        }

        info!(
            version = %raw.pack.version,
            compact_count = compact_patterns.len(),
            escalation_count = escalation_patterns.len(),
            prompt_count = prompt_patterns.len(),
            "Loaded detector pack"
        );

        Ok(Self {
            version: raw.pack.version,
            min_daemon_version: raw.pack.min_daemon_version,
            description: raw.pack.description,
            compact_patterns,
            escalation_patterns,
            prompt_patterns,
            source_path,
        })
    }

    /// Check if a line matches any prompt pattern.
    pub fn is_prompt(&self, line: &str) -> bool {
        self.prompt_patterns.iter().any(|re| re.is_match(line))
    }

    /// Find the first matching compact pattern.
    pub fn match_compact(&self, line: &str) -> Option<&CompiledCompactPattern> {
        self.compact_patterns.iter().find(|p| p.regex.is_match(line))
    }

    /// Find the first matching escalation pattern.
    pub fn match_escalation(&self, line: &str) -> Option<&CompiledEscalationPattern> {
        self.escalation_patterns.iter().find(|p| p.regex.is_match(line))
    }

    /// Get patterns by category.
    pub fn compact_patterns_by_category(&self, category: &str) -> Vec<&CompiledCompactPattern> {
        self.compact_patterns
            .iter()
            .filter(|p| p.category == category)
            .collect()
    }

    /// Get patterns by severity.
    pub fn escalation_patterns_by_severity(&self, severity: &str) -> Vec<&CompiledEscalationPattern> {
        self.escalation_patterns
            .iter()
            .filter(|p| p.severity == severity)
            .collect()
    }
}

/// Compile a regex pattern with optional flags.
fn compile_pattern(pattern: &str, flags: &str) -> Result<Regex, regex::Error> {
    let pattern_str = if flags.contains('i') {
        format!("(?i){pattern}")
    } else {
        pattern.to_string()
    };
    Regex::new(&pattern_str)
}

/// Check if current version satisfies the minimum required version.
fn version_compatible(required: &str, current: &str) -> bool {
    // Simple semver comparison: parse major.minor.patch
    fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() >= 3 {
            Some((
                parts[0].parse().ok()?,
                parts[1].parse().ok()?,
                parts[2].parse().ok()?,
            ))
        } else if parts.len() == 2 {
            Some((
                parts[0].parse().ok()?,
                parts[1].parse().ok()?,
                0,
            ))
        } else if parts.len() == 1 {
            Some((parts[0].parse().ok()?, 0, 0))
        } else {
            None
        }
    }

    match (parse_version(required), parse_version(current)) {
        (Some((req_major, req_minor, req_patch)), Some((cur_major, cur_minor, cur_patch))) => {
            if cur_major > req_major {
                return true;
            }
            if cur_major < req_major {
                return false;
            }
            if cur_minor > req_minor {
                return true;
            }
            if cur_minor < req_minor {
                return false;
            }
            cur_patch >= req_patch
        }
        _ => true, // If we can't parse, assume compatible
    }
}

/// Get the config directory for detector overrides.
fn config_dir() -> Option<PathBuf> {
    if let Some(config) = std::env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(config).join("ntm-tracker"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        return Some(PathBuf::from(home).join(".config/ntm-tracker"));
    }
    None
}

/// Thread-safe detector pack holder with hot reload support.
#[derive(Clone)]
pub struct PackHolder {
    inner: Arc<std::sync::RwLock<DetectorPack>>,
}

impl PackHolder {
    /// Create a new pack holder with the default pack.
    pub fn new() -> Result<Self, LoadError> {
        let pack = DetectorPack::load_with_override()?;
        Ok(Self {
            inner: Arc::new(std::sync::RwLock::new(pack)),
        })
    }

    /// Get a read reference to the current pack.
    pub fn get(&self) -> std::sync::RwLockReadGuard<'_, DetectorPack> {
        self.inner.read().expect("pack holder lock")
    }

    /// Reload the pack from disk.
    pub fn reload(&self) -> Result<(), LoadError> {
        let new_pack = DetectorPack::load_with_override()?;
        let mut guard = self.inner.write().expect("pack holder lock");
        *guard = new_pack;
        info!("Detector pack reloaded");
        Ok(())
    }

    /// Reload from a specific file.
    pub fn reload_from(&self, path: &PathBuf) -> Result<(), LoadError> {
        let new_pack = DetectorPack::load_from_file(path)?;
        let mut guard = self.inner.write().expect("pack holder lock");
        *guard = new_pack;
        info!(path = %path.display(), "Detector pack reloaded from file");
        Ok(())
    }
}

impl Default for PackHolder {
    fn default() -> Self {
        Self::new().expect("Failed to load default detector pack")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_default_pack() {
        let pack = DetectorPack::load_default().expect("load default");
        assert_eq!(pack.version, "1.0.0");
        assert!(!pack.compact_patterns.is_empty());
        assert!(!pack.escalation_patterns.is_empty());
    }

    #[test]
    fn compiles_compact_patterns() {
        let pack = DetectorPack::load_default().expect("load default");

        // Test case-insensitive matching
        let matched = pack.match_compact("AUTO-COMPACTING CONVERSATION NOW");
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().reason, "auto_compacting");
    }

    #[test]
    fn compiles_escalation_patterns() {
        let pack = DetectorPack::load_default().expect("load default");

        let matched = pack.match_escalation("fatal error occurred");
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().severity, "error");
    }

    #[test]
    fn prompt_pattern_detection() {
        let pack = DetectorPack::load_default().expect("load default");

        assert!(pack.is_prompt("Continue? (y/n)"));
        assert!(pack.is_prompt("user@host:~$ "));
        assert!(pack.is_prompt("What next? > "));
        assert!(pack.is_prompt("Press enter to continue"));
        assert!(!pack.is_prompt("Just some text"));
    }

    #[test]
    fn version_compatibility() {
        assert!(version_compatible("0.1.0", "0.1.0"));
        assert!(version_compatible("0.1.0", "0.2.0"));
        assert!(version_compatible("0.1.0", "1.0.0"));
        assert!(!version_compatible("1.0.0", "0.9.0"));
        assert!(!version_compatible("0.2.0", "0.1.0"));
    }

    #[test]
    fn pack_holder_works() {
        let holder = PackHolder::default();
        let pack = holder.get();
        assert!(!pack.compact_patterns.is_empty());
    }

    #[test]
    fn filters_by_category() {
        let pack = DetectorPack::load_default().expect("load default");
        let hard = pack.compact_patterns_by_category("hard");
        let warning = pack.compact_patterns_by_category("warning");

        assert!(hard.len() > warning.len());
        for p in hard {
            assert_eq!(p.category, "hard");
        }
        for p in warning {
            assert_eq!(p.category, "warning");
        }
    }

    #[test]
    fn filters_by_severity() {
        let pack = DetectorPack::load_default().expect("load default");
        let errors = pack.escalation_patterns_by_severity("error");
        let warns = pack.escalation_patterns_by_severity("warn");

        assert!(!errors.is_empty());
        assert!(!warns.is_empty());
    }

    #[test]
    fn handles_invalid_toml() {
        let result = DetectorPack::from_toml("invalid { toml", None);
        assert!(result.is_err());
    }

    #[test]
    fn handles_missing_patterns() {
        let minimal = r#"
[pack]
version = "1.0.0"
"#;
        let pack = DetectorPack::from_toml(minimal, None).expect("parse minimal");
        assert!(pack.compact_patterns.is_empty());
        assert!(pack.escalation_patterns.is_empty());
    }

    #[test]
    fn skips_invalid_regex() {
        let with_bad_pattern = r#"
[pack]
version = "1.0.0"

[[compact_patterns]]
pattern = "[invalid("
confidence = 1.0
category = "hard"
reason = "test"

[[compact_patterns]]
pattern = "valid pattern"
confidence = 1.0
category = "hard"
reason = "test2"
"#;
        let pack = DetectorPack::from_toml(with_bad_pattern, None).expect("parse");
        // Should have 1 pattern (the valid one), not 2
        assert_eq!(pack.compact_patterns.len(), 1);
        assert_eq!(pack.compact_patterns[0].reason, "test2");
    }
}
