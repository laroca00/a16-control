use std::process::Command;

use tauri::{AppHandle, Manager, State};

use crate::config::{self, Config};
use crate::elevation::{self, hidden};
use crate::fan::FanMode;
use crate::setup::{self, SetupStatus, TASK_NAME};
use crate::state::{self, AppState};
use crate::telemetry::Telemetry;
use crate::tray;

#[tauri::command]
pub fn get_setup_status() -> SetupStatus {
    setup::status()
}

#[tauri::command]
pub fn get_telemetry(state: State<'_, AppState>) -> Result<Telemetry, String> {
    state.telemetry()
}

#[tauri::command]
pub fn set_fan(app: AppHandle, state: State<'_, AppState>, mode: FanMode) -> Result<Telemetry, String> {
    let t = state::set_fan(&state, mode)?;
    tray::sync_fan_check(&app, Some(&t));
    Ok(t)
}

#[tauri::command]
pub fn run_install(app: AppHandle) -> Result<(), String> {
    match elevation::run_elevated_and_wait("--install")? {
        0 => {}
        code => return Err(format!("install exited with {code}")),
    }
    let _ = std::fs::create_dir_all(config::dir());
    let _ = std::fs::write(config::show_request_path(), b"");
    // Start the elevated copy only after this one has released the
    // single-instance lock, or the new copy would exit straight away.
    hidden(Command::new("cmd").args([
        "/c",
        &format!("ping -n 3 127.0.0.1 >nul & schtasks /run /tn {TASK_NAME}"),
    ]))
    .spawn()
    .map_err(|e| e.to_string())?;
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn run_uninstall(app: AppHandle) -> Result<(), String> {
    match elevation::run_elevated_and_wait("--uninstall")? {
        0 => {
            tray::quit(&app);
            Ok(())
        }
        code => Err(format!("uninstall exited with {code}")),
    }
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Config {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
pub fn set_lang(app: AppHandle, state: State<'_, AppState>, lang: String) -> Result<(), String> {
    if lang != "tr" && lang != "en" {
        return Err(format!("unsupported language {lang:?}"));
    }
    let mut cfg = state.config.lock().unwrap();
    cfg.lang = Some(lang);
    config::save(&cfg)?;
    drop(cfg);
    tray::relabel(&app);
    Ok(())
}

#[tauri::command]
pub fn open_main(app: AppHandle) {
    tray::show_main(&app);
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    tray::quit(&app);
}

pub fn handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        get_setup_status,
        get_telemetry,
        set_fan,
        run_install,
        run_uninstall,
        get_config,
        set_lang,
        open_main,
        quit_app
    ]
}

/// Language for native strings (tray menu): config first, then the Windows UI language.
pub fn lang(app: &AppHandle) -> String {
    if let Some(l) = app.state::<AppState>().config.lock().unwrap().lang.clone() {
        return l;
    }
    // LANG_TURKISH is primary language 0x1F.
    let ui = unsafe { windows::Win32::Globalization::GetUserDefaultUILanguage() };
    if ui & 0x3FF == 0x1F { "tr".into() } else { "en".into() }
}
