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
    /// How often we poll tmux/ntm for a full snapshot.
    pub snapshot_interval_ms: u64,
}

impl Default for PollingConfig {
    fn default() -> Self {
        Self {
            snapshot_interval_ms: 2_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CaptureConfig {
    pub capture_output: bool,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            capture_output: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct SecurityConfig {
    pub admin_token_path: Option<PathBuf>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            admin_token_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PrivacyConfig {
    pub redaction_patterns: Vec<String>,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            redaction_patterns: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct DaemonConfig {
    pub server: ServerConfig,
    pub polling: PollingConfig,
    pub capture: CaptureConfig,
    pub security: SecurityConfig,
    pub privacy: PrivacyConfig,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            polling: PollingConfig::default(),
            capture: CaptureConfig::default(),
            security: SecurityConfig::default(),
            privacy: PrivacyConfig::default(),
        }
    }
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

        for pattern in &self.privacy.redaction_patterns {
            Regex::new(pattern).map_err(|err| {
                ConfigError::new(format!("Invalid redaction regex '{pattern}': {err}"))
            })?;
        }

        if let Some(path) = &self.security.admin_token_path {
            validate_token_file_permissions(path)?;
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
            .unwrap_or_else(|_| panic!("Config lock poisoned"))
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

        *self
            .config
            .write()
            .unwrap_or_else(|_| panic!("Config lock poisoned")) = config.clone();

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
}
