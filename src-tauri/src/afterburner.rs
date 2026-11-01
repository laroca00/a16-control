//! MSI Afterburner: load the user's saved overclock profile in Performance
//! mode and the stock one elsewhere. Offsets are never invented here; the
//! user tunes a stable profile once in Afterburner.

use std::path::PathBuf;
use std::process::Command;

use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{FOLDERID_ProgramFiles, FOLDERID_ProgramFilesX86, SHGetKnownFolderPath, KF_FLAG_DEFAULT};

use crate::elevation::tool;
use crate::power::ProfileId;

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AbOptions {
    pub enabled: bool,
    pub oc_profile: u8,
    pub stock_profile: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AbAction {
    LoadProfile(u8),
    Close,
}

fn known_folder(id: &windows::core::GUID) -> Option<PathBuf> {
    unsafe {
        let p = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None).ok()?;
        let path = p.to_string().ok().map(PathBuf::from);
        CoTaskMemFree(Some(p.0 as *const _));
        path
    }
}

/// Afterburner's exe. The folders come from the shell, not from environment
/// variables, which a non-elevated process could point at its own copy for
/// this elevated app to launch.
pub fn exe() -> Option<PathBuf> {
    [FOLDERID_ProgramFilesX86, FOLDERID_ProgramFiles]
        .iter()
        .filter_map(known_folder)
        .map(|base| base.join(r"MSI Afterburner\MSIAfterburner.exe"))
        .find(|p| p.exists())
}

pub fn running() -> bool {
    tool("tasklist.exe")
        .args(["/FI", "IMAGENAME eq MSIAfterburner.exe", "/NH"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("MSIAfterburner.exe"))
        .unwrap_or(false)
}

/// What to do for a newly applied profile. `installed` is None when
/// Afterburner is not on this machine.
pub fn plan(profile: ProfileId, installed: Option<bool>, running: bool, o: &AbOptions, igpu_on_battery: bool) -> Vec<AbAction> {
    if installed.is_none() || !o.enabled {
        return Vec::new();
    }
    let mut actions = Vec::new();
    match profile {
        ProfileId::Performance => actions.push(AbAction::LoadProfile(o.oc_profile)),
        _ if running => actions.push(AbAction::LoadProfile(o.stock_profile)),
        _ => {}
    }
    // Afterburner polls the dGPU every second and keeps it awake on battery;
    // applied offsets survive the process exiting, so closing costs nothing.
    if profile == ProfileId::Battery && running && igpu_on_battery {
        actions.push(AbAction::Close);
    }
    actions
}

pub fn run(actions: &[AbAction]) {
    let Some(exe) = exe() else { return };
    for a in actions {
        match a {
            AbAction::LoadProfile(n) => {
                let _ = Command::new(&exe).arg(format!("-Profile{n}")).spawn();
            }
            AbAction::Close => {
                // Let the profile command land before closing.
                std::thread::sleep(std::time::Duration::from_secs(4));
                let _ = tool("taskkill.exe").args(["/IM", "MSIAfterburner.exe"]).output();
                let _ = tool("taskkill.exe").args(["/IM", "MSIAfterburner.exe", "/F"]).output();
                let _ = tool("taskkill.exe").args(["/IM", "RTSS.exe", "/F"]).output();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::power::ProfileId;

    const ON: AbOptions = AbOptions { enabled: true, oc_profile: 1, stock_profile: 5 };

    #[test]
    fn afterburner_absent_is_a_no_op() {
        assert!(plan(ProfileId::Performance, None, false, &ON, true).is_empty());
    }

    #[test]
    fn disabled_is_a_no_op() {
        let off = AbOptions { enabled: false, ..ON };
        assert!(plan(ProfileId::Performance, Some(true), false, &off, true).is_empty());
    }

    #[test]
    fn performance_loads_oc_profile() {
        assert_eq!(plan(ProfileId::Performance, Some(true), false, &ON, true), vec![AbAction::LoadProfile(1)]);
    }

    #[test]
    fn balanced_loads_stock_only_if_running() {
        assert_eq!(plan(ProfileId::BalancedAc, Some(true), true, &ON, true), vec![AbAction::LoadProfile(5)]);
        assert!(plan(ProfileId::BalancedAc, Some(true), false, &ON, true).is_empty());
    }

    #[test]
    fn battery_closes_when_igpu_option_on() {
        assert_eq!(
            plan(ProfileId::Battery, Some(true), true, &ON, true),
            vec![AbAction::LoadProfile(5), AbAction::Close]
        );
        assert_eq!(plan(ProfileId::Battery, Some(true), true, &ON, false), vec![AbAction::LoadProfile(5)]);
    }
}
