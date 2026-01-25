use crate::bootstrap;
use crate::commands::AppState;
use crate::daemon::DaemonManager;
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

const DAEMON_BIN_NAME: &str = "ntm-tracker-daemon";

#[derive(Debug, Serialize)]
struct DaemonVersionRecord {
    app_version: String,
    daemon_version: Option<String>,
    status: String,
    updated_at_unix: u64,
}

pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let upgrade_app = app.clone();
        let result =
            tauri::async_runtime::spawn_blocking(move || ensure_daemon_version(&upgrade_app))
                .await
                .map_err(|err| format!("upgrade task failed: {err}"))
                .and_then(|result| result);

        if let Err(err) = result {
            if let Some(state) = app.try_state::<AppState>() {
                let mut guard = match state.0.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                guard.last_error = Some(err);
            };
        }

        bootstrap::start(app);
    });
}

#[cfg(target_os = "windows")]
fn ensure_daemon_version(app: &AppHandle) -> Result<(), String> {
    let app_version = env!("CARGO_PKG_VERSION");
    let release_tag = format!("v{app_version}");
    let download_url = format!(
        "https://github.com/Al2800/ntm-tracker-2/releases/download/{release_tag}/{DAEMON_BIN_NAME}-x86_64-unknown-linux-gnu"
    );

    let script = format!(
        r#"set -euo pipefail

BIN="$HOME/.local/bin/{bin}"
CONFIG_DIR="$HOME/.config/ntm-tracker"
DATA_DIR="$HOME/.local/share/ntm-tracker"
TARGET_VERSION="{version}"

mkdir -p "$(dirname "$BIN")" "$CONFIG_DIR" "$DATA_DIR"

current_version=""
if command -v "{bin}" >/dev/null 2>&1; then
  current_version=$("{bin}" --version | awk '{{print $2}}')
fi

if [ "$current_version" = "$TARGET_VERSION" ]; then
  echo "up_to_date:$current_version"
  exit 0
fi

if command -v "{bin}" >/dev/null 2>&1; then
  "{bin}" stop >/dev/null 2>&1 || true
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required inside WSL to download {bin}" >&2
  exit 2
fi

tmp="$(mktemp)"
curl -fL "{url}" -o "$tmp"
chmod +x "$tmp"
if [ -f "$BIN" ]; then
  mv "$BIN" "$BIN.backup"
fi
mv "$tmp" "$BIN"

if ! "$BIN" --version >/dev/null 2>&1; then
  echo "rollback:health_check_failed" >&2
  if [ -f "$BIN.backup" ]; then
    mv "$BIN.backup" "$BIN"
    echo "rollback:restored"
    exit 3
  fi
  exit 4
fi
echo "upgraded:$TARGET_VERSION"
"#,
        bin = DAEMON_BIN_NAME,
        url = download_url,
        version = app_version
    );

    let output = Command::new("wsl.exe")
        .args(["--", "sh", "-lc", &script])
        .output()
        .map_err(|err| format!("Unable to run WSL upgrade: {err}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if !output.status.success() {
        write_version_record(app, None, "failed")?;
        return Err(format!(
            "WSL upgrade failed (status {:?}): {}",
            output.status.code(),
            if stderr.is_empty() { stdout } else { stderr }
        ));
    }

    let (status, daemon_version) = parse_status(&stdout);
    if status == "upgraded" {
        if let Err(err) = health_check() {
            let (rollback_status, rollback_version) = rollback_daemon()?;
            write_version_record(app, rollback_version, &rollback_status)?;
            return Err(format!("Upgrade failed health check: {err}"));
        }
    }
    write_version_record(app, daemon_version, &status)?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn ensure_daemon_version(_app: &AppHandle) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn parse_status(output: &str) -> (String, Option<String>) {
    if let Some(rest) = output.strip_prefix("up_to_date:") {
        return ("up_to_date".to_string(), Some(rest.trim().to_string()));
    }
    if let Some(rest) = output.strip_prefix("upgraded:") {
        return ("upgraded".to_string(), Some(rest.trim().to_string()));
    }
    if let Some(rest) = output.strip_prefix("rolled_back:") {
        return ("rolled_back".to_string(), Some(rest.trim().to_string()));
    }
    ("unknown".to_string(), None)
}

#[cfg(target_os = "windows")]
fn health_check() -> Result<(), String> {
    let manager = DaemonManager::start("wsl-stdio", None)?;
    let timeout = Duration::from_secs(5);
    let response = manager
        .call("health.get".to_string(), Value::Null, timeout)
        .map_err(|err| format!("health.get failed: {err}"));
    let _ = manager.stop();
    let payload = response?;
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown");
    if status != "ok" {
        return Err(format!("health status '{status}'"));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn rollback_daemon() -> Result<(String, Option<String>), String> {
    let script = format!(
        r#"set -euo pipefail

BIN="$HOME/.local/bin/{bin}"
if [ ! -f "$BIN.backup" ]; then
  echo "rollback_missing" >&2
  exit 2
fi

if [ -f "$BIN" ]; then
  mv "$BIN" "$BIN.failed"
fi
mv "$BIN.backup" "$BIN"
version=$("$BIN" --version | awk '{{print $2}}')
echo "rolled_back:$version"
"#,
        bin = DAEMON_BIN_NAME
    );

    let output = Command::new("wsl.exe")
        .args(["--", "sh", "-lc", &script])
        .output()
        .map_err(|err| format!("Unable to run WSL rollback: {err}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if !output.status.success() {
        return Err(format!(
            "WSL rollback failed (status {:?}): {}",
            output.status.code(),
            if stderr.is_empty() { stdout } else { stderr }
        ));
    }

    Ok(parse_status(&stdout))
}

#[cfg(target_os = "windows")]
fn write_version_record(
    app: &AppHandle,
    daemon_version: Option<String>,
    status: &str,
) -> Result<(), String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|err| format!("Unable to resolve app config directory: {err}"))?;
    fs::create_dir_all(&dir).map_err(|err| format!("Unable to create config directory: {err}"))?;

    let record = DaemonVersionRecord {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        daemon_version,
        status: status.to_string(),
        updated_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };
    let payload = serde_json::to_string_pretty(&record)
        .map_err(|err| format!("Unable to serialize version record: {err}"))?;
    let path = version_record_path(&dir);
    fs::write(&path, payload).map_err(|err| format!("Unable to write version record: {err}"))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn version_record_path(dir: &PathBuf) -> PathBuf {
    dir.join("daemon-version.json")
}
