//! App side of the keyboard light: settings, the lamp worker, the rainbow
//! animation, periodic re-apply and the global brightness shortcuts.

use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};

use crate::config;
use crate::keyboard::{self, hue_rgb, KeyboardSettings, KeyboardView, Lamp};
use crate::state::AppState;

const RAINBOW_TICK: Duration = Duration::from_millis(40);
const RAINBOW_STEP_DEG: f32 = 2.0;
// Firmware shortcuts or a driver blip can take the lamp back; re-assert it.
const REAPPLY_EVERY: Duration = Duration::from_secs(300);

pub struct KeyboardCtl {
    lamp: Lamp,
    settings: Mutex<KeyboardSettings>,
    last_on: Mutex<u8>,
}

impl KeyboardCtl {
    pub fn new(settings: KeyboardSettings) -> Self {
        KeyboardCtl {
            lamp: Lamp::start(crate::hw::hid::open_lamp),
            settings: Mutex::new(settings),
            last_on: Mutex::new(settings.brightness.max(10)),
        }
    }

    pub fn settings(&self) -> KeyboardSettings {
        *self.settings.lock().unwrap()
    }

    pub fn view(&self) -> KeyboardView {
        keyboard::view(&self.settings())
    }

    /// Writes the static colour; the rainbow thread owns the lamp while it runs.
    pub fn apply(&self) {
        let s = self.settings();
        if !s.rainbow || s.brightness == 0 {
            self.lamp.set(s.rgb, s.brightness);
        }
    }
}

/// Changes the settings, writes them to the lamp and config, and tells the UI.
pub fn update(app: &AppHandle, change: impl FnOnce(&mut KeyboardSettings, &mut u8)) -> KeyboardView {
    let ctl = app.state::<KeyboardCtl>();
    {
        let mut s = ctl.settings.lock().unwrap();
        let mut last = ctl.last_on.lock().unwrap();
        change(&mut s, &mut last);
        if s.brightness > 0 {
            *last = s.brightness;
        }
    }
    ctl.apply();
    let s = ctl.settings();
    {
        let st = app.state::<AppState>();
        let mut cfg = st.config.lock().unwrap();
        cfg.set_keyboard(&s);
        let _ = config::save(&cfg);
    }
    let v = ctl.view();
    let _ = app.emit("keyboard", v.clone());
    crate::tray::sync_lights(app, s.brightness > 0);
    v
}

pub fn spawn(app: AppHandle) {
    app.state::<KeyboardCtl>().apply();
    thread::spawn(move || {
        let ctl = app.state::<KeyboardCtl>();
        let mut hue = 0.0f32;
        let mut last_tick = Instant::now();
        let mut last_apply = Instant::now();
        loop {
            thread::sleep(RAINBOW_TICK);
            let now = Instant::now();
            let resumed = keyboard::is_resume(now.duration_since(last_tick));
            last_tick = now;
            let s = ctl.settings();
            if s.rainbow && s.brightness > 0 {
                hue = (hue + RAINBOW_STEP_DEG) % 360.0;
                ctl.lamp.set(hue_rgb(hue), s.brightness);
            } else if resumed || now.duration_since(last_apply) >= REAPPLY_EVERY {
                ctl.apply();
                last_apply = now;
            }
        }
    });
}

pub fn shortcut_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    use tauri_plugin_global_shortcut::{Code, ShortcutState};
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }
            let raise = shortcut.key == Code::ArrowUp;
            update(app, |s, _| s.brightness = keyboard::step_brightness(s.brightness, raise));
        })
        .build()
}

/// Ctrl+Up / Ctrl+Down for keyboard brightness. Registered after startup so
/// a chord already taken by another program only disables the shortcut
/// instead of stopping the app from starting.
pub fn register_shortcuts(app: &AppHandle) {
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
    for code in [Code::ArrowUp, Code::ArrowDown] {
        if let Err(e) = app.global_shortcut().register(Shortcut::new(Some(Modifiers::CONTROL), code)) {
            eprintln!("keyboard shortcut unavailable: {e}");
        }
    }
}
