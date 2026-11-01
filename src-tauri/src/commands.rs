use tauri::async_runtime::spawn_blocking;
use tauri::{AppHandle, Manager, State};

use crate::config::{self, Config};
use crate::elevation::{self, system_tool, tool};
use crate::display::{self, DisplayView};
use crate::afterburner::{self, AbOptions};
use crate::battery::BatteryView;
use crate::battery_ctl;
use crate::fan::FanMode;
use crate::gpu::{GpuCtl, GpuView};
use crate::keyboard::KeyboardView;
use crate::keyboard_ctl::{self, KeyboardCtl};
use crate::power::{self, PowerView, UiMode};
use crate::setup::{self, SetupStatus, TASK_NAME};
use crate::state::{self, AppState};
use crate::telemetry::Telemetry;
use crate::tray;

// Commands that spawn processes, wait on UAC or call the firmware run on a
// blocking thread; plain commands run on the main thread and would freeze
// the tray and both windows meanwhile.

fn joined<T>(r: Result<T, tauri::Error>) -> Result<T, String> {
    r.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_setup_status() -> Result<SetupStatus, String> {
    joined(spawn_blocking(setup::status).await)
}

#[tauri::command]
pub async fn get_telemetry(app: AppHandle) -> Result<Telemetry, String> {
    joined(spawn_blocking(move || app.state::<AppState>().telemetry()).await)?
}

#[tauri::command]
pub async fn set_fan(app: AppHandle, mode: FanMode) -> Result<Telemetry, String> {
    let worker = app.clone();
    let t = joined(spawn_blocking(move || state::set_fan(&worker.state::<AppState>(), mode)).await)??;
    tray::sync_fan_check(&app, Some(&t));
    Ok(t)
}

#[tauri::command]
pub async fn run_install(app: AppHandle) -> Result<(), String> {
    match joined(spawn_blocking(|| elevation::run_elevated_and_wait("--install")).await)?? {
        0 => {}
        code => return Err(format!("install exited with {code}")),
    }
    let _ = std::fs::create_dir_all(config::dir());
    let _ = std::fs::write(config::show_request_path(), b"");
    // Start the elevated copy only after this one has released the
    // single-instance lock, or the new copy would exit straight away.
    let ping = system_tool("PING.EXE");
    let schtasks = system_tool("schtasks.exe");
    tool("cmd.exe")
        .current_dir(system_tool(""))
        .args([
            "/c",
            &format!(
                "\"{}\" -n 3 127.0.0.1 >nul & \"{}\" /run /tn {TASK_NAME}",
                ping.display(),
                schtasks.display()
            ),
        ])
        .spawn()
        .map_err(|e| e.to_string())?;
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub async fn run_uninstall(app: AppHandle) -> Result<(), String> {
    match joined(spawn_blocking(|| elevation::run_elevated_and_wait("--uninstall")).await)?? {
        0 => {
            tray::quit(&app);
            Ok(())
        }
        code => Err(format!("uninstall exited with {code}")),
    }
}

#[tauri::command]
pub fn get_power(state: State<'_, AppState>) -> PowerView {
    state.power_view()
}

#[tauri::command]
pub async fn set_power_mode(app: AppHandle, mode: UiMode) -> Result<PowerView, String> {
    joined(
        spawn_blocking(move || {
            app.state::<AppState>().choose_mode(mode);
            crate::power_tick(&app);
            app.state::<AppState>().power_view()
        })
        .await,
    )
}

#[tauri::command]
pub fn set_power_option(state: State<'_, AppState>, key: String, value: bool) -> Result<PowerView, String> {
    {
        let mut cfg = state.config.lock().unwrap();
        let o = power::with_option(cfg.power_options(), &key, value)?;
        cfg.auto_game = o.auto_game;
        cfg.acdc_switch = o.acdc_switch;
        config::save(&cfg)?;
    }
    Ok(state.power_view())
}

#[tauri::command]
pub fn get_keyboard(ctl: State<'_, KeyboardCtl>) -> KeyboardView {
    ctl.view()
}

#[tauri::command]
pub fn set_keyboard_color(app: AppHandle, rgb: [u8; 3]) -> KeyboardView {
    keyboard_ctl::update(&app, |s, _| {
        s.rgb = rgb;
        s.rainbow = false;
    })
}

#[tauri::command]
pub fn set_keyboard_brightness(app: AppHandle, value: u8) -> KeyboardView {
    keyboard_ctl::update(&app, |s, _| s.brightness = value.min(100))
}

#[tauri::command]
pub fn set_keyboard_rainbow(app: AppHandle, on: bool) -> KeyboardView {
    keyboard_ctl::update(&app, |s, _| s.rainbow = on)
}

#[tauri::command]
pub async fn get_display(app: AppHandle) -> Result<DisplayView, String> {
    joined(spawn_blocking(move || display::view(&app)).await)
}

#[tauri::command]
pub async fn set_brightness(app: AppHandle, value: u32) -> Result<DisplayView, String> {
    joined(spawn_blocking(move || {
        display::set_brightness(value);
        display::view(&app)
    })
    .await)
}

#[tauri::command]
pub async fn set_refresh_rate(app: AppHandle, hz: u32) -> Result<DisplayView, String> {
    joined(spawn_blocking(move || {
        display::set_refresh(hz);
        display::view(&app)
    })
    .await)
}

// Building a window from a synchronous command deadlocks on Windows, so
// these run on the async runtime.
#[tauri::command]
pub async fn screen_off(app: AppHandle) -> Result<(), String> {
    display::screen_off(&app)
}

#[tauri::command]
pub async fn screen_on(app: AppHandle) {
    display::screen_on(&app);
}

fn gpu_view(app: &AppHandle) -> GpuView {
    let cfg = app.state::<AppState>().config.lock().unwrap().clone();
    GpuView { state: app.state::<GpuCtl>().state(), igpu_on_battery: cfg.igpu_on_battery, auto_reset: cfg.gpu_auto_reset }
}

#[tauri::command]
pub fn get_gpu(app: AppHandle) -> GpuView {
    gpu_view(&app)
}

#[tauri::command]
pub fn set_gpu_option(app: AppHandle, key: String, value: bool) -> Result<GpuView, String> {
    {
        let st = app.state::<AppState>();
        let mut cfg = st.config.lock().unwrap();
        match key.as_str() {
            "igpuOnBattery" => cfg.igpu_on_battery = value,
            "autoReset" => cfg.gpu_auto_reset = value,
            other => return Err(format!("unknown gpu option {other:?}")),
        }
        config::save(&cfg)?;
    }
    // Re-route on the next minute tick with the new setting.
    *app.state::<GpuCtl>().last_battery.lock().unwrap() = None;
    Ok(gpu_view(&app))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AfterburnerView {
    installed: bool,
    options: AbOptions,
}

#[tauri::command]
pub fn get_afterburner(state: State<'_, AppState>) -> AfterburnerView {
    AfterburnerView { installed: afterburner::exe().is_some(), options: state.config.lock().unwrap().afterburner }
}

#[tauri::command]
pub fn set_afterburner(state: State<'_, AppState>, options: AbOptions) -> Result<AfterburnerView, String> {
    let valid = 1..=5;
    if !valid.contains(&options.oc_profile) || !valid.contains(&options.stock_profile) {
        return Err("Afterburner profiles are numbered 1 to 5".into());
    }
    {
        let mut cfg = state.config.lock().unwrap();
        cfg.afterburner = options;
        config::save(&cfg)?;
    }
    Ok(get_afterburner(state))
}

#[tauri::command]
pub fn get_battery(app: AppHandle) -> BatteryView {
    battery_ctl::view(&app)
}

#[tauri::command]
pub async fn start_watch(app: AppHandle) -> Result<BatteryView, String> {
    joined(spawn_blocking(move || battery_ctl::start(&app)).await)?
}

#[tauri::command]
pub async fn cancel_watch(app: AppHandle) -> Result<BatteryView, String> {
    joined(spawn_blocking(move || battery_ctl::cancel(&app)).await)
}

#[tauri::command]
pub async fn set_eco_enabled(app: AppHandle, on: bool) -> Result<BatteryView, String> {
    joined(spawn_blocking(move || set_eco_enabled_blocking(&app, on)).await)?
}

fn set_eco_enabled_blocking(app: &AppHandle, on: bool) -> Result<BatteryView, String> {
    {
        let st = app.state::<AppState>();
        let mut cfg = st.config.lock().unwrap();
        cfg.eco_enabled = on;
        config::save(&cfg)?;
    }
    if !on {
        battery_ctl::release_all(app);
    }
    Ok(battery_ctl::view(app))
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
        get_power,
        set_power_mode,
        set_power_option,
        get_keyboard,
        set_keyboard_color,
        set_keyboard_brightness,
        set_keyboard_rainbow,
        get_display,
        set_brightness,
        set_refresh_rate,
        screen_off,
        screen_on,
        get_gpu,
        set_gpu_option,
        get_afterburner,
        set_afterburner,
        get_battery,
        start_watch,
        cancel_watch,
        set_eco_enabled,
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
