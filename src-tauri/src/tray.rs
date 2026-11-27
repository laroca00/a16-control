use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

use crate::commands::lang;
use crate::fan::{self, FanMode};
use crate::power::UiMode;
use crate::state::{self, AppState};
use crate::telemetry::Telemetry;

const TRAY_ID: &str = "main";

pub struct TrayItems {
    open: MenuItem<Wry>,
    fan: CheckMenuItem<Wry>,
    power: Submenu<Wry>,
    lights: CheckMenuItem<Wry>,
    modes: [(UiMode, CheckMenuItem<Wry>); 3],
    quit: MenuItem<Wry>,
    /// When the panel last hid itself on blur; the click that caused the
    /// blur must not immediately re-open it.
    panel_hidden_at: Mutex<Option<Instant>>,
    /// Whether the icon now carries the Max Fan dot, so it is only swapped on a change.
    dot_shown: Mutex<Option<bool>>,
    /// Last tooltip written, for the same reason.
    tooltip: Mutex<String>,
}

struct Labels {
    open: &'static str,
    fan: &'static str,
    power: &'static str,
    modes: [&'static str; 3],
    lights: &'static str,
    quit: &'static str,
}

fn labels(lang: &str) -> Labels {
    match lang {
        "tr" => Labels { open: "Aç", fan: "Max Fan", power: "Güç modu", modes: ["Pil", "Otomatik", "Performans"], lights: "Klavye ışığı", quit: "Çıkış" },
        _ => Labels { open: "Open", fan: "Max Fan", power: "Power mode", modes: ["Battery", "Auto", "Performance"], lights: "Keyboard light", quit: "Exit" },
    }
}

const MODE_IDS: [(UiMode, &str); 3] =
    [(UiMode::Battery, "mode-battery"), (UiMode::Auto, "mode-auto"), (UiMode::Performance, "mode-performance")];

pub fn build(app: &AppHandle) -> tauri::Result<TrayIcon> {
    let l = labels(&lang(app));
    let current = app.state::<AppState>().power.lock().unwrap().controller.mode();
    let open = MenuItem::with_id(app, "open", l.open, true, None::<&str>)?;
    let fan = CheckMenuItem::with_id(app, "fan", l.fan, true, false, None::<&str>)?;
    let item = |i: usize| -> tauri::Result<(UiMode, CheckMenuItem<Wry>)> {
        let (mode, id) = MODE_IDS[i];
        Ok((mode, CheckMenuItem::with_id(app, id, l.modes[i], true, mode == current, None::<&str>)?))
    };
    let modes = [item(0)?, item(1)?, item(2)?];
    let power = Submenu::with_items(app, l.power, true, &[&modes[0].1, &modes[1].1, &modes[2].1])?;
    let lit = app.state::<crate::keyboard_ctl::KeyboardCtl>().settings().brightness > 0;
    let lights = CheckMenuItem::with_id(app, "lights", l.lights, true, lit, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", l.quit, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &fan, &power, &lights, &quit])?;
    app.manage(TrayItems {
        open,
        fan,
        power,
        lights,
        modes,
        quit,
        panel_hidden_at: Mutex::new(None),
        dot_shown: Mutex::new(None),
        tooltip: Mutex::new(String::new()),
    });

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(app.default_window_icon().cloned().expect("bundled icon"))
        .tooltip("GiENEMY")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "fan" => {
                toggle_fan(app);
            }
            "quit" => self::quit(app),
            "lights" => {
                crate::keyboard_ctl::update(app, crate::keyboard::toggle_lights);
            }
            id => {
                if let Some((mode, _)) = MODE_IDS.iter().find(|(_, mid)| *mid == id) {
                    set_mode_from_tray(app, *mode);
                }
            }
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
    let l = labels(&lang(app));
    let _ = items.open.set_text(l.open);
    let _ = items.fan.set_text(l.fan);
    let _ = items.power.set_text(l.power);
    let _ = items.lights.set_text(l.lights);
    for (i, (_, item)) in items.modes.iter().enumerate() {
        let _ = item.set_text(l.modes[i]);
    }
    let _ = items.quit.set_text(l.quit);
}

pub fn sync_fan_check(app: &AppHandle, t: Option<&Telemetry>) {
    let max = matches!(t, Some(t) if t.fan_mode == FanMode::Max);
    if let Some(items) = app.try_state::<TrayItems>() {
        let _ = items.fan.set_checked(max);
        let mut shown = items.dot_shown.lock().unwrap();
        if *shown != Some(max) {
            *shown = Some(max);
            set_icon(app, max);
        }
    }
}

/// Bole, the colour the app gives Max Fan.
const DOT: [u8; 3] = [0xd2, 0x41, 0x2f];

fn set_icon(app: &AppHandle, max: bool) {
    let (Some(tray), Some(base)) = (app.tray_by_id(TRAY_ID), app.default_window_icon()) else { return };
    let icon = if max {
        Image::new_owned(dotted(base.rgba(), base.width(), base.height(), DOT), base.width(), base.height())
    } else {
        base.clone().to_owned()
    };
    let _ = tray.set_icon(Some(icon));
}

/// The icon with a filled dot in its lower right corner, ringed in near
/// black so it reads on light and dark taskbars.
pub fn dotted(rgba: &[u8], width: u32, height: u32, color: [u8; 3]) -> Vec<u8> {
    let mut out = rgba.to_vec();
    let size = width.min(height) as f32;
    let r = size * 0.24;
    let ring = (size * 0.06).max(1.0);
    let (cx, cy) = (width as f32 - r - ring, height as f32 - r - ring);
    for y in 0..height {
        for x in 0..width {
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            let px = if d <= r {
                [color[0], color[1], color[2], 255]
            } else if d <= r + ring {
                [16, 16, 18, 255]
            } else {
                continue;
            };
            let i = ((y * width + x) * 4) as usize;
            out[i..i + 4].copy_from_slice(&px);
        }
    }
    out
}

/// Hover text: the temperatures and fan mode, so a glance at the tray is enough.
pub fn tooltip_text(lang: &str, t: Option<&Telemetry>) -> String {
    let Some(t) = t else { return "GiENEMY".into() };
    let mode = match (lang, t.fan_mode) {
        (_, FanMode::Max) => "Max Fan",
        ("tr", FanMode::Auto) => "Otomatik",
        ("tr", FanMode::Quiet) => "Sessiz",
        ("tr", FanMode::Curve) => "Eğri",
        (_, FanMode::Curve) => "Curve",
        (_, FanMode::Auto) => "Auto",
        (_, FanMode::Quiet) => "Quiet",
    };
    format!("GiENEMY
CPU {}°C · GPU {}°C
Fan: {mode}", t.cpu_temp, t.gpu_temp)
}

pub fn sync_tooltip(app: &AppHandle, t: Option<&Telemetry>) {
    let Some(items) = app.try_state::<TrayItems>() else { return };
    let text = tooltip_text(&lang(app), t);
    let mut last = items.tooltip.lock().unwrap();
    if *last != text {
        if let Some(tray) = app.tray_by_id(TRAY_ID) {
            let _ = tray.set_tooltip(Some(&text));
        }
        *last = text;
    }
}

pub fn sync_lights(app: &AppHandle, on: bool) {
    if let Some(items) = app.try_state::<TrayItems>() {
        let _ = items.lights.set_checked(on);
    }
}

pub fn sync_power(app: &AppHandle, mode: UiMode) {
    if let Some(items) = app.try_state::<TrayItems>() {
        for (m, item) in &items.modes {
            let _ = item.set_checked(*m == mode);
        }
    }
}

pub fn set_mode_from_tray(app: &AppHandle, mode: UiMode) {
    let app = app.clone();
    // Applying a mode spawns processes; keep the menu thread free.
    std::thread::spawn(move || {
        app.state::<AppState>().choose_mode(mode);
        crate::power_tick(&app);
        sync_power(&app, mode);
    });
}

/// Flips Max Fan and returns the mode the fans are in afterwards, if known.
pub fn toggle_fan(app: &AppHandle) -> Option<FanMode> {
    let st = app.state::<AppState>();
    let next = match st.telemetry() {
        Ok(t) if t.fan_mode == FanMode::Max => FanMode::Auto,
        _ => FanMode::Max,
    };
    let t = state::set_fan(&st, next).ok();
    sync_fan_check(app, t.as_ref());
    t.map(|t| t.fan_mode)
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

/// A hidden panel is closed after this long: reopening it costs a fresh
/// webview, keeping it costs a renderer process.
const PANEL_KEEP: Duration = Duration::from_secs(20);

/// The main window and the panel are built when first shown and closed once
/// hidden, so the tray app holds no WebView2 processes while nobody looks.
fn window(app: &AppHandle, label: &str) -> Option<tauri::WebviewWindow> {
    if let Some(w) = app.get_webview_window(label) {
        return Some(w);
    }
    let config = app.config().app.windows.iter().find(|w| w.label == label)?;
    match tauri::WebviewWindowBuilder::from_config(app, config).and_then(|b| b.build()) {
        Ok(w) => Some(w),
        Err(e) => {
            eprintln!("could not open the {label} window: {e}");
            None
        }
    }
}

pub fn reap_panel(app: &AppHandle) {
    let Some(w) = app.get_webview_window("panel") else { return };
    if w.is_visible().unwrap_or(true) {
        return;
    }
    let hidden_at = *app.state::<TrayItems>().panel_hidden_at.lock().unwrap();
    if hidden_at.map_or(true, |t| t.elapsed() >= PANEL_KEEP) {
        let _ = w.close();
    }
}

fn toggle_panel(app: &AppHandle, anchor: (i32, i32)) {
    let Some(w) = window(app, "panel") else { return };
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
    if let Some(w) = window(app, "main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

pub fn quit(app: &AppHandle) {
    crate::battery_ctl::release_all(app);
    crate::hardware::release(app);
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

    fn telemetry(mode: FanMode) -> Telemetry {
        Telemetry { rpm1: 4000, rpm2: 4100, cpu_temp: 71, gpu_temp: 64, cpu_duty: 50, gpu_duty: 50, fan_mode: mode }
    }

    #[test]
    fn tooltip_names_temperatures_and_mode() {
        assert_eq!(tooltip_text("tr", Some(&telemetry(FanMode::Auto))), "GiENEMY
CPU 71°C · GPU 64°C
Fan: Otomatik");
        assert_eq!(tooltip_text("en", Some(&telemetry(FanMode::Max))), "GiENEMY
CPU 71°C · GPU 64°C
Fan: Max Fan");
        assert_eq!(tooltip_text("en", None), "GiENEMY");
    }

    #[test]
    fn dot_paints_the_corner_and_leaves_the_rest() {
        let (w, h) = (32, 32);
        let base = vec![0u8; (w * h * 4) as usize];
        let out = dotted(&base, w, h, DOT);
        let px = |x: u32, y: u32| &out[((y * w + x) * 4) as usize..((y * w + x) * 4 + 4) as usize];
        assert_eq!(px(24, 24), &[DOT[0], DOT[1], DOT[2], 255]);
        assert_eq!(px(2, 2), &[0, 0, 0, 0]);
        assert_eq!(px(16, 4), &[0, 0, 0, 0]);
    }

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
