use crate::autostart;
use crate::daemon::DaemonManager;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
    time::Duration,
};
use tauri::{AppHandle, Manager, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub transport: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wsl_distro: Option<String>,
    pub reconnect_interval_ms: u64,
    pub autostart_enabled: bool,
    pub show_notifications: bool,
    pub notify_on_compact: bool,
    pub notify_on_escalation: bool,
    pub quiet_hours_start: u8,
    pub quiet_hours_end: u8,
    pub notification_max_per_hour: u32,
    pub theme: String,
    pub debug_mode: bool,
    pub log_level: String,
    pub first_run_complete: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            transport: "wsl-stdio".to_string(),
            wsl_distro: None,
            reconnect_interval_ms: 5000,
            autostart_enabled: true,
            show_notifications: true,
            notify_on_compact: true,
            notify_on_escalation: true,
            quiet_hours_start: 22,
            quiet_hours_end: 7,
            notification_max_per_hour: 10,
            theme: "system".to_string(),
            debug_mode: false,
            log_level: "info".to_string(),
            first_run_complete: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

const EXPECTED_PROTOCOL_VERSION: u64 = 1;
const EXPECTED_SCHEMA_VERSION: u64 = 1;

fn validate_daemon_hello(manager: &DaemonManager) -> Result<(), String> {
    let timeout = Duration::from_secs(5);
    let hello = manager.call("core.hello".to_string(), Value::Null, timeout)?;

    let daemon_version = hello
        .get("daemonVersion")
        .and_then(|value| value.as_str())
        .unwrap_or("<unknown>");

    let protocol_version = hello
        .get("protocolVersion")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| "Daemon hello missing protocolVersion".to_string())?;

    let schema_version = hello
        .get("schemaVersion")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| "Daemon hello missing schemaVersion".to_string())?;

    if protocol_version != EXPECTED_PROTOCOL_VERSION {
        return Err(format!(
            "Daemon version {daemon_version} is incompatible: protocolVersion={protocol_version} (expected {EXPECTED_PROTOCOL_VERSION})"
        ));
    }

    if schema_version != EXPECTED_SCHEMA_VERSION {
        return Err(format!(
            "Daemon version {daemon_version} is incompatible: schemaVersion={schema_version} (expected {EXPECTED_SCHEMA_VERSION})"
        ));
    }

    Ok(())
}

#[derive(Debug)]
pub(crate) struct DaemonState {
    pub(crate) manager: Option<DaemonManager>,
    pub(crate) last_error: Option<String>,
    pub(crate) settings: AppSettings,
}

pub struct AppState(pub Mutex<DaemonState>);

impl AppState {
    pub fn new(settings: AppSettings) -> Self {
        Self(Mutex::new(DaemonState {
            manager: None,
            last_error: None,
            settings,
        }))
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(AppSettings::default())
    }
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|err| format!("Unable to resolve app config directory: {err}"))?;
    fs::create_dir_all(&dir).map_err(|err| format!("Unable to create config directory: {err}"))?;
    Ok(dir.join("settings.json"))
}

pub fn load_settings(app: &AppHandle) -> AppSettings {
    let Ok(path) = settings_path(app) else {
        let mut settings = AppSettings::default();
        settings.first_run_complete = false;
        return settings;
    };
    let Ok(raw) = fs::read_to_string(path) else {
        let mut settings = AppSettings::default();
        settings.first_run_complete = false;
        return settings;
    };
    serde_json::from_str(&raw).unwrap_or_else(|_| {
        let mut settings = AppSettings::default();
        settings.first_run_complete = false;
        settings
    })
}

fn persist_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = settings_path(app)?;
    let payload =
        serde_json::to_string_pretty(settings).map_err(|err| format!("Serialize error: {err}"))?;
    fs::write(&path, payload).map_err(|err| format!("Unable to write settings: {err}"))?;
    Ok(())
}

fn resolve_bundle_dir(path: &str) -> Result<PathBuf, String> {
    let raw = PathBuf::from(path);
    let (base, stem) = if raw.extension().is_some() {
        let parent = raw
            .parent()
            .ok_or_else(|| "Diagnostics path missing parent directory".to_string())?;
        let stem = raw
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("ntm-tracker-diagnostics");
        (parent.to_path_buf(), stem.to_string())
    } else {
        (raw, String::new())
    };
    if base.exists() && base.is_file() {
        return Err("Diagnostics path points to a file".to_string());
    }
    let dir = if stem.is_empty() {
        base
    } else {
        base.join(stem)
    };
    fs::create_dir_all(&dir).map_err(|err| format!("Unable to create diagnostics dir: {err}"))?;
    Ok(dir)
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    let payload =
        serde_json::to_string_pretty(value).map_err(|err| format!("Serialize error: {err}"))?;
    fs::write(path, payload).map_err(|err| format!("Unable to write diagnostics: {err}"))?;
    Ok(())
}

fn os_version() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("cmd").args(["/C", "ver"]).output().ok()?;
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("uname").args(["-sr"]).output().ok()?;
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
    }

    None
}

fn resolve_log_path(config_payload: &Value) -> Option<PathBuf> {
    let logging = config_payload
        .get("config")
        .and_then(|config| config.get("logging"))?;
    let file = logging.get("file")?;
    let file_str = file.as_str()?;
    if file_str.trim().is_empty() {
        return None;
    }
    Some(PathBuf::from(file_str))
}

fn read_log_excerpt(path: &Path, max_lines: usize) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let path_str = path.to_string_lossy();
        if path_str.starts_with('/') {
            return read_log_excerpt_wsl(&path_str, max_lines);
        }
    }
    read_log_excerpt_local(path, max_lines)
}

fn read_log_excerpt_local(path: &Path, max_lines: usize) -> Result<String, String> {
    let file = File::open(path).map_err(|err| format!("Unable to open log file: {err}"))?;
    let reader = BufReader::new(file);
    let mut lines = VecDeque::with_capacity(max_lines);
    for line in reader.lines() {
        let line = line.map_err(|err| format!("Unable to read log line: {err}"))?;
        if lines.len() == max_lines {
            lines.pop_front();
        }
        lines.push_back(redact_line(&line));
    }
    Ok(lines.into_iter().collect::<Vec<_>>().join("\n"))
}

#[cfg(target_os = "windows")]
fn read_log_excerpt_wsl(path: &str, max_lines: usize) -> Result<String, String> {
    let command = format!("tail -n {max_lines} {}", shell_escape(path));
    let output = Command::new("wsl.exe")
        .args(["--", "sh", "-lc", &command])
        .output()
        .map_err(|err| format!("Unable to read logs via WSL: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "WSL log read failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let raw = String::from_utf8_lossy(&output.stdout);
    let redacted = raw.lines().map(redact_line).collect::<Vec<_>>().join("\n");
    Ok(redacted)
}

#[cfg(target_os = "windows")]
fn shell_escape(value: &str) -> String {
    let escaped = value.replace('\'', "'\\''");
    format!("'{}'", escaped)
}

fn redact_line(line: &str) -> String {
    let mut redacted = line.to_string();
    for key in [
        "token",
        "api_key",
        "apikey",
        "authorization",
        "bearer",
        "secret",
        "password",
    ] {
        if let Some(updated) = redact_after_key(&redacted, key) {
            redacted = updated;
            break;
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        redacted = redacted.replace(&home, "~");
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        redacted = redacted.replace(&profile, "~");
    }
    redacted
}

fn redact_after_key(line: &str, key: &str) -> Option<String> {
    let lower = line.to_lowercase();
    let start = lower.find(key)?;
    let tail = &line[start + key.len()..];
    let delimiter_offset = tail.find(|ch| ch == '=' || ch == ':')?;
    let split = start + key.len() + delimiter_offset + 1;
    Some(format!("{} [REDACTED]", line[..split].trim_end()))
}

#[tauri::command]
pub async fn list_wsl_distros(_app: AppHandle) -> Result<Vec<String>, String> {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("wsl.exe")
            .args(["-l", "-q"])
            .output()
            .map_err(|err| format!("Unable to list WSL distros: {err}"))?;
        if !output.status.success() {
            return Err(format!(
                "wsl.exe -l -q failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        let raw = String::from_utf8_lossy(&output.stdout);
        let distros = raw
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .map(|line| line.to_string())
            .collect::<Vec<_>>();
        Ok(distros)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("WSL distro listing is only available on Windows".to_string())
    }
}

#[tauri::command]
pub async fn daemon_start(_app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state
        .0
        .lock()
        .map_err(|_| "App state lock poisoned".to_string())?;
    if let Some(manager) = guard.manager.as_ref() {
        if manager.is_running() {
            return Ok(());
        }
    }

    match DaemonManager::start(
        &guard.settings.transport,
        guard.settings.wsl_distro.as_deref(),
    ) {
        Ok(manager) => {
            if let Err(err) = validate_daemon_hello(&manager) {
                let _ = manager.stop();
                guard.manager = None;
                guard.last_error = Some(err.clone());
                return Err(err);
            }

            guard.manager = Some(manager);
            guard.last_error = None;
            Ok(())
        }
        Err(err) => {
            guard.last_error = Some(err.clone());
            Err(err)
        }
    }
}

#[tauri::command]
pub async fn daemon_stop(_app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state
        .0
        .lock()
        .map_err(|_| "App state lock poisoned".to_string())?;
    if let Some(manager) = guard.manager.as_ref() {
        let _ = manager.stop();
    }
    guard.manager = None;
    Ok(())
}

#[tauri::command]
pub async fn daemon_restart(_app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state
        .0
        .lock()
        .map_err(|_| "App state lock poisoned".to_string())?;
    if let Some(manager) = guard.manager.as_ref() {
        let _ = manager.stop();
    }
    guard.manager = None;

    match DaemonManager::start(
        &guard.settings.transport,
        guard.settings.wsl_distro.as_deref(),
    ) {
        Ok(manager) => {
            if let Err(err) = validate_daemon_hello(&manager) {
                let _ = manager.stop();
                guard.manager = None;
                guard.last_error = Some(err.clone());
                return Err(err);
            }

            guard.manager = Some(manager);
            guard.last_error = None;
            Ok(())
        }
        Err(err) => {
            guard.last_error = Some(err.clone());
            Err(err)
        }
    }
}

#[tauri::command]
pub async fn daemon_health(
    _app: AppHandle,
    state: State<'_, AppState>,
) -> Result<HealthResponse, String> {
    let mut guard = state
        .0
        .lock()
        .map_err(|_| "App state lock poisoned".to_string())?;

    let Some(manager) = guard.manager.as_ref() else {
        return Ok(HealthResponse {
            status: "stopped".to_string(),
            last_error: guard.last_error.clone(),
        });
    };

    let running = manager.is_running();
    if !running && guard.last_error.is_none() {
        guard.last_error = Some("Daemon process is not running".to_string());
    }

    Ok(HealthResponse {
        status: if running {
            "running".to_string()
        } else {
            "error".to_string()
        },
        last_error: guard.last_error.clone(),
    })
}

#[tauri::command]
pub async fn rpc_call(
    _app: AppHandle,
    state: State<'_, AppState>,
    method: String,
    params: Value,
) -> Result<Value, String> {
    let mut guard = state
        .0
        .lock()
        .map_err(|_| "App state lock poisoned".to_string())?;
    let Some(manager) = guard.manager.as_ref() else {
        guard.last_error = Some("Daemon is not running".to_string());
        return Err("Daemon is not running".to_string());
    };
    if !manager.is_running() {
        guard.last_error = Some("Daemon health check failed".to_string());
        return Err("Daemon health check failed".to_string());
    }

    let timeout = Duration::from_secs(15);
    let result = manager.call(method, params, timeout);
    if let Err(err) = &result {
        guard.last_error = Some(err.clone());
    }
    result
}

#[tauri::command]
pub async fn get_settings(
    _app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AppSettings, String> {
    let guard = state
        .0
        .lock()
        .map_err(|_| "App state lock poisoned".to_string())?;
    Ok(guard.settings.clone())
}

#[tauri::command]
pub async fn set_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<(), String> {
    let mut guard = state
        .0
        .lock()
        .map_err(|_| "App state lock poisoned".to_string())?;
    if guard.settings.autostart_enabled != settings.autostart_enabled {
        autostart::set_enabled(settings.autostart_enabled)?;
    }
    guard.settings = settings.clone();
    persist_settings(&app, &settings)
}

#[tauri::command]
pub async fn export_diagnostics(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let bundle_dir = resolve_bundle_dir(&path)?;

    let (daemon_running, settings, health_result, stats_result, config_result) = {
        let guard = state
            .0
            .lock()
            .map_err(|_| "App state lock poisoned".to_string())?;
        let manager = guard.manager.as_ref();
        let daemon_running = manager.map(|manager| manager.is_running()).unwrap_or(false);

        let health_result = match (daemon_running, manager) {
            (true, Some(manager)) => manager.call(
                "health.get".to_string(),
                Value::Null,
                Duration::from_secs(5),
            ),
            (true, None) => Err("Daemon manager unavailable".to_string()),
            (false, _) => Err("Daemon not running".to_string()),
        };

        let stats_result = match (daemon_running, manager) {
            (true, Some(manager)) => manager.call(
                "stats.summary".to_string(),
                Value::Null,
                Duration::from_secs(5),
            ),
            (true, None) => Err("Daemon manager unavailable".to_string()),
            (false, _) => Err("Daemon not running".to_string()),
        };

        let config_result = match (daemon_running, manager) {
            (true, Some(manager)) => manager.call(
                "config.get".to_string(),
                Value::Null,
                Duration::from_secs(5),
            ),
            (true, None) => Err("Daemon manager unavailable".to_string()),
            (false, _) => Err("Daemon not running".to_string()),
        };

        (
            daemon_running,
            guard.settings.clone(),
            health_result,
            stats_result,
            config_result,
        )
    };

    let info = app.package_info();
    let daemon_version = health_result
        .as_ref()
        .ok()
        .and_then(|value| value.get("version"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());

    let versions_payload = json!({
        "app": {
            "name": info.name,
            "version": info.version.to_string(),
        },
        "daemon": {
            "version": daemon_version,
            "running": daemon_running,
        },
        "os": {
            "name": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "version": os_version(),
        }
    });
    write_json(&bundle_dir.join("versions.json"), &versions_payload)?;

    let (config_payload, log_path) = match config_result {
        Ok(config) => (
            json!({
                "appSettings": settings,
                "daemonConfig": config.get("config").cloned().unwrap_or(Value::Null),
                "daemonConfigPath": config.get("configPath").cloned().unwrap_or(Value::Null),
                "daemonAdminMode": config.get("adminMode").cloned().unwrap_or(Value::Null),
            }),
            resolve_log_path(&config),
        ),
        Err(err) => (
            json!({
                "appSettings": settings,
                "daemonConfigError": err,
            }),
            None,
        ),
    };
    write_json(&bundle_dir.join("config.json"), &config_payload)?;

    let health_payload = match health_result {
        Ok(payload) => payload,
        Err(err) => json!({ "error": err }),
    };
    write_json(&bundle_dir.join("daemon_health.json"), &health_payload)?;

    let stats_payload = match stats_result {
        Ok(payload) => payload,
        Err(err) => json!({ "error": err }),
    };
    write_json(&bundle_dir.join("stats_summary.json"), &stats_payload)?;

    let log_text = log_path
        .map(|path| read_log_excerpt(&path, 500))
        .transpose()?
        .unwrap_or_else(|| "No log file configured.".to_string());
    fs::write(bundle_dir.join("recent_logs.txt"), log_text)
        .map_err(|err| format!("Unable to write diagnostics: {err}"))?;

    Ok(())
}

#[tauri::command]
pub async fn get_attach_command(
    _app: AppHandle,
    _state: State<'_, AppState>,
    pane_id: String,
) -> Result<String, String> {
    Ok(format!("tmux attach -t {}", pane_id))
}
