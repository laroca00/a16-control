pub mod cli;
pub mod commands;
pub mod config;
pub mod elevation;
pub mod fan;
pub mod hw;
pub mod setup;
pub mod state;
pub mod telemetry;
pub mod tray;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, WindowEvent};

use crate::hw::com::ComWmi;
use crate::hw::GbWmi;
use crate::state::AppState;

const TELEMETRY_INTERVAL: Duration = Duration::from_millis(1500);

pub fn run(needs_setup: bool, tray_start: bool) {
    let wmi: Option<Arc<dyn GbWmi>> = if needs_setup {
        None
    } else {
        ComWmi::connect().ok().map(|w| Arc::new(w) as Arc<dyn GbWmi>)
    };
    let show_main_now = needs_setup || wmi.is_none() || !tray_start;

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| tray::show_main(app)))
        .plugin(tauri_plugin_positioner::init())
        .manage(AppState { wmi, fan_lock: Mutex::new(()), config: Mutex::new(config::load()) })
        .invoke_handler(commands::handler())
        .on_window_event(|window, event| match (window.label(), event) {
            ("panel", WindowEvent::Focused(false)) => tray::panel_blurred(window.app_handle()),
            // Closing the main window only hides it; the app lives in the tray.
            ("main", WindowEvent::CloseRequested { api, .. }) => {
                api.prevent_close();
                let _ = window.hide();
            }
            _ => {}
        })
        .setup(move |app| {
            tray::build(app.handle())?;
            if show_main_now {
                tray::show_main(app.handle());
            }
            spawn_background(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running GiENEMY");
}

fn any_window_visible(app: &AppHandle) -> bool {
    ["main", "panel"]
        .iter()
        .filter_map(|l| app.get_webview_window(l))
        .any(|w| w.is_visible().unwrap_or(false))
}

/// One loop for the show-window requests and telemetry. Telemetry is only
/// read while someone can see it.
fn spawn_background(app: AppHandle) {
    thread::spawn(move || loop {
        let request = config::show_request_path();
        if request.exists() {
            let _ = std::fs::remove_file(&request);
            tray::show_main(&app);
        }
        if any_window_visible(&app) {
            match app.state::<AppState>().telemetry() {
                Ok(t) => {
                    tray::sync_fan_check(&app, Some(&t));
                    let _ = app.emit("telemetry", t);
                }
                Err(e) => {
                    let _ = app.emit("telemetry-error", e);
                }
            }
        }
        thread::sleep(TELEMETRY_INTERVAL);
    });
}
