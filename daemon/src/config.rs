use regex::Regex;
use serde::{Deserialize, Serialize};
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct ConfigError {
    pub message: String,
}

impl ConfigError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ConfigError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ServerConfig {
    pub bind: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:3847".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PollingConfig {
    /// How often we poll tmux/ntm for a full snapshot when activity is high.
    pub snapshot_interval_ms: u64,
    /// Polling interval when sessions are idle.
    pub snapshot_idle_interval_ms: u64,
    /// Polling interval when no sessions are detected.
    pub snapshot_background_interval_ms: u64,
    /// Polling interval when the daemon is degraded or unhealthy.
    pub snapshot_degraded_interval_ms: u64,
    /// Idle threshold (seconds) to classify sessions as active vs idle.
    pub idle_threshold_secs: i64,
}

impl Default for PollingConfig {
    fn default() -> Self {
        Self {
            snapshot_interval_ms: 2_000,
            snapshot_idle_interval_ms: 5_000,
            snapshot_background_interval_ms: 15_000,
            snapshot_degraded_interval_ms: 10_000,
            idle_threshold_secs: 300,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
#[derive(Default)]
pub struct CaptureConfig {
    pub capture_output: bool,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
#[derive(Default)]
pub struct SecurityConfig {
    pub admin_token_path: Option<PathBuf>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
#[derive(Default)]
pub struct PrivacyConfig {
    pub redaction_patterns: Vec<String>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct LoggingConfig {
    /// Log level: trace, debug, info, warn, error
    pub level: String,
    /// Log file path (optional, logs to file if set)
    pub file: Option<PathBuf>,
    /// Max log file size in MB before rotation
    pub max_file_mb: u64,
    /// Max number of rotated log files to keep
    pub max_files: usize,
    /// Output format: "json" or "text"
    pub format: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file: None,
            max_file_mb: 10,
            max_files: 5,
            format: "text".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct MaintenanceConfig {
    /// Rollup cadence for hourly/daily stats.
    pub rollup_interval_ms: u64,
    /// Vacuum interval in hours.
    pub vacuum_interval_hours: u64,
    /// Retention for minute samples (hours).
    pub minute_samples_retention_hours: u64,
    /// Retention for events (days).
    pub events_retention_days: u64,
    /// Retention for ended sessions (days).
    pub sessions_retention_days: u64,
    /// Maximum database size before aggressive pruning (MB).
    pub max_db_mb: u64,
}

impl Default for MaintenanceConfig {
    fn default() -> Self {
        Self {
            rollup_interval_ms: 3_600_000,
            vacuum_interval_hours: 168,
            minute_samples_retention_hours: 72,
            events_retention_days: 30,
            sessions_retention_days: 90,
            max_db_mb: 512,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
#[derive(Default)]
pub struct DaemonConfig {
    pub server: ServerConfig,
    pub polling: PollingConfig,
    pub capture: CaptureConfig,
    pub security: SecurityConfig,
    pub privacy: PrivacyConfig,
    pub logging: LoggingConfig,
    pub maintenance: MaintenanceConfig,
}


impl DaemonConfig {
    pub fn from_toml_str(raw: &str) -> Result<Self, ConfigError> {
        toml::from_str(raw).map_err(|err| ConfigError::new(format!("TOML parse error: {err}")))
    }

    pub fn apply_env_overrides(&mut self) {
        if let Ok(bind) = env::var("NTM_TRACKER_SERVER_BIND") {
            if !bind.trim().is_empty() {
                self.server.bind = bind;
            }
        }
        if let Ok(interval) = env::var("NTM_TRACKER_POLLING_SNAPSHOT_INTERVAL_MS") {
            if let Ok(parsed) = interval.parse::<u64>() {
                self.polling.snapshot_interval_ms = parsed;
            }
        }
        if let Ok(interval) = env::var("NTM_TRACKER_POLLING_SNAPSHOT_IDLE_INTERVAL_MS") {
            if let Ok(parsed) = interval.parse::<u64>() {
                self.polling.snapshot_idle_interval_ms = parsed;
            }
        }
        if let Ok(interval) = env::var("NTM_TRACKER_POLLING_SNAPSHOT_BACKGROUND_INTERVAL_MS") {
            if let Ok(parsed) = interval.parse::<u64>() {
                self.polling.snapshot_background_interval_ms = parsed;
            }
        }
        if let Ok(interval) = env::var("NTM_TRACKER_POLLING_SNAPSHOT_DEGRADED_INTERVAL_MS") {
            if let Ok(parsed) = interval.parse::<u64>() {
                self.polling.snapshot_degraded_interval_ms = parsed;
            }
        }
        if let Ok(threshold) = env::var("NTM_TRACKER_POLLING_IDLE_THRESHOLD_SECS") {
            if let Ok(parsed) = threshold.parse::<i64>() {
                self.polling.idle_threshold_secs = parsed;
            }
        }
        if let Ok(capture) = env::var("NTM_TRACKER_CAPTURE_OUTPUT") {
            let value = capture.trim().to_lowercase();
            self.capture.capture_output = matches!(value.as_str(), "1" | "true" | "yes" | "on");
        }
        if let Ok(patterns) = env::var("NTM_TRACKER_PRIVACY_REDACTION_PATTERNS") {
            let parsed: Vec<String> = patterns
                .split(',')
                .map(|p| p.trim())
                .filter(|p| !p.is_empty())
                .map(|p| p.to_string())
                .collect();
            if !parsed.is_empty() {
                self.privacy.redaction_patterns = parsed;
            }
        }
        if let Ok(path) = env::var("NTM_TRACKER_SECURITY_ADMIN_TOKEN_PATH") {
            let trimmed = path.trim();
            if !trimmed.is_empty() {
                self.security.admin_token_path = Some(PathBuf::from(trimmed));
            }
        }

        if let Ok(level) = env::var("NTM_TRACKER_LOG_LEVEL") {
            let trimmed = level.trim();
            if !trimmed.is_empty() {
                self.logging.level = trimmed.to_string();
            }
        }

        if let Ok(format) = env::var("NTM_TRACKER_LOG_FORMAT") {
            let trimmed = format.trim();
            if !trimmed.is_empty() {
                self.logging.format = trimmed.to_string();
            }
        }

        if let Ok(path) = env::var("NTM_TRACKER_LOG_FILE") {
            let trimmed = path.trim();
            if !trimmed.is_empty() {
                self.logging.file = Some(PathBuf::from(trimmed));
            }
        }

        if let Ok(max_mb) = env::var("NTM_TRACKER_LOG_MAX_FILE_MB") {
            if let Ok(parsed) = max_mb.trim().parse::<u64>() {
                self.logging.max_file_mb = parsed;
            }
        }

        if let Ok(max_files) = env::var("NTM_TRACKER_LOG_MAX_FILES") {
            if let Ok(parsed) = max_files.trim().parse::<usize>() {
                self.logging.max_files = parsed;
            }
        }

        if let Ok(interval) = env::var("NTM_TRACKER_MAINTENANCE_ROLLUP_INTERVAL_MS") {
            if let Ok(parsed) = interval.trim().parse::<u64>() {
                self.maintenance.rollup_interval_ms = parsed;
            }
        }

        if let Ok(hours) = env::var("NTM_TRACKER_MAINTENANCE_VACUUM_INTERVAL_HOURS") {
            if let Ok(parsed) = hours.trim().parse::<u64>() {
                self.maintenance.vacuum_interval_hours = parsed;
            }
        }

        if let Ok(hours) = env::var("NTM_TRACKER_MAINTENANCE_MINUTE_SAMPLES_RETENTION_HOURS") {
            if let Ok(parsed) = hours.trim().parse::<u64>() {
                self.maintenance.minute_samples_retention_hours = parsed;
            }
        }

        if let Ok(days) = env::var("NTM_TRACKER_MAINTENANCE_EVENTS_RETENTION_DAYS") {
            if let Ok(parsed) = days.trim().parse::<u64>() {
                self.maintenance.events_retention_days = parsed;
            }
        }

        if let Ok(days) = env::var("NTM_TRACKER_MAINTENANCE_SESSIONS_RETENTION_DAYS") {
            if let Ok(parsed) = days.trim().parse::<u64>() {
                self.maintenance.sessions_retention_days = parsed;
            }
        }

        if let Ok(max_mb) = env::var("NTM_TRACKER_MAINTENANCE_MAX_DB_MB") {
            if let Ok(parsed) = max_mb.trim().parse::<u64>() {
                self.maintenance.max_db_mb = parsed;
            }
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.polling.snapshot_interval_ms < 250 {
            return Err(ConfigError::new(
                "polling.snapshot-interval-ms must be >= 250",
            ));
        }
        if self.polling.snapshot_interval_ms > 60_000 {
            return Err(ConfigError::new(
                "polling.snapshot-interval-ms must be <= 60000",
            ));
        }
        if self.polling.snapshot_idle_interval_ms < self.polling.snapshot_interval_ms {
            return Err(ConfigError::new(
                "polling.snapshot-idle-interval-ms must be >= snapshot-interval-ms",
            ));
        }
        if self.polling.snapshot_idle_interval_ms > 120_000 {
            return Err(ConfigError::new(
                "polling.snapshot-idle-interval-ms must be <= 120000",
            ));
        }
        if self.polling.snapshot_background_interval_ms < self.polling.snapshot_idle_interval_ms {
            return Err(ConfigError::new(
                "polling.snapshot-background-interval-ms must be >= snapshot-idle-interval-ms",
            ));
        }
        if self.polling.snapshot_background_interval_ms > 300_000 {
            return Err(ConfigError::new(
                "polling.snapshot-background-interval-ms must be <= 300000",
            ));
        }
        if self.polling.snapshot_degraded_interval_ms < self.polling.snapshot_interval_ms {
            return Err(ConfigError::new(
                "polling.snapshot-degraded-interval-ms must be >= snapshot-interval-ms",
            ));
        }
        if self.polling.snapshot_degraded_interval_ms > 300_000 {
            return Err(ConfigError::new(
                "polling.snapshot-degraded-interval-ms must be <= 300000",
            ));
        }
        if self.polling.idle_threshold_secs < 30 {
            return Err(ConfigError::new(
                "polling.idle-threshold-secs must be >= 30",
            ));
        }
        if self.polling.idle_threshold_secs > 7_200 {
            return Err(ConfigError::new(
                "polling.idle-threshold-secs must be <= 7200",
            ));
        }

        for pattern in &self.privacy.redaction_patterns {
            Regex::new(pattern).map_err(|err| {
                ConfigError::new(format!("Invalid redaction regex '{pattern}': {err}"))
            })?;
        }

        if let Some(path) = &self.security.admin_token_path {
            validate_token_file_permissions(path)?;
        }

        if self.logging.max_files == 0 {
            return Err(ConfigError::new("logging.max-files must be >= 1"));
        }

        if self.logging.format != "text" && self.logging.format != "json" {
            return Err(ConfigError::new(
                "logging.format must be either 'text' or 'json'",
            ));
        }

        if self.maintenance.rollup_interval_ms < 60_000 {
            return Err(ConfigError::new(
                "maintenance.rollup-interval-ms must be >= 60000",
            ));
        }

        if self.maintenance.vacuum_interval_hours == 0 {
            return Err(ConfigError::new(
                "maintenance.vacuum-interval-hours must be >= 1",
            ));
        }

        if self.maintenance.minute_samples_retention_hours == 0 {
            return Err(ConfigError::new(
                "maintenance.minute-samples-retention-hours must be >= 1",
            ));
        }

        if self.maintenance.events_retention_days == 0 {
            return Err(ConfigError::new(
                "maintenance.events-retention-days must be >= 1",
            ));
        }

        if self.maintenance.sessions_retention_days == 0 {
            return Err(ConfigError::new(
                "maintenance.sessions-retention-days must be >= 1",
            ));
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct ConfigManager {
    path: Option<PathBuf>,
    config: Arc<RwLock<DaemonConfig>>,
}

impl ConfigManager {
    pub fn load_from_fs(config_override: Option<PathBuf>) -> Result<Self, ConfigError> {
        let path = resolve_config_path(config_override);
        let mut config = if let Some(ref path) = path {
            let raw = fs::read_to_string(path).map_err(|err| {
                ConfigError::new(format!("Unable to read config '{}': {err}", path.display()))
            })?;
            DaemonConfig::from_toml_str(&raw)?
        } else {
            DaemonConfig::default()
        };

        config.apply_env_overrides();
        config.validate()?;

        Ok(Self {
            path,
            config: Arc::new(RwLock::new(config)),
        })
    }

    pub fn current(&self) -> DaemonConfig {
        self.config
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn reload(&self) -> Result<DaemonConfig, ConfigError> {
        let Some(path) = &self.path else {
            // Nothing to reload: we are running on defaults only.
            return Ok(self.current());
        };

        let raw = fs::read_to_string(path).map_err(|err| {
            ConfigError::new(format!("Unable to read config '{}': {err}", path.display()))
        })?;
        let mut config = DaemonConfig::from_toml_str(&raw)?;
        config.apply_env_overrides();
        config.validate()?;

        let mut guard = self
            .config
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = config.clone();

        Ok(config)
    }

    pub fn config_path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self {
            path: None,
            config: Arc::new(RwLock::new(DaemonConfig::default())),
        }
    }
}

#[cfg(unix)]
pub async fn watch_sighup_for_reload(config: ConfigManager) -> Result<(), ConfigError> {
    use tokio::signal::unix::{signal, SignalKind};

    let mut stream =
        signal(SignalKind::hangup()).map_err(|err| ConfigError::new(err.to_string()))?;
    while stream.recv().await.is_some() {
        match config.reload() {
            Ok(_) => tracing::info!("config reloaded via SIGHUP"),
            Err(err) => tracing::warn!(error = %err, "config reload via SIGHUP failed"),
        }
    }

    Ok(())
}

fn resolve_config_path(config_override: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(path) = config_override {
        return Some(path);
    }

    let mut candidates = Vec::new();
    if let Some(home) = env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        candidates.push(PathBuf::from(home).join("ntm-tracker").join("daemon.toml"));
    } else if let Some(home) = env::var_os("HOME").filter(|value| !value.is_empty()) {
        candidates.push(
            PathBuf::from(home)
                .join(".config")
                .join("ntm-tracker")
                .join("daemon.toml"),
        );
    }
    candidates.push(PathBuf::from("/etc/ntm-tracker/daemon.toml"));

    candidates.into_iter().find(|path| path.exists())
}

#[cfg(unix)]
fn validate_token_file_permissions(path: &Path) -> Result<(), ConfigError> {
    use std::os::unix::fs::PermissionsExt;

    let meta = fs::metadata(path)
        .map_err(|err| ConfigError::new(format!("Unable to stat token file: {err}")))?;
    if !meta.is_file() {
        return Err(ConfigError::new("Admin token path is not a file"));
    }

    let mode = meta.permissions().mode() & 0o777;
    if mode != 0o600 {
        return Err(ConfigError::new(format!(
            "Admin token file permissions must be 0600 (got {:o})",
            mode
        )));
    }

    Ok(())
}

#[cfg(not(unix))]
fn validate_token_file_permissions(_path: &Path) -> Result<(), ConfigError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_parse_and_validate() {
        let mut config = DaemonConfig::default();
        config.apply_env_overrides();
        config.validate().expect("defaults validate");
    }

    #[test]
    fn toml_missing_fields_use_defaults() {
        let mut config = DaemonConfig::from_toml_str(
            r#"
[capture]
capture-output = true
"#,
        )
        .expect("parse");
        config.apply_env_overrides();
        assert!(config.capture.capture_output);
        assert_eq!(config.server.bind, "127.0.0.1:3847");
    }

    #[test]
    fn invalid_redaction_regex_fails_validation() {
        let mut config = DaemonConfig::default();
        config.privacy.redaction_patterns = vec!["[unclosed".to_string()];
        let err = config.validate().expect_err("validation error");
        assert!(err.message.contains("Invalid redaction regex"));
    }

    // --- Default values for all config sections ---

    #[test]
    fn server_config_defaults() {
        let config = ServerConfig::default();
        assert_eq!(config.bind, "127.0.0.1:3847");
    }

    #[test]
    fn polling_config_defaults() {
        let config = PollingConfig::default();
        assert_eq!(config.snapshot_interval_ms, 2_000);
        assert_eq!(config.snapshot_idle_interval_ms, 5_000);
        assert_eq!(config.snapshot_background_interval_ms, 15_000);
        assert_eq!(config.snapshot_degraded_interval_ms, 10_000);
        assert_eq!(config.idle_threshold_secs, 300);
    }

    #[test]
    fn logging_config_defaults() {
        let config = LoggingConfig::default();
        assert_eq!(config.level, "info");
        assert!(config.file.is_none());
        assert_eq!(config.max_file_mb, 10);
        assert_eq!(config.max_files, 5);
        assert_eq!(config.format, "text");
    }

    #[test]
    fn maintenance_config_defaults() {
        let config = MaintenanceConfig::default();
        assert_eq!(config.rollup_interval_ms, 3_600_000);
        assert_eq!(config.vacuum_interval_hours, 168);
        assert_eq!(config.minute_samples_retention_hours, 72);
        assert_eq!(config.events_retention_days, 30);
        assert_eq!(config.sessions_retention_days, 90);
        assert_eq!(config.max_db_mb, 512);
    }

    #[test]
    fn capture_config_defaults() {
        let config = CaptureConfig::default();
        assert!(!config.capture_output);
    }

    #[test]
    fn security_config_defaults() {
        let config = SecurityConfig::default();
        assert!(config.admin_token_path.is_none());
    }

    #[test]
    fn privacy_config_defaults() {
        let config = PrivacyConfig::default();
        assert!(config.redaction_patterns.is_empty());
    }

    // --- TOML parsing tests ---

    #[test]
    fn empty_toml_uses_all_defaults() {
        let config = DaemonConfig::from_toml_str("").unwrap();
        config.validate().unwrap();
        assert_eq!(config.server.bind, "127.0.0.1:3847");
        assert_eq!(config.polling.snapshot_interval_ms, 2_000);
    }

    #[test]
    fn full_toml_parse() {
        let config = DaemonConfig::from_toml_str(
            r#"
[server]
bind = "0.0.0.0:8080"

[polling]
snapshot-interval-ms = 1000
snapshot-idle-interval-ms = 3000
snapshot-background-interval-ms = 10000
snapshot-degraded-interval-ms = 5000
idle-threshold-secs = 120

[capture]
capture-output = true

[logging]
level = "debug"
format = "json"
max-file-mb = 50
max-files = 10

[maintenance]
rollup-interval-ms = 120000
vacuum-interval-hours = 24
events-retention-days = 7
sessions-retention-days = 30
max-db-mb = 256
"#,
        )
        .unwrap();
        assert_eq!(config.server.bind, "0.0.0.0:8080");
        assert_eq!(config.polling.snapshot_interval_ms, 1000);
        assert_eq!(config.polling.idle_threshold_secs, 120);
        assert!(config.capture.capture_output);
        assert_eq!(config.logging.level, "debug");
        assert_eq!(config.logging.format, "json");
        assert_eq!(config.maintenance.events_retention_days, 7);
    }

    #[test]
    fn invalid_toml_returns_error() {
        let result = DaemonConfig::from_toml_str("not valid {{toml");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("TOML parse error"));
    }

    // --- Validation boundary tests ---

    #[test]
    fn validation_snapshot_interval_too_low() {
        let mut config = DaemonConfig::default();
        config.polling.snapshot_interval_ms = 100;
        let err = config.validate().unwrap_err();
        assert!(err.message.contains("snapshot-interval-ms must be >= 250"));
    }

    #[test]
    fn validation_snapshot_interval_too_high() {
        let mut config = DaemonConfig::default();
        config.polling.snapshot_interval_ms = 100_000;
        let err = config.validate().unwrap_err();
        assert!(err.message.contains("snapshot-interval-ms must be <= 60000"));
    }

    #[test]
    fn validation_idle_less_than_snapshot() {
        let mut config = DaemonConfig::default();
        config.polling.snapshot_interval_ms = 5000;
        config.polling.snapshot_idle_interval_ms = 3000;
        let err = config.validate().unwrap_err();
        assert!(err.message.contains("idle-interval-ms must be >= snapshot-interval-ms"));
    }

    #[test]
    fn validation_idle_threshold_too_low() {
        let mut config = DaemonConfig::default();
        config.polling.idle_threshold_secs = 10;
        let err = config.validate().unwrap_err();
        assert!(err.message.contains("idle-threshold-secs must be >= 30"));
    }

    #[test]
    fn validation_idle_threshold_too_high() {
        let mut config = DaemonConfig::default();
        config.polling.idle_threshold_secs = 10_000;
        let err = config.validate().unwrap_err();
        assert!(err.message.contains("idle-threshold-secs must be <= 7200"));
    }

    #[test]
    fn validation_logging_format_invalid() {
        let mut config = DaemonConfig::default();
        config.logging.format = "yaml".to_string();
        let err = config.validate().unwrap_err();
        assert!(err.message.contains("logging.format must be either 'text' or 'json'"));
    }

    #[test]
    fn validation_logging_max_files_zero() {
        let mut config = DaemonConfig::default();
        config.logging.max_files = 0;
        let err = config.validate().unwrap_err();
        assert!(err.message.contains("logging.max-files must be >= 1"));
    }

    #[test]
    fn validation_maintenance_rollup_too_low() {
        let mut config = DaemonConfig::default();
        config.maintenance.rollup_interval_ms = 1000;
        let err = config.validate().unwrap_err();
        assert!(err.message.contains("rollup-interval-ms must be >= 60000"));
    }

    #[test]
    fn validation_maintenance_vacuum_zero() {
        let mut config = DaemonConfig::default();
        config.maintenance.vacuum_interval_hours = 0;
        let err = config.validate().unwrap_err();
        assert!(err.message.contains("vacuum-interval-hours must be >= 1"));
    }

    #[test]
    fn validation_maintenance_events_retention_zero() {
        let mut config = DaemonConfig::default();
        config.maintenance.events_retention_days = 0;
        let err = config.validate().unwrap_err();
        assert!(err.message.contains("events-retention-days must be >= 1"));
    }

    #[test]
    fn validation_maintenance_sessions_retention_zero() {
        let mut config = DaemonConfig::default();
        config.maintenance.sessions_retention_days = 0;
        let err = config.validate().unwrap_err();
        assert!(err.message.contains("sessions-retention-days must be >= 1"));
    }

    #[test]
    fn validation_valid_redaction_patterns() {
        let mut config = DaemonConfig::default();
        config.privacy.redaction_patterns = vec![
            r"\d{3}-\d{2}-\d{4}".to_string(),
            r"sk-[a-zA-Z0-9]+".to_string(),
        ];
        config.validate().unwrap();
    }

    // --- ConfigManager tests ---

    #[test]
    fn config_manager_default_returns_defaults() {
        let mgr = ConfigManager::default();
        let config = mgr.current();
        assert_eq!(config.server.bind, "127.0.0.1:3847");
        assert!(mgr.config_path().is_none());
    }

    #[test]
    fn config_manager_reload_without_path() {
        let mgr = ConfigManager::default();
        let result = mgr.reload().unwrap();
        assert_eq!(result.server.bind, "127.0.0.1:3847");
    }

    #[test]
    fn config_error_display() {
        let err = ConfigError::new("test error message");
        assert_eq!(err.to_string(), "test error message");
    }
}
