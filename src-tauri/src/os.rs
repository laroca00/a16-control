//! Windows-side levers used by the power modes: the power-mode slider, the
//! power source, refresh rate, brightness, fullscreen and GPU load detection,
//! and the battery power plan.

use std::time::Duration;

use windows::core::{s, w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, FILETIME, HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    ChangeDisplaySettingsExW, EnumDisplaySettingsW, GetMonitorInfoW, MonitorFromWindow, CDS_TEST,
    CDS_UPDATEREGISTRY, DEVMODEW, DISP_CHANGE_SUCCESSFUL, DM_DISPLAYFREQUENCY, ENUM_CURRENT_SETTINGS,
    ENUM_DISPLAY_SETTINGS_MODE, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
    PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Power::{
    CallNtPowerInformation, GetSystemPowerStatus, SystemBatteryState, SYSTEM_BATTERY_STATE, SYSTEM_POWER_STATUS,
};
use windows::Win32::System::Threading::{
    GetProcessTimes, GetSystemTimes, OpenProcess, ProcessPowerThrottling, SetProcessInformation,
    PROCESS_POWER_THROTTLING_CURRENT_VERSION, PROCESS_POWER_THROTTLING_EXECUTION_SPEED, PROCESS_POWER_THROTTLING_STATE,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_INFORMATION,
};
use windows::Win32::System::SystemInformation::GetTickCount;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetDesktopWindow, GetForegroundWindow, GetShellWindow, GetWindowLongW, GetWindowRect, IsZoomed,
    GetWindowThreadProcessId, GWL_STYLE, WS_CAPTION,
};

use crate::elevation::tool;
use crate::power::Refresh;

/// Same memory layout as the Win32 GUID, so it can be passed to the API by value.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Guid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

pub fn parse_guid(s: &str) -> Option<Guid> {
    let hex: String = s.chars().filter(|c| *c != '-').collect();
    if hex.len() != 32 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok();
    let mut data4 = [0u8; 8];
    for (k, b) in data4.iter_mut().enumerate() {
        *b = byte(8 + k)?;
    }
    Some(Guid {
        data1: u32::from_str_radix(&hex[0..8], 16).ok()?,
        data2: u16::from_str_radix(&hex[8..12], 16).ok()?,
        data3: u16::from_str_radix(&hex[12..16], 16).ok()?,
        data4,
    })
}

/// Sets the Windows power-mode slider. The API is exported by powrprof.dll
/// but not declared in the SDK headers, so it is looked up at run time.
pub fn set_overlay(guid: &str) -> bool {
    let Some(g) = parse_guid(guid) else { return false };
    unsafe {
        let Ok(lib) = LoadLibraryW(w!("powrprof.dll")) else { return false };
        let Some(f) = GetProcAddress(lib, s!("PowerSetActiveOverlayScheme")) else { return false };
        let f: extern "system" fn(Guid) -> u32 = std::mem::transmute(f);
        f(g) == 0
    }
}

/// (on AC, battery percent); None where Windows does not know.
pub fn power_status() -> (Option<bool>, Option<u8>) {
    let mut s = SYSTEM_POWER_STATUS::default();
    if unsafe { GetSystemPowerStatus(&mut s) }.is_err() {
        return (None, None);
    }
    let ac = match s.ACLineStatus {
        1 => Some(true),
        0 => Some(false),
        _ => None,
    };
    let pct = (s.BatteryLifePercent <= 100).then_some(s.BatteryLifePercent);
    (ac, pct)
}

pub fn pick_max(modes: &[(u32, u32, u32)], current: (u32, u32)) -> Option<u32> {
    modes.iter().filter(|m| (m.0, m.1) == current).map(|m| m.2).max()
}

fn devmode() -> DEVMODEW {
    DEVMODEW { dmSize: std::mem::size_of::<DEVMODEW>() as u16, ..Default::default() }
}

fn current_mode() -> Option<DEVMODEW> {
    let mut dm = devmode();
    unsafe { EnumDisplaySettingsW(PCWSTR::null(), ENUM_CURRENT_SETTINGS, &mut dm) }.as_bool().then_some(dm)
}

/// Refresh rates the panel offers at its current resolution, highest first.
pub fn refresh_rates() -> Vec<u32> {
    let Some(cur) = current_mode() else { return Vec::new() };
    let mut rates = Vec::new();
    let mut i = 0;
    loop {
        let mut dm = devmode();
        if !unsafe { EnumDisplaySettingsW(PCWSTR::null(), ENUM_DISPLAY_SETTINGS_MODE(i), &mut dm) }.as_bool() {
            break;
        }
        if (dm.dmPelsWidth, dm.dmPelsHeight) == (cur.dmPelsWidth, cur.dmPelsHeight) && !rates.contains(&dm.dmDisplayFrequency) {
            rates.push(dm.dmDisplayFrequency);
        }
        i += 1;
    }
    rates.sort_unstable_by(|a, b| b.cmp(a));
    rates
}

/// Time since the last keyboard or mouse input anywhere in the session.
pub fn user_idle() -> Duration {
    let mut lii = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
    if !unsafe { GetLastInputInfo(&mut lii) }.as_bool() {
        return Duration::ZERO;
    }
    Duration::from_millis(unsafe { GetTickCount() }.wrapping_sub(lii.dwTime) as u64)
}

pub fn current_refresh() -> Option<u32> {
    current_mode().map(|dm| dm.dmDisplayFrequency)
}

fn max_refresh() -> Option<u32> {
    let cur = current_mode()?;
    let mut modes = Vec::new();
    let mut i = 0;
    loop {
        let mut dm = devmode();
        if !unsafe { EnumDisplaySettingsW(PCWSTR::null(), ENUM_DISPLAY_SETTINGS_MODE(i), &mut dm) }.as_bool() {
            break;
        }
        modes.push((dm.dmPelsWidth, dm.dmPelsHeight, dm.dmDisplayFrequency));
        i += 1;
    }
    pick_max(&modes, (cur.dmPelsWidth, cur.dmPelsHeight))
}

/// Changes the primary panel's refresh rate; true if a change was committed.
pub fn set_refresh(r: Refresh) -> bool {
    let hz = match r {
        Refresh::Leave => return false,
        Refresh::Hz(hz) => hz,
        Refresh::Max => match max_refresh() {
            Some(hz) => hz,
            None => return false,
        },
    };
    let Some(mut dm) = current_mode() else { return false };
    if dm.dmDisplayFrequency == hz {
        return false;
    }
    dm.dmFields = DM_DISPLAYFREQUENCY;
    dm.dmDisplayFrequency = hz;
    unsafe {
        ChangeDisplaySettingsExW(PCWSTR::null(), Some(&dm), None, CDS_TEST, None) == DISP_CHANGE_SUCCESSFUL
            && ChangeDisplaySettingsExW(PCWSTR::null(), Some(&dm), None, CDS_UPDATEREGISTRY, None)
                == DISP_CHANGE_SUCCESSFUL
    }
}

fn class_name(hwnd: HWND) -> String {
    let mut buf = [0u16; 64];
    let n = unsafe { GetClassNameW(hwnd, &mut buf) } as usize;
    String::from_utf16_lossy(&buf[..n])
}

/// True when the foreground window covers its whole monitor, the way a game
/// or a fullscreen video does. The desktop and shell never count.
pub fn foreground_fullscreen() -> bool {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.is_invalid() || fg == GetShellWindow() || fg == GetDesktopWindow() {
            return false;
        }
        if matches!(class_name(fg).as_str(), "WorkerW" | "Progman" | "Shell_TrayWnd") {
            return false;
        }
        let mut r = RECT::default();
        if GetWindowRect(fg, &mut r).is_err() {
            return false;
        }
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if !GetMonitorInfoW(MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST), &mut mi).as_bool() {
            return false;
        }
        let style = GetWindowLongW(fg, GWL_STYLE) as u32;
        looks_fullscreen(
            (r.left, r.top, r.right, r.bottom),
            (mi.rcMonitor.left, mi.rcMonitor.top, mi.rcMonitor.right, mi.rcMonitor.bottom),
            IsZoomed(fg).as_bool(),
            style & WS_CAPTION.0 == WS_CAPTION.0,
        )
    }
}

/// A maximized or captioned window can also cover the monitor when the
/// taskbar reserves no space (auto-hide, third-party docks); games run
/// borderless and unmaximized, so only those count.
pub fn looks_fullscreen(win: (i32, i32, i32, i32), mon: (i32, i32, i32, i32), zoomed: bool, captioned: bool) -> bool {
    !zoomed && !captioned && win.0 <= mon.0 && win.1 <= mon.1 && win.2 >= mon.2 && win.3 >= mon.3
}

/// Total 3D-engine utilisation across all GPUs, in percent. Reading this
/// wakes a sleeping dGPU, so callers only ask while a fullscreen app runs.
pub fn gpu_3d_util() -> Option<f32> {
    unsafe {
        let mut query = PDH_HQUERY::default();
        if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) != 0 {
            return None;
        }
        let result = (|| {
            let mut counter = PDH_HCOUNTER::default();
            let path = HSTRING::from(r"\GPU Engine(*engtype_3D)\Utilization Percentage");
            if PdhAddEnglishCounterW(query, &path, 0, &mut counter) != 0 {
                return None;
            }
            // Rates need two samples.
            PdhCollectQueryData(query);
            std::thread::sleep(Duration::from_millis(250));
            if PdhCollectQueryData(query) != 0 {
                return None;
            }
            let (mut size, mut count) = (0u32, 0u32);
            if PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, None) != PDH_MORE_DATA {
                return None;
            }
            let items = (size as usize).div_ceil(std::mem::size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>());
            let mut buf = vec![PDH_FMT_COUNTERVALUE_ITEM_W::default(); items.max(1)];
            if PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, Some(buf.as_mut_ptr())) != 0 {
                return None;
            }
            let total: f64 = buf[..count as usize].iter().map(|it| it.FmtValue.Anonymous.doubleValue).sum();
            Some(total.min(100.0) as f32)
        })();
        let _ = PdhCloseQuery(query);
        result
    }
}

/// Battery-side power-plan values that cost no responsiveness: USB selective
/// suspend, PCIe ASPM, efficient boost and Wi-Fi power saving.
pub fn tune_battery_plan() {
    const SETTINGS: [(&str, &str, &str); 4] = [
        ("2a737441-1930-4402-8d77-b2bebba308a3", "48e6b7a6-50f5-4782-a5d4-53bb8f07e226", "1"),
        ("501a4d13-42af-4429-9fd1-a8218c268e20", "ee12f906-d277-404b-b6da-e5fa1a576df5", "2"),
        ("54533251-82be-4824-96c1-47b60b740d00", "be337238-0d82-4146-a960-4f3749d470c7", "3"),
        ("19cbb8fa-5279-450e-9fac-8a3d5fedd0c1", "12bbebe6-58d6-4636-95bb-3217ef867c1a", "3"),
    ];
    for (sub, setting, value) in SETTINGS {
        let _ = tool("powercfg.exe").args(["/setdcvalueindex", "SCHEME_CURRENT", sub, setting, value]).output();
    }
    let _ = tool("powercfg.exe").args(["/setactive", "SCHEME_CURRENT"]).output();
}

/// Battery draw in watts while discharging; None on AC or if unknown.
pub fn battery_watts() -> Option<f32> {
    let mut s = SYSTEM_BATTERY_STATE::default();
    let ok = unsafe {
        CallNtPowerInformation(
            SystemBatteryState,
            None,
            0,
            Some(&mut s as *mut _ as *mut _),
            std::mem::size_of::<SYSTEM_BATTERY_STATE>() as u32,
        )
    };
    // Rate is a signed milliwatt value packed in an unsigned field; negative while discharging.
    (ok.is_ok() && s.Discharging).then(|| (s.Rate as i32).unsigned_abs() as f32 / 1000.0)
}

/// (idle, kernel, user) in 100 ns units; kernel includes idle.
pub fn system_times() -> Option<(u64, u64, u64)> {
    let (mut i, mut k, mut u) = (FILETIME::default(), FILETIME::default(), FILETIME::default());
    unsafe { GetSystemTimes(Some(&mut i), Some(&mut k), Some(&mut u)) }.ok()?;
    let v = |f: FILETIME| ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64;
    Some((v(i), v(k), v(u)))
}

pub fn cpu_percent(prev: (u64, u64, u64), cur: (u64, u64, u64)) -> f32 {
    let idle = cur.0.saturating_sub(prev.0);
    let total = cur.1.saturating_sub(prev.1) + cur.2.saturating_sub(prev.2);
    if total == 0 {
        return 0.0;
    }
    (100.0 * total.saturating_sub(idle) as f32 / total as f32).clamp(0.0, 100.0)
}

/// pid → (image name without .exe, CPU time in 100 ns units).
pub fn process_cpu_snapshot() -> std::collections::HashMap<u32, (String, u64)> {
    let mut out = std::collections::HashMap::new();
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return out };
        let mut e = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut more = Process32FirstW(snap, &mut e).is_ok();
        while more {
            let len = e.szExeFile.iter().position(|c| *c == 0).unwrap_or(e.szExeFile.len());
            let name = String::from_utf16_lossy(&e.szExeFile[..len]);
            let name = name.strip_suffix(".exe").unwrap_or(&name).to_string();
            if let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, e.th32ProcessID) {
                let (mut c, mut x, mut k, mut u) = (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
                if GetProcessTimes(h, &mut c, &mut x, &mut k, &mut u).is_ok() {
                    let v = |f: FILETIME| ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64;
                    out.insert(e.th32ProcessID, (name, v(k) + v(u)));
                }
                let _ = CloseHandle(h);
            }
            more = Process32NextW(snap, &mut e).is_ok();
        }
        let _ = CloseHandle(snap);
    }
    out
}

pub fn foreground_pid() -> Option<u32> {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut pid)) };
    (pid != 0).then_some(pid)
}

/// Turns EcoQoS (execution-speed throttling) on or off for one process.
pub fn set_eco(pid: u32, on: bool) -> bool {
    unsafe {
        let Ok(h) = OpenProcess(PROCESS_SET_INFORMATION, false, pid) else { return false };
        let state = PROCESS_POWER_THROTTLING_STATE {
            Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
            StateMask: if on { PROCESS_POWER_THROTTLING_EXECUTION_SPEED } else { 0 },
        };
        let ok = SetProcessInformation(
            h,
            ProcessPowerThrottling,
            &state as *const _ as *const _,
            std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
        )
        .is_ok();
        let _ = CloseHandle(h);
        ok
    }
}

pub fn set_brightness(pct: u32) -> bool {
    crate::hw::com::set_brightness(pct.min(100)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_refresh_picks_highest_for_current_mode() {
        let modes = [(2560, 1600, 60), (2560, 1600, 165), (1920, 1200, 240), (2560, 1600, 120)];
        assert_eq!(pick_max(&modes, (2560, 1600)), Some(165));
        assert_eq!(pick_max(&modes, (800, 600)), None);
    }

    /// Read-only probe of the real system: `cargo test os::tests::probe -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe() {
        println!("power {:?}", power_status());
        println!("refresh {:?} max {:?}", current_refresh(), max_refresh());
        println!("fullscreen {}", foreground_fullscreen());
        println!("gpu3d {:?}", gpu_3d_util());
    }

    /// Flips the power-mode slider to performance and back:
    /// `cargo test os::tests::overlay_probe -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn overlay_probe() {
        let read = || {
            std::process::Command::new("reg")
                .args(["query", r"HKLM\SYSTEM\CurrentControlSet\Control\Power\User\PowerSchemes", "/v", "ActiveOverlayAcPowerScheme"])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).lines().filter(|l| l.contains("Overlay")).collect::<String>())
                .unwrap_or_default()
        };
        println!("before {}", read());
        println!("set performance -> {}", set_overlay(crate::power::OVERLAY_PERFORMANCE));
        println!("after  {}", read());
        println!("set balanced -> {}", set_overlay(crate::power::OVERLAY_BALANCED));
        println!("back   {}", read());
    }

    #[test]
    fn guid_layout_matches_windows() {
        let ours = parse_guid("ded574b5-45a0-4f42-8737-46345c09c238").unwrap();
        let theirs = windows::core::GUID::from_u128(0xded574b5_45a0_4f42_8737_46345c09c238);
        let a: [u8; 16] = unsafe { std::mem::transmute(ours) };
        let b: [u8; 16] = unsafe { std::mem::transmute(theirs) };
        assert_eq!(a, b);
    }

    #[test]
    fn maximized_or_captioned_windows_are_not_fullscreen() {
        let mon = (0, 0, 2560, 1600);
        assert!(looks_fullscreen(mon, mon, false, false));
        assert!(!looks_fullscreen((-8, -8, 2568, 1608), mon, true, false));
        assert!(!looks_fullscreen(mon, mon, false, true));
        assert!(!looks_fullscreen((0, 0, 1280, 800), mon, false, false));
    }

    #[test]
    fn parse_overlay_guid_roundtrip() {
        let g = parse_guid("ded574b5-45a0-4f42-8737-46345c09c238").unwrap();
        assert_eq!(g.data1, 0xded574b5);
        assert_eq!(g.data2, 0x45a0);
        assert_eq!(g.data3, 0x4f42);
        assert_eq!(g.data4, [0x87, 0x37, 0x46, 0x34, 0x5c, 0x09, 0xc2, 0x38]);
        assert!(parse_guid("nope").is_none());
    }
}
