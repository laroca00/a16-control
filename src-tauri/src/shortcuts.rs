//! Global shortcuts. They work from anywhere, a game included, so each one
//! answers with a short notification instead of a window.
//!
//! Ctrl+Up / Ctrl+Down   keyboard brightness
//! Ctrl+Alt+F            Max Fan on or off
//! Ctrl+Alt+P            next power mode

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::commands::lang;
use crate::fan::FanMode;
use crate::monitor::notify;
use crate::power::UiMode;
use crate::state::AppState;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    Brightness { raise: bool },
    ToggleFan,
    NextPowerMode,
}

pub const BINDINGS: [(Modifiers, Code, Action); 4] = [
    (Modifiers::CONTROL, Code::ArrowUp, Action::Brightness { raise: true }),
    (Modifiers::CONTROL, Code::ArrowDown, Action::Brightness { raise: false }),
    (Modifiers::CONTROL.union(Modifiers::ALT), Code::KeyF, Action::ToggleFan),
    (Modifiers::CONTROL.union(Modifiers::ALT), Code::KeyP, Action::NextPowerMode),
];

pub fn action_for(shortcut: &Shortcut) -> Option<Action> {
    BINDINGS
        .iter()
        .find(|(mods, code, _)| shortcut.mods == *mods && shortcut.key == *code)
        .map(|(_, _, action)| *action)
}

/// Battery, Auto and Performance in a loop.
pub fn next_mode(mode: UiMode) -> UiMode {
    match mode {
        UiMode::Battery => UiMode::Auto,
        UiMode::Auto => UiMode::Performance,
        UiMode::Performance => UiMode::Battery,
    }
}

fn mode_name(tr: bool, mode: UiMode) -> &'static str {
    match (tr, mode) {
        (true, UiMode::Battery) => "Pil",
        (true, UiMode::Auto) => "Otomatik",
        (true, UiMode::Performance) => "Performans",
        (false, UiMode::Battery) => "Battery",
        (false, UiMode::Auto) => "Auto",
        (false, UiMode::Performance) => "Performance",
    }
}

fn run(app: &AppHandle, action: Action) {
    let tr = lang(app) == "tr";
    match action {
        Action::Brightness { raise } => {
            crate::keyboard_ctl::update(app, |s, _| s.brightness = crate::keyboard::step_brightness(s.brightness, raise));
        }
        Action::ToggleFan => {
            let body = match (crate::tray::toggle_fan(app), tr) {
                (Some(FanMode::Max), true) => "Fanlar tam devirde.",
                (Some(FanMode::Max), false) => "Fans at full speed.",
                (Some(_), true) => "Fan hızını yeniden firmware ayarlıyor.",
                (Some(_), false) => "The firmware sets the fan speed again.",
                (None, true) => "Fanlara şu an ulaşılamıyor.",
                (None, false) => "The fans cannot be reached right now.",
            };
            notify(app, "Max Fan", body);
        }
        Action::NextPowerMode => {
            let next = next_mode(app.state::<AppState>().power.lock().unwrap().controller.mode());
            crate::tray::set_mode_from_tray(app, next);
            notify(app, if tr { "Güç modu" } else { "Power mode" }, mode_name(tr, next));
        }
    }
}

pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }
            let Some(action) = action_for(shortcut) else { return };
            // Fan and power changes call the firmware; keep the event loop free.
            let app = app.clone();
            std::thread::spawn(move || run(&app, action));
        })
        .build()
}

/// Registered after startup so a chord already taken by another program only
/// disables that shortcut instead of stopping the app from starting.
pub fn register(app: &AppHandle) {
    for (mods, code, _) in BINDINGS {
        if let Err(e) = app.global_shortcut().register(Shortcut::new(Some(mods), code)) {
            eprintln!("shortcut {mods:?}+{code:?} unavailable: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chords_map_to_their_actions() {
        let fan = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyF);
        assert_eq!(action_for(&fan), Some(Action::ToggleFan));
        let up = Shortcut::new(Some(Modifiers::CONTROL), Code::ArrowUp);
        assert_eq!(action_for(&up), Some(Action::Brightness { raise: true }));
        let other = Shortcut::new(Some(Modifiers::CONTROL), Code::KeyF);
        assert_eq!(action_for(&other), None);
    }

    #[test]
    fn power_modes_cycle() {
        assert_eq!(next_mode(UiMode::Battery), UiMode::Auto);
        assert_eq!(next_mode(UiMode::Auto), UiMode::Performance);
        assert_eq!(next_mode(UiMode::Performance), UiMode::Battery);
    }
}
