use crate::commands::AppState;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

const TRAY_ID: &str = "ntm-tracker-tray";
const MENU_STATUS: &str = "tray_status";
const MENU_SESSIONS_MENU: &str = "tray_sessions_menu";
const MENU_OPEN_DASHBOARD: &str = "tray_open_dashboard";
const MENU_SNOOZE_NOTIFICATIONS: &str = "tray_snooze_notifications";
const MENU_SETTINGS: &str = "tray_settings";
const MENU_QUIT: &str = "tray_quit";
const MENU_SESSION_PREFIX: &str = "tray_session:";

const UPDATE_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Clone, Debug)]
pub struct TraySummary {
    pub sessions: u32,
    pub panes: u32,
    pub active_panes: u32,
    pub alerts: u32,
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
            alerts: 0,
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
        let alert_line = if self.alerts > 0 {
            format!("\n{} alert(s)", self.alerts)
        } else {
            String::new()
        };
        format!(
            "{} sessions, {} panes active{}\\n{} compacts today | {:.1}h usage\\ndaemon: {} | last: {}s",
            self.sessions,
            self.active_panes,
            alert_line,
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
    sessions_menu: Submenu,
    summary: Mutex<TraySummary>,
}

impl TrayState {
    pub fn update(&self, summary: TraySummary, sessions: Vec<SessionEntry>) -> tauri::Result<()> {
        self.status_item.set_text(summary.status_line())?;
        self.tray.set_tooltip(Some(summary.tooltip()))?;
        let _ = self.tray.set_title(Some(summary.sessions.to_string()));
        let _ = self.tray.set_icon(Some(render_tray_icon(&summary)));
        self.replace_sessions_menu(sessions)?;
        let mut guard = match self.summary.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        *guard = summary;
        Ok(())
    }

    fn replace_sessions_menu(&self, sessions: Vec<SessionEntry>) -> tauri::Result<()> {
        // Clear existing items.
        while let Some(_item) = self.sessions_menu.remove_at(0)? {}

        if sessions.is_empty() {
            let placeholder = MenuItem::with_id(
                self.tray.app_handle(),
                "tray_sessions_empty",
                "No sessions",
                false,
                None::<&str>,
            )?;
            self.sessions_menu.append(&placeholder)?;
            return Ok(());
        }

        for session in sessions.into_iter().take(20) {
            let label = format!("{} ({})", session.name, session.status);
            let id = format!("{MENU_SESSION_PREFIX}{}", session.session_id);
            let item = MenuItem::with_id(self.tray.app_handle(), id, label, true, None::<&str>)?;
            self.sessions_menu.append(&item)?;
        }

        Ok(())
    }
}

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let menu = Menu::new(app)?;
    let status_item = MenuItem::with_id(app, MENU_STATUS, "0 sessions", false, None::<&str>)?;
    menu.append(&status_item)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;

    let sessions_menu = Submenu::with_id(app, MENU_SESSIONS_MENU, "Sessions", true)?;
    let placeholder = MenuItem::with_id(
        app,
        "tray_sessions_empty",
        "No sessions",
        false,
        None::<&str>,
    )?;
    sessions_menu.append(&placeholder)?;
    menu.append(&sessions_menu)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;

    let open_dashboard = MenuItem::with_id(
        app,
        MENU_OPEN_DASHBOARD,
        "Open Dashboard",
        true,
        None::<&str>,
    )?;
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

    let mut tray_builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip("NTM Tracker");
    if let Some(icon) = app.default_window_icon().cloned() {
        tray_builder = tray_builder.icon(icon);
    }

    let tray = tray_builder
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            if let Some(session_id) = id.strip_prefix(MENU_SESSION_PREFIX) {
                show_main_window(app);
                let _ = app.emit_all("tray:open-session", session_id.to_string());
                return;
            }

            match id {
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
            }
        })
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::DoubleClick { .. } => show_main_window(tray.app_handle()),
            TrayIconEvent::Click {
                button,
                button_state,
                ..
            } => {
                if matches!(button, MouseButton::Left)
                    && matches!(button_state, MouseButtonState::Up)
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
        sessions_menu,
        summary: Mutex::new(TraySummary::default()),
    };
    state.update(TraySummary::default(), Vec::new())?;
    app.manage(state);
    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[derive(Clone, Debug)]
pub struct SessionEntry {
    pub session_id: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotStats {
    total_compacts: u64,
    active_minutes: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotSession {
    session_id: String,
    name: String,
    status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotPane {
    status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotEvent {
    event_type: String,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotResponse {
    sessions: Vec<SnapshotSession>,
    panes: Vec<SnapshotPane>,
    events: Vec<SnapshotEvent>,
    stats: SnapshotStats,
}

pub fn spawn_updater(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let started = Instant::now();
        loop {
            let app_clone = app.clone();
            let fetched = tauri::async_runtime::spawn_blocking(move || fetch_summary(&app_clone))
                .await
                .ok()
                .flatten();

            if let Some(tray) = app.try_state::<TrayState>() {
                if let Some((summary, sessions)) = fetched {
                    let _ = tray.update(summary, sessions);
                } else {
                    let summary = TraySummary {
                        connection: "stopped".to_string(),
                        last_update_secs: started.elapsed().as_secs(),
                        ..TraySummary::default()
                    };
                    let _ = tray.update(summary, Vec::new());
                }
            }

            tokio::time::sleep(UPDATE_INTERVAL).await;
        }
    });
}

fn fetch_summary(app: &AppHandle) -> Option<(TraySummary, Vec<SessionEntry>)> {
    let state = app.try_state::<AppState>()?;
    let mut guard = state.0.lock().ok()?;

    let Some(manager) = guard.manager.as_ref() else {
        return None;
    };
    if !manager.is_running() {
        return None;
    }

    let timeout = Duration::from_secs(2);
    let snapshot = manager
        .call("snapshot.get".to_string(), Value::Null, timeout)
        .ok()?;
    let parsed: SnapshotResponse = serde_json::from_value(snapshot).ok()?;

    let active_panes = parsed
        .panes
        .iter()
        .filter(|pane| pane.status == "active")
        .count() as u32;

    let sessions: Vec<SessionEntry> = parsed
        .sessions
        .iter()
        .map(|session| SessionEntry {
            session_id: session.session_id.clone(),
            name: session.name.clone(),
            status: session.status.clone(),
        })
        .collect();

    let alerts = parsed
        .events
        .iter()
        .filter(|event| event.event_type == "escalation")
        .filter(|event| event.status.as_deref() != Some("dismissed"))
        .count() as u32;

    let summary = TraySummary {
        sessions: sessions.len() as u32,
        panes: parsed.panes.len() as u32,
        active_panes,
        alerts,
        compacts_today: parsed.stats.total_compacts as u32,
        usage_hours: (parsed.stats.active_minutes as f32) / 60.0,
        last_update_secs: 0,
        connection: if alerts > 0 {
            "alert".to_string()
        } else {
            "ok".to_string()
        },
    };

    Some((summary, sessions))
}

fn render_tray_icon(summary: &TraySummary) -> Image<'static> {
    const SIZE: u32 = 32;
    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];

    let base = if matches!(summary.connection.as_str(), "disconnected" | "stopped") {
        [120, 120, 120, 255]
    } else if summary.connection == "alert" {
        [220, 60, 50, 255]
    } else {
        [45, 125, 179, 255]
    };

    fill_circle(&mut rgba, SIZE, 16, 16, 14, base);

    if summary.alerts > 0 {
        fill_circle(&mut rgba, SIZE, 24, 8, 7, [220, 60, 50, 255]);
        draw_exclamation(&mut rgba, SIZE, 22, 3, [255, 255, 255, 255]);
    } else if summary.sessions > 0
        && !matches!(summary.connection.as_str(), "disconnected" | "stopped")
    {
        fill_circle(&mut rgba, SIZE, 24, 8, 7, [40, 170, 90, 255]);
        let shown = summary.sessions.min(99);
        draw_number(&mut rgba, SIZE, shown, 20, 5, [255, 255, 255, 255]);
    }

    Image::new_owned(rgba, SIZE, SIZE)
}

fn fill_circle(rgba: &mut [u8], size: u32, cx: i32, cy: i32, r: i32, color: [u8; 4]) {
    let r2 = r * r;
    for y in 0..(size as i32) {
        for x in 0..(size as i32) {
            let dx = x - cx;
            let dy = y - cy;
            if dx * dx + dy * dy <= r2 {
                set_px(rgba, size, x, y, color);
            }
        }
    }
}

fn set_px(rgba: &mut [u8], size: u32, x: i32, y: i32, color: [u8; 4]) {
    if x < 0 || y < 0 || x >= size as i32 || y >= size as i32 {
        return;
    }
    let idx = ((y as u32 * size + x as u32) * 4) as usize;
    rgba[idx..idx + 4].copy_from_slice(&color);
}

fn draw_number(rgba: &mut [u8], size: u32, value: u32, x: i32, y: i32, color: [u8; 4]) {
    let s = value.to_string();
    let digits: Vec<u32> = s.chars().filter_map(|ch| ch.to_digit(10)).collect();
    let start_x = match digits.len() {
        1 => x + 4,
        _ => x,
    };
    for (i, digit) in digits.into_iter().take(2).enumerate() {
        draw_digit(rgba, size, digit, start_x + (i as i32 * 4), y, color);
    }
}

fn draw_exclamation(rgba: &mut [u8], size: u32, x: i32, y: i32, color: [u8; 4]) {
    for dy in 0..6 {
        set_px(rgba, size, x + 1, y + dy, color);
        set_px(rgba, size, x + 2, y + dy, color);
    }
    set_px(rgba, size, x + 1, y + 7, color);
    set_px(rgba, size, x + 2, y + 7, color);
}

fn draw_digit(rgba: &mut [u8], size: u32, digit: u32, x: i32, y: i32, color: [u8; 4]) {
    let bitmap: [u8; 15] = match digit {
        0 => [1, 1, 1, 1, 0, 1, 1, 0, 1, 1, 0, 1, 1, 1, 1],
        1 => [0, 1, 0, 1, 1, 0, 0, 1, 0, 0, 1, 0, 1, 1, 1],
        2 => [1, 1, 1, 0, 0, 1, 1, 1, 1, 1, 0, 0, 1, 1, 1],
        3 => [1, 1, 1, 0, 0, 1, 1, 1, 1, 0, 0, 1, 1, 1, 1],
        4 => [1, 0, 1, 1, 0, 1, 1, 1, 1, 0, 0, 1, 0, 0, 1],
        5 => [1, 1, 1, 1, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 1],
        6 => [1, 1, 1, 1, 0, 0, 1, 1, 1, 1, 0, 1, 1, 1, 1],
        7 => [1, 1, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1],
        8 => [1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1],
        9 => [1, 1, 1, 1, 0, 1, 1, 1, 1, 0, 0, 1, 1, 1, 1],
        _ => [0; 15],
    };

    for row in 0..5 {
        for col in 0..3 {
            if bitmap[(row * 3 + col) as usize] != 0 {
                set_px(rgba, size, x + col as i32, y + row as i32, color);
            }
        }
    }
}
