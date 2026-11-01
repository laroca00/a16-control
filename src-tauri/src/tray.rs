use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::menu::{CheckMenuItem, Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

use crate::commands::lang;
use crate::fan::{self, FanMode};
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
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                position,
                ..
            } = event
            {
                toggle_panel(tray.app_handle(), (position.x as i32, position.y as i32));
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Where the panel goes for a click at `anchor`. Taskbar replacements and
/// docks report the tray rectangle unreliably, so this works from the click
/// point: above it, or below it for a bar at the top, kept inside the
/// monitor's work area.
pub fn panel_position(anchor: (i32, i32), size: (i32, i32), work: Area, margin: i32) -> (i32, i32) {
    let (w, h) = size;
    let min_x = work.x + margin;
    let max_x = (work.x + work.w - w - margin).max(min_x);
    let min_y = work.y + margin;
    let max_y = (work.y + work.h - h - margin).max(min_y);
    let mut y = anchor.1 - h - margin;
    if y < min_y {
        y = anchor.1 + margin;
    }
    ((anchor.0 - w / 2).clamp(min_x, max_x), y.clamp(min_y, max_y))
}

fn place_panel(app: &AppHandle, w: &tauri::WebviewWindow, anchor: (i32, i32)) {
    let Ok(size) = w.outer_size() else { return };
    let monitor = app
        .monitor_from_point(anchor.0 as f64, anchor.1 as f64)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else { return };
    let r = monitor.work_area();
    let work = Area { x: r.position.x, y: r.position.y, w: r.size.width as i32, h: r.size.height as i32 };
    let margin = (12.0 * monitor.scale_factor()) as i32;
    let (x, y) = panel_position(anchor, (size.width as i32, size.height as i32), work, margin);
    let _ = w.set_position(tauri::PhysicalPosition::new(x, y));
}

fn toggle_panel(app: &AppHandle, anchor: (i32, i32)) {
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
    place_panel(app, &w, anchor);
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
    if let Ok(wmi) = app.state::<AppState>().wmi() {
        if let Err(e) = fan::release(wmi.as_ref()) {
            eprintln!("could not restore fans before exit: {e}");
        }
    }
    app.exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Area = Area { x: 0, y: 0, w: 1920, h: 1032 };

    #[test]
    fn panel_sits_above_the_click_inside_the_work_area() {
        assert_eq!(panel_position((1800, 1060), (360, 440), SCREEN, 12), (1548, 580));
    }

    #[test]
    fn panel_stays_above_a_dock_that_reserves_no_space() {
        let full = Area { x: 0, y: 0, w: 1920, h: 1080 };
        assert_eq!(panel_position((960, 1070), (360, 440), full, 12), (780, 618));
    }

    #[test]
    fn panel_drops_below_a_top_bar() {
        assert_eq!(panel_position((100, 10), (360, 440), Area { x: 0, y: 40, w: 1920, h: 1040 }, 12), (12, 52));
    }

    #[test]
    fn panel_respects_a_second_monitor_offset() {
        let right = Area { x: 1920, y: 0, w: 2560, h: 1400 };
        assert_eq!(panel_position((4400, 1430), (360, 440), right, 12), (4108, 948));
    }
}
