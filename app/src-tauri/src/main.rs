#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autostart;
mod bootstrap;
mod commands;
mod daemon;
mod transport;
mod tray;
mod upgrade;

use commands::{
    daemon_health, daemon_restart, daemon_start, daemon_stop, export_diagnostics,
    get_attach_command, get_settings, list_wsl_distros, load_settings, rpc_call, set_settings,
    AppState,
};
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let settings = load_settings(app.handle());
            let _ = autostart::set_enabled(settings.autostart_enabled);
            app.manage(AppState::new(settings));
            if std::env::args().any(|arg| arg == "--minimized") {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            tray::init(app.handle())?;
            tray::spawn_updater(app.handle().clone());
            upgrade::start(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            daemon_start,
            daemon_stop,
            daemon_restart,
            daemon_health,
            rpc_call,
            list_wsl_distros,
            get_settings,
            set_settings,
            export_diagnostics,
            get_attach_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
