//! Panel brightness, refresh rate and the "screen off" blackout.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::os;
use crate::power::Refresh;

const STEP: u32 = 5;
// Ignore the click or key that turned the screen off.
const BLACKOUT_GRACE: Duration = Duration::from_millis(1500);
const BLACKOUT_LABEL: &str = "blackout";

pub fn step5(current: u32, up: bool) -> u32 {
    let c = current.min(100);
    let rem = c % STEP;
    let next = match (up, rem) {
        (true, 0) => c + STEP,
        (true, r) => c + (STEP - r),
        (false, 0) => c.saturating_sub(STEP),
        (false, r) => c - r,
    };
    next.min(100)
}

pub fn blackout_should_end(shown_at: Instant, last_input: Instant) -> bool {
    last_input > shown_at + BLACKOUT_GRACE
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DisplayView {
    pub brightness: Option<u32>,
    pub refresh: Option<u32>,
    pub rates: Vec<u32>,
    pub screen_off: bool,
}

#[derive(Default)]
pub struct DisplayState {
    blackout: Mutex<Option<(Instant, u32)>>,
}

pub fn view(app: &AppHandle) -> DisplayView {
    DisplayView {
        brightness: crate::hw::com::get_brightness().ok(),
        refresh: os::current_refresh(),
        rates: os::refresh_rates(),
        screen_off: app.state::<DisplayState>().blackout.lock().unwrap().is_some(),
    }
}

pub fn set_brightness(value: u32) -> bool {
    os::set_brightness(value.min(100))
}

pub fn set_refresh(hz: u32) -> bool {
    os::set_refresh(Refresh::Hz(hz))
}

/// Covers every screen in black and drops brightness to 0. Nothing sleeps
/// or locks; any input brings the screen back.
pub fn screen_off(app: &AppHandle) -> Result<(), String> {
    let previous = crate::hw::com::get_brightness().unwrap_or(50);
    {
        let state = app.state::<DisplayState>();
        let mut b = state.blackout.lock().unwrap();
        if b.is_some() {
            return Ok(());
        }
        *b = Some((Instant::now(), previous));
    }
    if let Some(w) = app.get_webview_window(BLACKOUT_LABEL) {
        let _ = w.show();
        let _ = w.set_focus();
    } else {
        WebviewWindowBuilder::new(app, BLACKOUT_LABEL, WebviewUrl::App("index.html".into()))
            .fullscreen(true)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(true)
            .build()
            .map_err(|e| e.to_string())?;
    }
    os::set_brightness(0);
    Ok(())
}

pub fn screen_on(app: &AppHandle) {
    let previous = app.state::<DisplayState>().blackout.lock().unwrap().take();
    if let Some(w) = app.get_webview_window(BLACKOUT_LABEL) {
        let _ = w.close();
    }
    if let Some((_, level)) = previous {
        os::set_brightness(level.max(STEP));
    }
}

/// Backstop for the blackout window's own input handlers: if it loses focus,
/// the system-wide last-input time still ends it.
pub fn watch(app: &AppHandle) {
    let shown = app.state::<DisplayState>().blackout.lock().unwrap().map(|(t, _)| t);
    if let Some(shown_at) = shown {
        let last_input = Instant::now().checked_sub(os::user_idle()).unwrap_or(shown_at);
        if blackout_should_end(shown_at, last_input) {
            screen_on(app);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step5_snaps_and_clamps() {
        assert_eq!(step5(42, true), 45);
        assert_eq!(step5(42, false), 40);
        assert_eq!(step5(45, true), 50);
        assert_eq!(step5(100, true), 100);
        assert_eq!(step5(0, false), 0);
    }

    #[test]
    fn input_after_blackout_wakes_screen() {
        let shown = Instant::now();
        assert!(!blackout_should_end(shown, shown + Duration::from_millis(500)));
        assert!(blackout_should_end(shown, shown + Duration::from_secs(2)));
        assert!(!blackout_should_end(shown, shown - Duration::from_secs(5)));
    }
}
