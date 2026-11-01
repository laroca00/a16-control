//! Headless commands handled before Tauri starts. The exe uses the windows
//! subsystem, so output only shows if we attach to the parent console.

use std::io::Write;

use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};

use crate::elevation;
use crate::fan::{self, FanMode};
use crate::setup;
use crate::hw::com::ComWmi;
use crate::hw::GbWmi;
use crate::telemetry;

pub fn is_cli(args: &[String]) -> bool {
    matches!(
        args.get(1).map(String::as_str),
        Some("--probe" | "--fan" | "--install" | "--uninstall")
    )
}

/// Runs the command and returns the process exit code.
pub fn run(args: &[String], elevated: bool) -> i32 {
    unsafe {
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
    let cmd = args.get(1).map(String::as_str);
    if matches!(cmd, Some("--install" | "--uninstall")) {
        if !elevated {
            // One UAC prompt; the elevated copy does the work.
            return match elevation::run_elevated_and_wait(cmd.unwrap()) {
                Ok(code) => code,
                Err(e) => {
                    eprintln!("error: {e}");
                    1
                }
            };
        }
        let result = if cmd == Some("--install") {
            std::env::current_exe()
                .map_err(|e| e.to_string())
                .and_then(|exe| setup::install(&exe))
        } else {
            setup::uninstall()
        };
        return match result {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("error: {e}");
                1
            }
        };
    }
    match execute(args) {
        Ok(out) => {
            println!("{out}");
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

fn execute(args: &[String]) -> Result<String, String> {
    let wmi = ComWmi::connect().map_err(|e| e.to_string())?;
    match (args.get(1).map(String::as_str), args.get(2).map(String::as_str)) {
        (Some("--probe"), _) => probe(&wmi),
        (Some("--fan"), Some(mode)) => {
            let mode = match mode {
                "max" => FanMode::Max,
                "auto" => FanMode::Auto,
                other => return Err(format!("unknown fan mode {other:?}, use max or auto")),
            };
            fan::apply(&wmi, mode).map_err(|e| e.to_string())?;
            probe(&wmi)
        }
        _ => Err("usage: GiENEMY.exe --probe | --fan max|auto".into()),
    }
}

fn probe(wmi: &dyn GbWmi) -> Result<String, String> {
    let t = telemetry::read(wmi).map_err(|e| e.to_string())?;
    let raw_auto = wmi.get("GetAutoFanStatus").map_err(|e| e.to_string())?;
    let raw_fixed = wmi.get("GetFixedFanStatus").map_err(|e| e.to_string())?;
    let mut json = serde_json::to_value(&t).map_err(|e| e.to_string())?;
    json["rawAutoFanStatus"] = raw_auto.into();
    json["rawFixedFanStatus"] = raw_fixed.into();
    let _ = std::io::stdout().flush();
    serde_json::to_string_pretty(&json).map_err(|e| e.to_string())
}
