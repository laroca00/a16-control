//! Headless commands handled before Tauri starts. The exe uses the windows
//! subsystem, so output only shows if we attach to the parent console.

use std::io::Write;

use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};

use crate::elevation;
use crate::fan::{self, FanMode};
use crate::power::{self, Decision, ProfileId, Reason};
use crate::setup;
use crate::hw::com::ComWmi;
use crate::hw::GbWmi;
use crate::telemetry;

pub fn is_cli(args: &[String]) -> bool {
    matches!(
        args.get(1).map(String::as_str),
        Some("--probe" | "--fan" | "--fan-steps" | "--fan-whisper" | "--power" | "--install" | "--uninstall")
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
        (Some("--fan-steps"), _) => fan_steps(&wmi),
        (Some("--fan-whisper"), _) => fan_whisper(&wmi),
        (Some("--fan"), Some(mode)) => {
            let mode: FanMode = serde_json::from_value(mode.into())
                .map_err(|_| format!("unknown fan mode {mode:?}, use auto, quiet or max"))?;
            // From here quiet is a bare lock: nothing hands it to the
            // firmware when the chip gets hot, as the app does.
            fan::apply(&wmi, mode).map_err(|e| e.to_string())?;
            probe(&wmi)
        }
        (Some("--power"), Some(mode)) => {
            let profile = match mode {
                "battery" => ProfileId::Battery,
                "balanced" => ProfileId::BalancedAc,
                "performance" => ProfileId::Performance,
                other => return Err(format!("unknown power mode {other:?}, use battery, balanced or performance")),
            };
            let decision = Decision { profile, displays: true, reason: Reason::UserBattery };
            power::apply_decision(Ok(&wmi), &decision, power::profile(profile).fan)?;
            probe(&wmi)
        }
        _ => Err("usage: GiENEMY.exe --probe | --fan auto|quiet|max | --fan-steps | --fan-whisper | --power battery|balanced|performance".into()),
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

/// Hardware experiment for a capped fan mode: holds the step-mode lock at a
/// range of duties and step indices and reports where the fans settle, then
/// hands them back to the firmware. Takes about four minutes.
fn fan_steps(wmi: &dyn GbWmi) -> Result<String, String> {
    const SETTLE: std::time::Duration = std::time::Duration::from_secs(15);
    let duties = [40, 55, 70, 85, 100].map(|d| (0, d));
    let steps = [1, 2, 3, 4, 5, 6].map(|s| (s, 100));
    let mut result = Ok(());
    for (step, duty) in duties.into_iter().chain(steps) {
        let sequence = [
            ("SetCurrentFanStep", step),
            ("SetAutoFanStatus", 0),
            ("SetFixedFanSpeed", duty),
            ("SetGPUFanDuty", duty),
            ("SetStepFanStatus", 1),
            ("SetFixedFanStatus", 1),
        ];
        if let Some(e) = sequence.iter().find_map(|(m, v)| wmi.set(m, *v).err()) {
            println!("step {step} duty {duty}: write failed: {e}");
            continue;
        }
        std::thread::sleep(SETTLE);
        match telemetry::read(wmi) {
            Ok(t) => println!(
                "step {step} duty {duty}: {} / {} RPM, duty reads {} / {}, {} / {} C",
                t.rpm1, t.rpm2, t.cpu_duty, t.gpu_duty, t.cpu_temp, t.gpu_temp
            ),
            Err(e) => {
                result = Err(e.to_string());
                break;
            }
        }
        let _ = std::io::stdout().flush();
    }
    fan::apply(wmi, FanMode::Auto).map_err(|e| e.to_string())?;
    result.map(|()| "fans handed back to the firmware".into())
}

/// Hardware experiment: the firmware's own curve with Whisper mode on, under
/// two minutes of load on every core, to see where Whisper caps the fans.
fn fan_whisper(wmi: &dyn GbWmi) -> Result<String, String> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    fan::apply(wmi, FanMode::Auto).map_err(|e| e.to_string())?;
    wmi.set("SetWhisperMode", 1).map_err(|e| format!("SetWhisperMode is not available: {e}"))?;
    let stop = Arc::new(AtomicBool::new(false));
    let cores = std::thread::available_parallelism().map_or(8, |n| n.get());
    let workers: Vec<_> = (0..cores)
        .map(|_| {
            let stop = stop.clone();
            std::thread::spawn(move || {
                let mut x = 1u64;
                while !stop.load(Ordering::Relaxed) {
                    x = std::hint::black_box(x.wrapping_mul(6364136223846793005).wrapping_add(1));
                }
            })
        })
        .collect();
    let mut result = Ok(());
    for second in (10..=120).step_by(10) {
        std::thread::sleep(std::time::Duration::from_secs(10));
        match telemetry::read(wmi) {
            Ok(t) => println!("{second:>3}s: {} / {} RPM, {} / {} C", t.rpm1, t.rpm2, t.cpu_temp, t.gpu_temp),
            Err(e) => {
                result = Err(e.to_string());
                break;
            }
        }
        let _ = std::io::stdout().flush();
    }
    stop.store(true, Ordering::Relaxed);
    for w in workers {
        let _ = w.join();
    }
    let _ = wmi.set("SetWhisperMode", 0);
    fan::apply(wmi, FanMode::Auto).map_err(|e| e.to_string())?;
    result.map(|()| "load stopped, whisper off, fans handed back to the firmware".into())
}
