use std::sync::Mutex;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

const TRAY_ID: &str = "ntm-tracker-tray";
const MENU_STATUS: &str = "tray_status";
const MENU_SESSIONS_PLACEHOLDER: &str = "tray_sessions_placeholder";
const MENU_OPEN_DASHBOARD: &str = "tray_open_dashboard";
const MENU_SNOOZE_NOTIFICATIONS: &str = "tray_snooze_notifications";
const MENU_SETTINGS: &str = "tray_settings";
const MENU_QUIT: &str = "tray_quit";

#[derive(Clone, Debug)]
pub struct TraySummary {
    pub sessions: u32,
    pub panes: u32,
    pub active_panes: u32,
    pub compacts_today: u32,
    pub usage_hours: f32,
    pub last_update_secs: u64,
    pub connection: String,
}

impl Default for TraySummary {
    fn default() -> Self {
        Self {
            sessions: 0,
            panes: 0,
            active_panes: 0,
            compacts_today: 0,
            usage_hours: 0.0,
            last_update_secs: 0,
            connection: "disconnected".to_string(),
        }
    }
}

impl TraySummary {
    fn status_line(&self) -> String {
        format!(
            "{} sessions, {} panes active",
            self.sessions, self.active_panes
        )
    }

    fn tooltip(&self) -> String {
        format!(
            "{} sessions, {} panes active\n{} compacts today | {:.1}h usage\ndaemon: {} | last: {}s",
            self.sessions,
            self.active_panes,
            self.compacts_today,
            self.usage_hours,
            self.connection,
            self.last_update_secs
        )
    }
}

pub struct TrayState {
    tray: TrayIcon,
    status_item: MenuItem,
    summary: Mutex<TraySummary>,
}

impl TrayState {
    pub fn update(&self, summary: TraySummary) -> tauri::Result<()> {
        self.status_item.set_text(summary.status_line())?;
        self.tray.set_tooltip(Some(summary.tooltip()))?;
        let _ = self.tray.set_title(Some(summary.sessions.to_string()));
        let mut guard = match self.summary.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        *guard = summary;
        Ok(())
    }
}

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let menu = Menu::new(app)?;
    let status_item = MenuItem::with_id(app, MENU_STATUS, "0 sessions", false, None::<&str>)?;
    menu.append(&status_item)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;

    let sessions_menu = Submenu::new(app, "Sessions", true)?;
    let placeholder =
        MenuItem::with_id(app, MENU_SESSIONS_PLACEHOLDER, "No sessions", false, None::<&str>)?;
    sessions_menu.append(&placeholder)?;
    menu.append(&sessions_menu)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;

    let open_dashboard =
        MenuItem::with_id(app, MENU_OPEN_DASHBOARD, "Open Dashboard", true, None::<&str>)?;
    let snooze = MenuItem::with_id(
        app,
        MENU_SNOOZE_NOTIFICATIONS,
        "Snooze Notifications",
        true,
        None::<&str>,
    )?;
    let settings = MenuItem::with_id(app, MENU_SETTINGS, "Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit", true, None::<&str>)?;

    menu.append(&open_dashboard)?;
    menu.append(&snooze)?;
    menu.append(&settings)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&quit)?;

    let mut tray_builder = TrayIconBuilder::with_id(TRAY_ID).menu(&menu).tooltip("NTM Tracker");
    if let Some(icon) = app.default_window_icon().cloned() {
        tray_builder = tray_builder.icon(icon);
    }

    let tray = tray_builder
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_OPEN_DASHBOARD => show_main_window(app),
            MENU_SNOOZE_NOTIFICATIONS => {
                let _ = app.emit_all("tray:snooze", ());
            }
            MENU_SETTINGS => {
                show_main_window(app);
                let _ = app.emit_all("tray:open-settings", ());
            }
            MENU_QUIT => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::DoubleClick { .. } => show_main_window(tray.app_handle()),
            TrayIconEvent::Click {
                button,
                button_state,
                ..
            } => {
                if matches!(button, MouseButton::Left) && matches!(button_state, MouseButtonState::Up)
                {
                    show_main_window(tray.app_handle());
                }
            }
            _ => {}
        })
        .build(app)?;

    let state = TrayState {
        tray,
        status_item,
        summary: Mutex::new(TraySummary::default()),
    };
    state.update(TraySummary::default())?;
    app.manage(state);
    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}
