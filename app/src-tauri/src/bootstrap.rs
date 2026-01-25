use crate::commands::AppState;
use crate::daemon::DaemonManager;
use std::time::Duration;
use tauri::{AppHandle, Manager};

const STARTUP_RETRY_FLOOR_MS: u64 = 1_000;

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
    if let Ok(state) = std::panic::catch_unwind(|| app.state::<AppState>()) {
        let mut guard = match state.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if let Some(manager) = guard.manager.as_ref() {
            let _ = manager.stop();
        }
        guard.manager = None;
    }
}

async fn ensure_daemon(app: &AppHandle) -> bool {
    let (transport, already_running) = {
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
        (guard.settings.transport.clone(), already_running)
    };

    if already_running {
        return true;
    }

    let start_result =
        tauri::async_runtime::spawn_blocking(move || DaemonManager::start(&transport))
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
