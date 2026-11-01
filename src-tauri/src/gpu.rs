//! dGPU discipline on battery: route common apps to the iGPU, and reload the
//! NVIDIA driver when the card stays awake with nothing using it.

use std::time::{Duration, Instant};

use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_Get_DevNode_PropertyW, CM_Get_Device_ID_ListW, CM_Get_Device_ID_List_SizeW, CM_Locate_DevNodeW,
    CM_GETIDLIST_FILTER_ENUMERATOR, CM_GETIDLIST_FILTER_PRESENT, CM_LOCATE_DEVNODE_NORMAL, CR_SUCCESS,
};
use windows::Win32::Devices::Properties::{DEVPKEY_Device_Class, DEVPKEY_Device_PowerData, DEVPROPTYPE};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegGetValueW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE,
    REG_OPTION_NON_VOLATILE, REG_SZ, RRF_RT_REG_SZ,
};

use crate::elevation::tool;

const STUCK_MINUTES: u32 = 5;
const IDLE_BEFORE_RESET: Duration = Duration::from_secs(120);
const COOLDOWN: Duration = Duration::from_secs(30 * 60);
const GPU_PREF_KEY: &str = r"Software\Microsoft\DirectX\UserGpuPreferences";

// Apps that commonly keep a laptop dGPU awake; only ones present are touched.
const APPS: [&str; 10] = [
    r"%ProgramFiles%\Google\Chrome\Application\chrome.exe",
    r"%ProgramFiles(x86)%\Google\Chrome\Application\chrome.exe",
    r"%ProgramFiles(x86)%\Microsoft\Edge\Application\msedge.exe",
    r"%ProgramFiles%\Mozilla Firefox\firefox.exe",
    r"%ProgramFiles%\BraveSoftware\Brave-Browser\Application\brave.exe",
    r"%APPDATA%\Spotify\Spotify.exe",
    r"%ProgramFiles%\Microsoft\Teams\current\Teams.exe",
    r"%LOCALAPPDATA%\Programs\Microsoft VS Code\Code.exe",
    r"%LOCALAPPDATA%\Programs\obsidian\Obsidian.exe",
    r"%LOCALAPPDATA%\slack\slack.exe",
];

pub struct GpuInputs {
    pub now: Instant,
    pub enabled: bool,
    pub on_battery: bool,
    /// The dGPU is in D0 (awake).
    pub d0: bool,
    pub fullscreen: bool,
    pub idle: Duration,
}

/// Decides, once a minute, whether a stuck dGPU should be reset.
#[derive(Default)]
pub struct ResetGuard {
    stuck: u32,
    last_reset: Option<Instant>,
}

impl ResetGuard {
    /// `busy` is only called when a reset is otherwise due: reading GPU load
    /// costs nothing once the card is awake, but should not be done blindly.
    pub fn tick(&mut self, i: &GpuInputs, busy: impl FnOnce() -> Option<bool>) -> bool {
        if !i.enabled || !i.on_battery || !i.d0 {
            self.stuck = 0;
            return false;
        }
        // Resetting drops the device for a second; never while someone works or plays.
        if i.fullscreen || i.idle < IDLE_BEFORE_RESET {
            return false;
        }
        self.stuck += 1;
        if self.stuck < STUCK_MINUTES || self.last_reset.is_some_and(|t| i.now.duration_since(t) < COOLDOWN) {
            return false;
        }
        self.stuck = 0;
        if busy() != Some(false) {
            return false;
        }
        self.last_reset = Some(i.now);
        true
    }
}

pub fn preference_value(pref: u32) -> String {
    format!("GpuPreference={pref};")
}

fn expand(path: &str) -> String {
    let mut out = path.to_string();
    for var in ["ProgramFiles(x86)", "ProgramFiles", "APPDATA", "LOCALAPPDATA"] {
        if let Ok(v) = std::env::var(var) {
            out = out.replace(&format!("%{var}%"), &v);
        }
    }
    out
}

#[derive(Debug, PartialEq)]
pub struct RoutePlan {
    pub write: Vec<String>,
    pub delete: Vec<String>,
    /// Apps that carry our value afterwards, kept in config.
    pub routed: Vec<String>,
}

/// `existing` pairs each installed app with its current preference value.
/// A value the user set is never overwritten, and only values this app
/// wrote, still unchanged, are removed.
pub fn route_plan(route: bool, existing: &[(String, Option<String>)], routed: &[String]) -> RoutePlan {
    let ours = preference_value(1);
    if route {
        let write: Vec<String> = existing.iter().filter(|(_, v)| v.is_none()).map(|(a, _)| a.clone()).collect();
        let mut all: Vec<String> = routed.to_vec();
        all.extend(write.iter().cloned());
        all.sort();
        all.dedup();
        RoutePlan { write, delete: Vec::new(), routed: all }
    } else {
        let delete = existing
            .iter()
            .filter(|(a, v)| routed.contains(a) && v.as_deref() == Some(ours.as_str()))
            .map(|(a, _)| a.clone())
            .collect();
        RoutePlan { write: Vec::new(), delete, routed: Vec::new() }
    }
}

fn read_preference(app: &str) -> Option<String> {
    let mut buf = [0u16; 128];
    let mut size = (buf.len() * 2) as u32;
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &HSTRING::from(GPU_PREF_KEY),
            &HSTRING::from(app),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut _),
            Some(&mut size),
        )
    }
    .ok()
    .ok()?;
    let len = (size as usize / 2).saturating_sub(1);
    Some(String::from_utf16_lossy(&buf[..len.min(buf.len())]))
}

/// On battery, ask Windows to run the listed apps on the iGPU; otherwise
/// remove what this app set. Takes effect the next time each app starts.
/// Returns the apps now carrying our value.
pub fn set_apps_gpu(route: bool, routed: &[String]) -> Vec<String> {
    let existing: Vec<(String, Option<String>)> = APPS
        .iter()
        .map(|a| expand(a))
        .filter(|p| std::path::Path::new(p).exists())
        .map(|p| {
            let v = read_preference(&p);
            (p, v)
        })
        .collect();
    let plan = route_plan(route, &existing, routed);
    unsafe {
        let mut key = HKEY::default();
        if RegCreateKeyExW(HKEY_CURRENT_USER, &HSTRING::from(GPU_PREF_KEY), None, PCWSTR::null(), REG_OPTION_NON_VOLATILE, KEY_SET_VALUE, None, &mut key, None).is_err() {
            return routed.to_vec();
        }
        let data: Vec<u16> = preference_value(1).encode_utf16().chain([0]).collect();
        let bytes = std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 2);
        for app in &plan.write {
            let _ = RegSetValueExW(key, &HSTRING::from(app.as_str()), None, REG_SZ, Some(bytes));
        }
        for app in &plan.delete {
            let _ = RegDeleteValueW(key, &HSTRING::from(app.as_str()));
        }
        let _ = RegCloseKey(key);
    }
    plan.routed
}

fn devnode(id: &str) -> Option<u32> {
    let mut inst = 0u32;
    let wide = HSTRING::from(id);
    (unsafe { CM_Locate_DevNodeW(&mut inst, &wide, CM_LOCATE_DEVNODE_NORMAL) } == CR_SUCCESS).then_some(inst)
}

fn devnode_prop(inst: u32, key: &windows::Win32::Foundation::DEVPROPKEY) -> Option<Vec<u8>> {
    let mut kind = DEVPROPTYPE::default();
    let mut buf = vec![0u8; 256];
    let mut size = buf.len() as u32;
    let r = unsafe { CM_Get_DevNode_PropertyW(inst, key, &mut kind, Some(buf.as_mut_ptr()), &mut size, 0) };
    (r == CR_SUCCESS).then(|| {
        buf.truncate(size as usize);
        buf
    })
}

/// Instance ID of the NVIDIA display adapter (not its HD Audio sibling).
pub fn find_dgpu() -> Option<String> {
    let filter = HSTRING::from("PCI");
    let flags = CM_GETIDLIST_FILTER_ENUMERATOR | CM_GETIDLIST_FILTER_PRESENT;
    let mut len = 0u32;
    if unsafe { CM_Get_Device_ID_List_SizeW(&mut len, &filter, flags) } != CR_SUCCESS {
        return None;
    }
    let mut buf = vec![0u16; len as usize];
    if unsafe { CM_Get_Device_ID_ListW(&filter, &mut buf, flags) } != CR_SUCCESS {
        return None;
    }
    String::from_utf16_lossy(&buf)
        .split('\0')
        .filter(|id| id.to_uppercase().contains("VEN_10DE"))
        .find(|id| {
            devnode(id)
                .and_then(|n| devnode_prop(n, &DEVPKEY_Device_Class))
                .map(|raw| String::from_utf16_lossy(&raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect::<Vec<_>>()))
                .is_some_and(|class| class.trim_end_matches('\0') == "Display")
        })
        .map(str::to_string)
}

#[derive(serde::Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum DState {
    D0,
    D3,
    Other,
}

/// Cached power state from the PnP manager; reading it never wakes the GPU.
pub fn power_state(id: &str) -> Option<DState> {
    let raw = devnode(id).and_then(|n| devnode_prop(n, &DEVPKEY_Device_PowerData))?;
    let state = u32::from_le_bytes(raw.get(4..8)?.try_into().ok()?);
    Some(match state {
        1 => DState::D0,
        4 => DState::D3,
        _ => DState::Other,
    })
}

/// Reloads the driver so the card re-evaluates its power state. This is a
/// restart, not a disable: disabling removes the power-policy owner.
pub fn restart(id: &str) -> bool {
    tool("pnputil.exe").args(["/restart-device", id]).output().map(|o| o.status.success()).unwrap_or(false)
}

#[derive(serde::Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpuView {
    pub state: Option<DState>,
    pub igpu_on_battery: bool,
    pub auto_reset: bool,
}

/// App-side state for the once-a-minute dGPU check.
pub struct GpuCtl {
    pub id: Option<String>,
    pub guard: std::sync::Mutex<ResetGuard>,
    pub last_battery: std::sync::Mutex<Option<bool>>,
}

impl GpuCtl {
    pub fn new() -> Self {
        GpuCtl { id: find_dgpu(), guard: Default::default(), last_battery: Default::default() }
    }

    pub fn state(&self) -> Option<DState> {
        self.id.as_deref().and_then(power_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs(now: Instant) -> GpuInputs {
        GpuInputs { now, enabled: true, on_battery: true, d0: true, fullscreen: false, idle: Duration::from_secs(600) }
    }

    fn run(g: &mut ResetGuard, i: &GpuInputs, n: usize) -> Vec<bool> {
        (0..n).map(|_| g.tick(i, || Some(false))).collect()
    }

    #[test]
    fn reset_needs_five_stuck_minutes() {
        let mut g = ResetGuard::default();
        assert_eq!(run(&mut g, &inputs(Instant::now()), 5), vec![false, false, false, false, true]);
    }

    #[test]
    fn reset_waits_for_idle_and_no_fullscreen() {
        let mut g = ResetGuard::default();
        let busy_user = GpuInputs { idle: Duration::from_secs(30), ..inputs(Instant::now()) };
        assert!(run(&mut g, &busy_user, 10).iter().all(|r| !r));
        let gaming = GpuInputs { fullscreen: true, ..inputs(Instant::now()) };
        assert!(run(&mut g, &gaming, 10).iter().all(|r| !r));
    }

    #[test]
    fn reset_respects_cooldown() {
        let t0 = Instant::now();
        let mut g = ResetGuard::default();
        assert!(run(&mut g, &inputs(t0), 5)[4]);
        assert!(run(&mut g, &inputs(t0 + Duration::from_secs(600)), 5).iter().all(|r| !r));
        // Still stuck once the cooldown ends: no need to wait another five minutes.
        assert!(run(&mut g, &inputs(t0 + Duration::from_secs(1900)), 1)[0]);
    }

    #[test]
    fn busy_or_unknown_gpu_is_left_alone() {
        let mut g = ResetGuard::default();
        let i = inputs(Instant::now());
        assert!((0..5).all(|_| !g.tick(&i, || Some(true))));
        assert!((0..5).all(|_| !g.tick(&i, || None)));
    }

    #[test]
    fn routing_never_overrides_user_choice() {
        let existing = vec![("chrome".to_string(), Some("GpuPreference=2;".to_string())), ("code".to_string(), None)];
        let plan = route_plan(true, &existing, &[]);
        assert_eq!(plan.write, vec!["code".to_string()]);
        assert!(plan.delete.is_empty());
        assert_eq!(plan.routed, vec!["code".to_string()]);
    }

    #[test]
    fn routing_only_removes_own_values() {
        let existing = vec![
            ("chrome".to_string(), Some("GpuPreference=2;".to_string())),
            ("code".to_string(), Some("GpuPreference=1;".to_string())),
            ("slack".to_string(), Some("GpuPreference=1;".to_string())),
        ];
        // chrome was changed by the user since; slack was never ours.
        let plan = route_plan(false, &existing, &["chrome".to_string(), "code".to_string()]);
        assert_eq!(plan.delete, vec!["code".to_string()]);
        assert!(plan.write.is_empty());
        assert!(plan.routed.is_empty());
    }

    #[test]
    fn preference_value_format() {
        assert_eq!(preference_value(1), "GpuPreference=1;");
    }
}

#[cfg(test)]
mod probe {
    /// Read-only: `cargo test gpu::probe -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe() {
        let id = super::find_dgpu();
        println!("dgpu {id:?}");
        if let Some(id) = id {
            println!("state {:?}", super::power_state(&id));
        }
    }
}
