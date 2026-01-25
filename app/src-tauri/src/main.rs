#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bootstrap;
mod commands;
mod daemon;
mod transport;
mod tray;

use commands::{
    daemon_health, daemon_start, daemon_stop, export_diagnostics, get_attach_command, get_settings,
    load_settings, rpc_call, set_settings, AppState,
};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let settings = load_settings(app.handle());
            app.manage(AppState::new(settings));
            tray::init(app.handle())?;
            tray::spawn_updater(app.handle().clone());
            bootstrap::start(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            daemon_start,
            daemon_stop,
            daemon_health,
            rpc_call,
            get_settings,
            set_settings,
            export_diagnostics,
            get_attach_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
