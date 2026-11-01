#![windows_subsystem = "windows"]

use gienemy_lib::elevation::{self, StartupAction};
use gienemy_lib::{cli, config, setup};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let elevated = elevation::is_elevated();
    match elevation::startup_action(elevated, setup::task_exists(), cli::is_cli(&args)) {
        StartupAction::Cli => std::process::exit(cli::run(&args, elevated)),
        StartupAction::RelaunchViaTask => {
            // A manual launch wants a window, whether or not an instance is running.
            let _ = std::fs::create_dir_all(config::dir());
            let _ = std::fs::write(config::show_request_path(), b"");
            if elevation::launch_via_task().is_ok() {
                return;
            }
            // The task exists but would not start; fall back to the setup screen.
            gienemy_lib::run(true, false)
        }
        StartupAction::RunApp { needs_setup } => {
            let tray_start = args.iter().any(|a| a == "--tray");
            gienemy_lib::run(needs_setup, tray_start)
        }
    }
}
