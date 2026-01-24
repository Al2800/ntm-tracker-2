use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fs, path::PathBuf, sync::Mutex};
use tauri::{AppHandle, Manager, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub transport: String,
    pub reconnect_interval_ms: u64,
    pub show_notifications: bool,
    pub notify_on_compact: bool,
    pub notify_on_escalation: bool,
    pub theme: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            transport: "wsl-stdio".to_string(),
            reconnect_interval_ms: 5000,
            show_notifications: true,
            notify_on_compact: true,
            notify_on_escalation: true,
            theme: "system".to_string(),
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

#[derive(Debug)]
struct DaemonState {
    running: bool,
    last_error: Option<String>,
    settings: AppSettings,
}

pub struct AppState(pub Mutex<DaemonState>);

impl AppState {
    pub fn new(settings: AppSettings) -> Self {
        Self(Mutex::new(DaemonState {
            running: false,
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
    let resolver = app.path_resolver();
    let dir = resolver
        .app_config_dir()
        .ok_or_else(|| "Unable to resolve app config directory".to_string())?;
    fs::create_dir_all(&dir)
        .map_err(|err| format!("Unable to create config directory: {err}"))?;
    Ok(dir.join("settings.json"))
}

pub fn load_settings(app: &AppHandle) -> AppSettings {
    let Ok(path) = settings_path(app) else {
        return AppSettings::default();
    };
    let Ok(raw) = fs::read_to_string(path) else {
        return AppSettings::default();
    };
    serde_json::from_str(&raw).unwrap_or_else(|_| AppSettings::default())
}

fn persist_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = settings_path(app)?;
    let payload =
        serde_json::to_string_pretty(settings).map_err(|err| format!("Serialize error: {err}"))?;
    fs::write(&path, payload).map_err(|err| format!("Unable to write settings: {err}"))?;
    Ok(())
}

#[tauri::command]
pub async fn daemon_start(_app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state
        .0
        .lock()
        .map_err(|_| "App state lock poisoned".to_string())?;
    guard.running = true;
    guard.last_error = None;
    Ok(())
}

#[tauri::command]
pub async fn daemon_stop(_app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state
        .0
        .lock()
        .map_err(|_| "App state lock poisoned".to_string())?;
    guard.running = false;
    Ok(())
}

#[tauri::command]
pub async fn daemon_health(_app: AppHandle, state: State<'_, AppState>) -> Result<HealthResponse, String> {
    let guard = state
        .0
        .lock()
        .map_err(|_| "App state lock poisoned".to_string())?;
    Ok(HealthResponse {
        status: if guard.running {
            "running".to_string()
        } else {
            "stopped".to_string()
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
    let guard = state
        .0
        .lock()
        .map_err(|_| "App state lock poisoned".to_string())?;
    if !guard.running {
        return Err("Daemon is not running".to_string());
    }
    let _ = params;
    Err(format!(
        "RPC passthrough not implemented yet for method '{method}'"
    ))
}

#[tauri::command]
pub async fn get_settings(_app: AppHandle, state: State<'_, AppState>) -> Result<AppSettings, String> {
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
    guard.settings = settings.clone();
    persist_settings(&app, &settings)
}

#[tauri::command]
pub async fn export_diagnostics(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    #[derive(Serialize)]
    struct Diagnostics {
        app_name: String,
        app_version: String,
        daemon_running: bool,
        last_error: Option<String>,
        settings: AppSettings,
    }

    let (running, last_error, settings) = {
        let guard = state
            .0
            .lock()
            .map_err(|_| "App state lock poisoned".to_string())?;
        (guard.running, guard.last_error.clone(), guard.settings.clone())
    };

    let info = app.package_info();
    let payload = Diagnostics {
        app_name: info.name.clone(),
        app_version: info.version.to_string(),
        daemon_running: running,
        last_error,
        settings,
    };
    let body =
        serde_json::to_string_pretty(&payload).map_err(|err| format!("Serialize error: {err}"))?;
    fs::write(path, body).map_err(|err| format!("Unable to write diagnostics: {err}"))?;
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
