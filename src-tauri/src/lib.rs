pub mod fan;
pub mod hw;
pub mod telemetry;

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running GiENEMY");
}
