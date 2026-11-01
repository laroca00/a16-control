pub mod afterburner;
pub mod battery;
pub mod battery_ctl;
pub mod cli;
pub mod commands;
pub mod config;
pub mod display;
pub mod elevation;
pub mod fan;
pub mod gpu;
pub mod hw;
pub mod keyboard;
pub mod keyboard_ctl;
pub mod os;
pub mod power;
pub mod setup;
pub mod state;
pub mod telemetry;
pub mod tray;

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, WindowEvent};

use crate::hw::com::ComWmi;
use crate::hw::GbWmi;
use crate::power::{Inputs, UiMode};
use crate::state::{AppState, Connector};

const TELEMETRY_INTERVAL: Duration = Duration::from_millis(1500);

pub fn run(needs_setup: bool, tray_start: bool) {
    let connector: Option<Connector> = if needs_setup {
        None
    } else {
        Some(Box::new(|| ComWmi::connect().map(|w| Arc::new(w) as Arc<dyn GbWmi>)))
    };
    let cfg = config::load();
    let keyboard = keyboard_ctl::KeyboardCtl::new(cfg.keyboard());
    let state = AppState::new(connector, cfg);
    // A firmware interface that is not reachable at sign-in needs explaining.
    let show_main_now = needs_setup || state.wmi().is_err() || !tray_start;

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| tray::show_main(app)))
        .plugin(keyboard_ctl::shortcut_plugin())
        .manage(state)
        .manage(keyboard)
        .manage(display::DisplayState::default())
        .manage(gpu::GpuCtl::new())
        .manage(battery_ctl::BatteryCtl::default())
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
            keyboard_ctl::spawn(app.handle().clone());
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

/// Collects the power inputs, lets the controller decide and applies the
/// result. Runs on every background tick and right after the user picks a mode.
pub fn power_tick(app: &AppHandle) {
    let st = app.state::<AppState>();
    let options = st.config.lock().unwrap().power_options();
    let (on_ac, battery_pct) = os::power_status();
    let fullscreen = os::foreground_fullscreen();
    let mut inputs = Inputs { now: std::time::Instant::now(), on_ac, battery_pct, fullscreen, gpu_util: None };
    let wants_gpu = st.power.lock().unwrap().controller.wants_gpu_counter(&inputs, &options);
    if wants_gpu {
        inputs.gpu_util = os::gpu_3d_util();
    }
    let (decision, mode) = {
        let mut p = st.power.lock().unwrap();
        let d = p.controller.tick(&inputs, &options);
        (d, p.controller.mode())
    };
    persist_mode(&st, mode);
    let Some(decision) = decision else { return };
    // Applying spawns powercfg and touches the display, so no lock is held.
    let result = st.wmi().and_then(|w| power::apply_decision(w.as_ref(), &decision));
    let (ab, igpu) = {
        let cfg = st.config.lock().unwrap();
        (cfg.afterburner, cfg.igpu_on_battery)
    };
    let installed = afterburner::exe().map(|_| true);
    let actions = afterburner::plan(decision.profile, installed, installed.is_some() && afterburner::running(), &ab, igpu);
    if !actions.is_empty() {
        thread::spawn(move || afterburner::run(&actions));
    }
    {
        let mut p = st.power.lock().unwrap();
        p.last = Some(decision);
        p.error = result.err();
    }
    tray::sync_power(app, mode);
    let _ = app.emit("power", st.power_view());
}

fn persist_mode(st: &AppState, mode: UiMode) {
    let mut cfg = st.config.lock().unwrap();
    if cfg.power_mode != mode {
        cfg.power_mode = mode;
        let _ = config::save(&cfg);
    }
}

const GPU_CHECK_EVERY: std::time::Duration = std::time::Duration::from_secs(60);

/// Once a minute: route apps by power source and reset a stuck dGPU.
fn gpu_tick(app: &AppHandle) {
    let (igpu, auto_reset) = {
        let st = app.state::<AppState>();
        let cfg = st.config.lock().unwrap();
        (cfg.igpu_on_battery, cfg.gpu_auto_reset)
    };
    let ctl = app.state::<gpu::GpuCtl>();
    let on_battery = os::power_status().0 == Some(false);
    {
        let mut last = ctl.last_battery.lock().unwrap();
        if *last != Some(on_battery) {
            *last = Some(on_battery);
            gpu::set_apps_gpu(on_battery && igpu);
        }
    }
    let Some(id) = ctl.id.clone() else { return };
    let d0 = gpu::power_state(&id) == Some(gpu::DState::D0);
    let inputs = gpu::GpuInputs {
        now: std::time::Instant::now(),
        enabled: auto_reset,
        on_battery,
        d0,
        fullscreen: os::foreground_fullscreen(),
        idle: os::user_idle(),
    };
    // The card is already awake when this runs, so reading its load is free.
    if ctl.guard.lock().unwrap().tick(&inputs, || os::gpu_3d_util().map(|u| u >= 5.0)) {
        gpu::restart(&id);
    }
}

/// One loop for the show-window requests and telemetry. Telemetry is only
/// read while someone can see it.
fn spawn_background(app: AppHandle) {
    let mut last_gpu = std::time::Instant::now() - GPU_CHECK_EVERY;
    thread::spawn(move || loop {
        let request = config::show_request_path();
        if request.exists() {
            let _ = std::fs::remove_file(&request);
            tray::show_main(&app);
        }
        power_tick(&app);
        display::watch(&app);
        battery_ctl::tick(&app);
        if last_gpu.elapsed() >= GPU_CHECK_EVERY {
            last_gpu = std::time::Instant::now();
            gpu_tick(&app);
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
