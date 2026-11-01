//! Headless commands handled before Tauri starts. The exe uses the windows
//! subsystem, so output only shows if we attach to the parent console.

use std::io::Write;

use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};

use crate::fan::{self, FanMode};
use crate::hw::com::ComWmi;
use crate::hw::GbWmi;
use crate::telemetry;

pub fn is_cli(args: &[String]) -> bool {
    matches!(args.get(1).map(String::as_str), Some("--probe" | "--fan"))
}

/// Runs the command and returns the process exit code.
pub fn run(args: &[String]) -> i32 {
    unsafe {
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
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
