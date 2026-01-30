use crate::commands::AppState;
use crate::daemon::DaemonManager;
use std::process::Command;
use std::time::Duration;
use tauri::{AppHandle, Manager};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(target_os = "windows")]
fn set_no_window(command: &mut Command) {
    command.creation_flags(CREATE_NO_WINDOW);
}

const STARTUP_RETRY_FLOOR_MS: u64 = 1_000;
const DAEMON_BIN_NAME: &str = "ntm-tracker-daemon";

pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            if ensure_daemon(&app).await {
                break;
            }

            let delay_ms = next_retry_delay(&app).unwrap_or(STARTUP_RETRY_FLOOR_MS);
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
    });
}

pub fn shutdown(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let mut guard = match state.0.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(manager) = guard.manager.as_ref() {
        let _ = manager.stop();
    }
    guard.manager = None;
}

async fn ensure_daemon(app: &AppHandle) -> bool {
    let (transport, wsl_distro, already_running) = {
        let state = app.state::<AppState>();
        let guard = match state.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let already_running = guard
            .manager
            .as_ref()
            .map(|manager| manager.is_running())
            .unwrap_or(false);
        (
            guard.settings.transport.clone(),
            guard.settings.wsl_distro.clone(),
            already_running,
        )
    };

    if already_running {
        return true;
    }

    #[cfg(target_os = "windows")]
    if transport == "wsl-stdio" {
        if let Err(err) = ensure_wsl_daemon_installed(wsl_distro.as_deref()) {
            let state = app.state::<AppState>();
            let mut guard = match state.0.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            guard.last_error = Some(err);
            return false;
        }
    }

    let start_result = tauri::async_runtime::spawn_blocking(move || {
        DaemonManager::start(&transport, wsl_distro.as_deref())
    })
    .await
    .map_err(|err| format!("daemon start task failed: {err}"))
    .and_then(|result| result);

    let state = app.state::<AppState>();
    let mut guard = match state.0.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };

    match start_result {
        Ok(manager) => {
            let healthy = manager.is_running();
            guard.manager = Some(manager);
            guard.last_error = if healthy {
                None
            } else {
                Some("daemon failed health check after start".to_string())
            };
            healthy
        }
        Err(err) => {
            guard.last_error = Some(err);
            false
        }
    }
}

#[cfg(target_os = "windows")]
fn ensure_wsl_daemon_installed(wsl_distro: Option<&str>) -> Result<(), String> {
    // If the daemon is already on PATH inside WSL, only ensure directories exist.
    let version = env!("CARGO_PKG_VERSION");
    let release_tag = format!("v{version}");
    let download_url = format!(
        "https://github.com/Al2800/ntm-tracker-2/releases/download/{release_tag}/{DAEMON_BIN_NAME}-x86_64-unknown-linux-gnu"
    );

    let script = format!(
        r#"set -euo pipefail

BIN="$HOME/.local/bin/{bin}"
CONFIG_DIR="$HOME/.config/ntm-tracker"
DATA_DIR="$HOME/.local/share/ntm-tracker"

mkdir -p "$(dirname "$BIN")" "$CONFIG_DIR" "$DATA_DIR"

if command -v "{bin}" >/dev/null 2>&1; then
  exit 0
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required inside WSL to download {bin} (missing)" >&2
  exit 2
fi

tmp="$(mktemp)"
curl -fL "{url}" -o "$tmp"
chmod +x "$tmp"
mv "$tmp" "$BIN"

"$BIN" --version >/dev/null
"#,
        bin = DAEMON_BIN_NAME,
        url = download_url
    );

    let mut cmd = Command::new("wsl.exe");
    set_no_window(&mut cmd);
    if let Some(distro) = wsl_distro {
        if !distro.trim().is_empty() {
            cmd.args(["-d", distro]);
        }
    }
    let output = cmd
        .args(["--", "sh", "-lc", &script])
        .output()
        .map_err(|err| format!("Unable to run WSL bootstrap: {err}"))?;

    if !output.status.success() {
        return Err(format!(
            "WSL daemon bootstrap failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn ensure_wsl_daemon_installed(_wsl_distro: Option<&str>) -> Result<(), String> {
    Ok(())
}

fn next_retry_delay(app: &AppHandle) -> Option<u64> {
    let state = app.state::<AppState>();
    let guard = state.0.lock().ok()?;
    Some(
        guard
            .settings
            .reconnect_interval_ms
            .max(STARTUP_RETRY_FLOOR_MS),
    )
}
