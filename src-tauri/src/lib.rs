pub mod cli;
pub mod elevation;
pub mod fan;
pub mod hw;
pub mod setup;
pub mod telemetry;

pub fn run(_needs_setup: bool, _tray_start: bool) {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running GiENEMY");
}
