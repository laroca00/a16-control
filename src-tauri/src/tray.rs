use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::menu::{CheckMenuItem, Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_positioner::{Position, WindowExt};

use crate::commands::lang;
use crate::fan::FanMode;
use crate::state::{self, AppState};
use crate::telemetry::Telemetry;

const TRAY_ID: &str = "main";

pub struct TrayItems {
    open: MenuItem<Wry>,
    fan: CheckMenuItem<Wry>,
    quit: MenuItem<Wry>,
    /// When the panel last hid itself on blur; the click that caused the
    /// blur must not immediately re-open it.
    panel_hidden_at: Mutex<Option<Instant>>,
}

fn labels(lang: &str) -> (&'static str, &'static str, &'static str) {
    match lang {
        "tr" => ("Aç", "Max Fan", "Çıkış"),
        _ => ("Open", "Max Fan", "Exit"),
    }
}

pub fn build(app: &AppHandle) -> tauri::Result<TrayIcon> {
    let (open_l, fan_l, quit_l) = labels(&lang(app));
    let open = MenuItem::with_id(app, "open", open_l, true, None::<&str>)?;
    let fan = CheckMenuItem::with_id(app, "fan", fan_l, true, false, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", quit_l, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &fan, &quit])?;
    app.manage(TrayItems { open, fan, quit, panel_hidden_at: Mutex::new(None) });

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(app.default_window_icon().cloned().expect("bundled icon"))
        .tooltip("GiENEMY")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "fan" => toggle_fan(app),
            "quit" => self::quit(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                toggle_panel(tray.app_handle());
            }
        })
        .build(app)
}

pub fn relabel(app: &AppHandle) {
    let items = app.state::<TrayItems>();
    let (open_l, fan_l, quit_l) = labels(&lang(app));
    let _ = items.open.set_text(open_l);
    let _ = items.fan.set_text(fan_l);
    let _ = items.quit.set_text(quit_l);
}

pub fn sync_fan_check(app: &AppHandle, t: Option<&Telemetry>) {
    if let Some(items) = app.try_state::<TrayItems>() {
        let _ = items.fan.set_checked(matches!(t, Some(t) if t.fan_mode == FanMode::Max));
    }
}

fn toggle_fan(app: &AppHandle) {
    let st = app.state::<AppState>();
    let next = match st.telemetry() {
        Ok(t) if t.fan_mode == FanMode::Max => FanMode::Auto,
        _ => FanMode::Max,
    };
    let t = state::set_fan(&st, next).ok();
    sync_fan_check(app, t.as_ref());
}

pub fn panel_blurred(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("panel") {
        let _ = w.hide();
        *app.state::<TrayItems>().panel_hidden_at.lock().unwrap() = Some(Instant::now());
    }
}

fn toggle_panel(app: &AppHandle) {
    let Some(w) = app.get_webview_window("panel") else { return };
    let just_hidden = app
        .state::<TrayItems>()
        .panel_hidden_at
        .lock()
        .unwrap()
        .is_some_and(|t| t.elapsed() < Duration::from_millis(300));
    if w.is_visible().unwrap_or(false) || just_hidden {
        let _ = w.hide();
        return;
    }
    let _ = w.move_window(Position::TrayBottomCenter);
    let _ = w.show();
    let _ = w.set_focus();
}

pub fn show_main(app: &AppHandle) {
    if let Some(p) = app.get_webview_window("panel") {
        let _ = p.hide();
    }
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

pub fn quit(app: &AppHandle) {
    let st = app.state::<AppState>();
    let t = st.telemetry().ok();
    if let Err(e) = state::restore_before_exit(st.wmi.as_deref(), t.as_ref()) {
        eprintln!("could not restore fans before exit: {e}");
    }
    app.exit(0);
}
