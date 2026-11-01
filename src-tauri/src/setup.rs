//! One-time elevated install: registers the firmware WMI classes (only where
//! missing) and the logon task that starts GiENEMY elevated without UAC.

use std::path::Path;
use std::process::Command;

use serde::Serialize;

use crate::elevation::{hidden, is_elevated};
use crate::hw::com::class_methods;

pub const TASK_NAME: &str = "GiENEMY";

pub const REQUIRED_SET: [&str; 6] = [
    "SetCurrentFanStep",
    "SetAutoFanStatus",
    "SetFixedFanSpeed",
    "SetGPUFanDuty",
    "SetStepFanStatus",
    "SetFixedFanStatus",
];

pub const REQUIRED_GET: [&str; 8] = [
    "getRpm1",
    "getRpm2",
    "getCpuTemp",
    "getGpuTemp1",
    "GetCPUFanDuty",
    "GetGPUFanDuty",
    "GetFixedFanStatus",
    "GetAutoFanStatus",
];

const SET_MOF: &str = include_str!("../resources/gienemy-set.mof");
const GET_MOF: &str = include_str!("../resources/gienemy-get.mof");

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SetupStatus {
    pub elevated: bool,
    pub task: bool,
    pub wmi_set: bool,
    pub wmi_get: bool,
    pub keyboard_light: bool,
}

pub fn missing_methods(present: &[String], required: &[&str]) -> Vec<String> {
    required
        .iter()
        .filter(|r| !present.iter().any(|p| p == *r))
        .map(|r| r.to_string())
        .collect()
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

pub fn task_xml(exe: &Path, user: &str) -> String {
    let exe = xml_escape(&exe.display().to_string());
    let user = xml_escape(user);
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>Starts GiENEMY with the rights it needs to drive the fans.</Description>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <UserId>{user}</UserId>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{user}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>false</StartWhenAvailable>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>4</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{exe}</Command>
      <Arguments>--tray</Arguments>
    </Exec>
  </Actions>
</Task>
"#
    )
}

fn run_ok(program: &str, args: &[&str]) -> bool {
    hidden(Command::new(program).args(args))
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn task_exists() -> bool {
    run_ok("schtasks", &["/query", "/tn", TASK_NAME])
}

fn class_has(class: &str, required: &[&str]) -> bool {
    class_methods(class)
        .map(|present| missing_methods(&present, required).is_empty())
        .unwrap_or(false)
}

fn keyboard_light_present() -> bool {
    let running = hidden(Command::new("tasklist").args(["/FI", "IMAGENAME eq KeyboardLight.exe", "/NH"]))
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("KeyboardLight.exe"))
        .unwrap_or(false);
    running || run_ok("schtasks", &["/query", "/tn", "AeroControlLogon"])
}

pub fn status() -> SetupStatus {
    SetupStatus {
        elevated: is_elevated(),
        task: task_exists(),
        wmi_set: class_has("GB_WMIACPI_Set", &REQUIRED_SET),
        wmi_get: class_has("GB_WMIACPI_Get", &REQUIRED_GET),
        keyboard_light: keyboard_light_present(),
    }
}

fn mofcomp(contents: &str, name: &str) -> Result<(), String> {
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, contents).map_err(|e| format!("write {name}: {e}"))?;
    let out = hidden(Command::new("mofcomp").arg(&path))
        .output()
        .map_err(|e| format!("mofcomp: {e}"))?;
    let _ = std::fs::remove_file(&path);
    if out.status.success() {
        Ok(())
    } else {
        Err(format!("mofcomp {name}: {}", String::from_utf8_lossy(&out.stdout).trim()))
    }
}

fn current_user() -> String {
    let domain = std::env::var("USERDOMAIN").unwrap_or_default();
    let user = std::env::var("USERNAME").unwrap_or_default();
    if domain.is_empty() { user } else { format!("{domain}\\{user}") }
}

pub fn install(exe: &Path) -> Result<(), String> {
    if !class_has("GB_WMIACPI_Set", &REQUIRED_SET) {
        mofcomp(SET_MOF, "gienemy-set.mof")?;
    }
    // Only when absent: Gigabyte's own definition is fuller than ours.
    if class_methods("GB_WMIACPI_Get").is_err() {
        mofcomp(GET_MOF, "gienemy-get.mof")?;
    }

    // schtasks expects the XML file in UTF-16 with a BOM.
    let xml = task_xml(exe, &current_user());
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(xml.encode_utf16().flat_map(u16::to_le_bytes));
    let path = std::env::temp_dir().join("gienemy-task.xml");
    std::fs::write(&path, bytes).map_err(|e| format!("write task xml: {e}"))?;
    let path_str = path.display().to_string();
    let out = hidden(Command::new("schtasks").args(["/create", "/tn", TASK_NAME, "/xml", &path_str, "/f"]))
        .output()
        .map_err(|e| format!("schtasks: {e}"))?;
    let _ = std::fs::remove_file(&path);
    if !out.status.success() {
        return Err(format!("schtasks /create: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(())
}

/// Removes the logon task. The WMI classes stay: Gigabyte's software uses
/// the same ones.
pub fn uninstall() -> Result<(), String> {
    if !task_exists() {
        return Ok(());
    }
    if run_ok("schtasks", &["/delete", "/tn", TASK_NAME, "/f"]) {
        Ok(())
    } else {
        Err("schtasks /delete failed".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_xml_has_required_settings() {
        let x = task_xml(Path::new(r"C:\Users\a b\AppData\Local\GiENEMY\GiENEMY.exe"), r"PC\a b");
        for needle in [
            "<RunLevel>HighestAvailable</RunLevel>",
            "<LogonTrigger>",
            r"<UserId>PC\a b</UserId>",
            "<ExecutionTimeLimit>PT0S</ExecutionTimeLimit>",
            "<MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>",
            "<DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>",
            "<StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>",
            "<Arguments>--tray</Arguments>",
        ] {
            assert!(x.contains(needle), "missing {needle}");
        }
    }

    #[test]
    fn task_xml_escapes_path() {
        let x = task_xml(Path::new(r"C:\R&D\GiENEMY.exe"), r"PC\u");
        assert!(x.contains(r"C:\R&amp;D\GiENEMY.exe"));
    }

    #[test]
    fn missing_methods_lists_only_absent() {
        let present = vec!["SetAutoFanStatus".to_string(), "SetFixedFanSpeed".to_string()];
        assert_eq!(
            missing_methods(&present, &["SetAutoFanStatus", "SetGPUFanDuty"]),
            vec!["SetGPUFanDuty"]
        );
    }
}
