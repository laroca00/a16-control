//! Firmware switches this BIOS answers besides the fans: the charge limit,
//! Fn and Windows key locks, the touchpad, the power and battery LEDs, the
//! boot animation and the GPU mode for the next boot. Method numbers and
//! values come from the BIOS's own WMBD/WMBC dispatchers; see
//! docs/firmware-methods.md.

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::hw::{GbWmi, HwError};

/// Charge limits offered; 100 means charging is never stopped early.
pub const CHARGE_LIMITS: [u32; 3] = [60, 80, 100];

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Toggle {
    FnLock,
    WinLock,
    Touchpad,
    PowerLed,
    BatteryLed,
    BootAnimation,
}

impl Toggle {
    pub const ALL: [Toggle; 6] =
        [Toggle::FnLock, Toggle::WinLock, Toggle::Touchpad, Toggle::PowerLed, Toggle::BatteryLed, Toggle::BootAnimation];

    fn setter(self) -> &'static str {
        match self {
            Toggle::FnLock => "SetFnKeyLockStatus",
            Toggle::WinLock => "SetWinKeyLockStatus",
            Toggle::Touchpad => "SetTouchPadStatus",
            Toggle::PowerLed => "SetPwrLightDisplay",
            Toggle::BatteryLed => "SetBatteryLightDisplay",
            Toggle::BootAnimation => "SetPostAnimate",
        }
    }

    /// None for the Windows key lock: the BIOS can set it but not report it.
    fn getter(self) -> Option<&'static str> {
        match self {
            Toggle::FnLock => Some("getFnKeyLockStatus"),
            Toggle::WinLock => None,
            Toggle::Touchpad => Some("getTouchPadStatus"),
            Toggle::PowerLed => Some("getPwrLightDisplay"),
            Toggle::BatteryLed => Some("getBatteryLightDisplay"),
            Toggle::BootAnimation => Some("getPostAnimate"),
        }
    }

    /// The EC's touchpad bit means "switched off"; every other bit means "on".
    fn raw(self, on: bool) -> u32 {
        match self {
            Toggle::Touchpad => u32::from(!on),
            _ => u32::from(on),
        }
    }

    fn from_raw(self, v: u32) -> bool {
        match self {
            Toggle::Touchpad => v == 0,
            _ => v != 0,
        }
    }
}

/// Which GPU drives the internal panel. The BIOS reads the choice from CMOS
/// at power-on, so a change takes effect after a restart.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub enum GpuMode {
    Hybrid,
    Discrete,
}

impl GpuMode {
    fn raw(self) -> u32 {
        match self {
            GpuMode::Hybrid => 0,
            GpuMode::Discrete => 1,
        }
    }

    fn from_raw(v: u32) -> Option<GpuMode> {
        match v {
            0 => Some(GpuMode::Hybrid),
            1 => Some(GpuMode::Discrete),
            _ => None,
        }
    }
}

fn mismatch(method: &str, wrote: u32, read: u32) -> HwError {
    HwError::Call { method: method.to_string(), detail: format!("wrote {wrote}, firmware reports {read}") }
}

pub fn read_charge_limit(wmi: &dyn GbWmi) -> Result<u32, HwError> {
    if wmi.get("GetChargePolicy")? == 0 {
        return Ok(100);
    }
    let stop = wmi.get("GetChargeStop")?;
    Ok(if (1..100).contains(&stop) { stop } else { 100 })
}

pub fn write_charge_limit(wmi: &dyn GbWmi, limit: u32) -> Result<(), HwError> {
    if !CHARGE_LIMITS.contains(&limit) {
        return Err(HwError::Call { method: "SetChargeStop".into(), detail: format!("unsupported limit {limit}") });
    }
    if limit == 100 {
        wmi.set("SetChargePolicy", 0)?;
    } else {
        // The limit goes first so the policy never stops at a stale value.
        wmi.set("SetChargeStop", limit)?;
        wmi.set("SetChargePolicy", 1)?;
    }
    let now = read_charge_limit(wmi)?;
    if now != limit {
        return Err(mismatch("SetChargeStop", limit, now));
    }
    Ok(())
}

pub fn read_toggle(wmi: &dyn GbWmi, t: Toggle) -> Option<Result<bool, HwError>> {
    t.getter().map(|g| wmi.get(g).map(|v| t.from_raw(v)))
}

pub fn write_toggle(wmi: &dyn GbWmi, t: Toggle, on: bool) -> Result<(), HwError> {
    wmi.set(t.setter(), t.raw(on))?;
    if let Some(Ok(now)) = read_toggle(wmi, t) {
        if now != on {
            return Err(mismatch(t.setter(), t.raw(on), t.raw(now)));
        }
    }
    Ok(())
}

pub fn read_gpu_mode(wmi: &dyn GbWmi) -> Result<Option<GpuMode>, HwError> {
    Ok(GpuMode::from_raw(wmi.get("GetPEG2orSG2")?))
}

pub fn write_gpu_mode(wmi: &dyn GbWmi, mode: GpuMode) -> Result<(), HwError> {
    wmi.set("SetPEG2orSG2", mode.raw())?;
    let now = wmi.get("GetPEG2orSG2")?;
    if now != mode.raw() {
        return Err(mismatch("SetPEG2orSG2", mode.raw(), now));
    }
    Ok(())
}

/// Saved choices that the firmware may lose (the Windows key lock always
/// resets at power-on) and that background ticks need.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct HardwareOptions {
    /// None until the user picks one, so a limit set elsewhere is left alone.
    pub charge_limit: Option<u32>,
    pub win_lock: bool,
    /// Lock the Windows key while a fullscreen window has the focus.
    pub win_lock_in_games: bool,
}

impl Default for HardwareOptions {
    fn default() -> Self {
        HardwareOptions { charge_limit: None, win_lock: false, win_lock_in_games: false }
    }
}

/// Whether the Windows key should be locked right now.
pub fn want_win_lock(o: &HardwareOptions, fullscreen: bool) -> bool {
    o.win_lock || (o.win_lock_in_games && fullscreen)
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BatteryHealth {
    /// mWh
    pub design: u32,
    /// mWh
    pub full: u32,
    pub cycles: Option<u32>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HardwareView {
    /// The setter methods are registered; false means setup must run again.
    pub supported: bool,
    pub charge_limit: Option<u32>,
    pub toggles: BTreeMap<Toggle, bool>,
    pub win_lock_in_games: bool,
    /// What the next boot uses.
    pub gpu_mode: Option<GpuMode>,
    /// What this boot uses, from the adapter that drives the desktop.
    pub gpu_active: Option<GpuMode>,
    pub battery: Option<BatteryHealth>,
}

pub fn view(
    wmi: Option<&dyn GbWmi>,
    supported: bool,
    o: &HardwareOptions,
    win_locked: Option<bool>,
    gpu_active: Option<GpuMode>,
    battery: Option<BatteryHealth>,
) -> HardwareView {
    let mut toggles = BTreeMap::new();
    let (mut charge_limit, mut gpu_mode) = (None, None);
    if let Some(w) = wmi.filter(|_| supported) {
        charge_limit = read_charge_limit(w).ok();
        gpu_mode = read_gpu_mode(w).ok().flatten();
        for t in Toggle::ALL {
            let on = match read_toggle(w, t) {
                Some(r) => r.ok(),
                None => Some(win_locked.unwrap_or(o.win_lock)),
            };
            if let Some(on) = on {
                toggles.insert(t, on);
            }
        }
    }
    HardwareView { supported, charge_limit, toggles, win_lock_in_games: o.win_lock_in_games, gpu_mode, gpu_active, battery }
}

/// What the background tick last wrote for the Windows key, so it writes only on change.
#[derive(Default)]
pub struct HardwareCtl {
    pub win_written: Mutex<Option<bool>>,
}

// ---- App glue -------------------------------------------------------------

use tauri::{AppHandle, Manager};

use crate::state::AppState;

/// Methods the features above need; an older install registered none of them.
pub const SETTERS: [&str; 9] = [
    "SetChargePolicy",
    "SetChargeStop",
    "SetFnKeyLockStatus",
    "SetWinKeyLockStatus",
    "SetTouchPadStatus",
    "SetPwrLightDisplay",
    "SetBatteryLightDisplay",
    "SetPostAnimate",
    "SetPEG2orSG2",
];

pub fn supported() -> bool {
    crate::hw::com::class_methods("GB_WMIACPI_Set")
        .map(|present| crate::setup::missing_methods(&present, &SETTERS).is_empty())
        .unwrap_or(false)
}

fn options(app: &AppHandle) -> HardwareOptions {
    app.state::<AppState>().config.lock().unwrap().hardware
}

pub fn active_gpu() -> Option<GpuMode> {
    let name = crate::os::primary_adapter()?.to_ascii_lowercase();
    if name.contains("nvidia") {
        Some(GpuMode::Discrete)
    } else if name.contains("amd") || name.contains("radeon") {
        Some(GpuMode::Hybrid)
    } else {
        None
    }
}

pub fn battery_health() -> Option<BatteryHealth> {
    use crate::hw::com::first_u32;
    let design = first_u32("BatteryStaticData", "DesignedCapacity").ok().filter(|&v| v > 0)?;
    let full = first_u32("BatteryFullChargedCapacity", "FullChargedCapacity").ok().filter(|&v| v > 0)?;
    let cycles = first_u32("BatteryCycleCount", "CycleCount").ok().filter(|&v| v > 0);
    Some(BatteryHealth { design, full, cycles })
}

pub fn current_view(app: &AppHandle) -> HardwareView {
    let st = app.state::<AppState>();
    let wmi = st.wmi().ok();
    let win = *app.state::<HardwareCtl>().win_written.lock().unwrap();
    view(wmi.as_deref(), supported(), &options(app), win, active_gpu(), battery_health())
}

/// Puts back what the firmware forgets: the Windows key lock always, the
/// charge limit when something else changed it.
pub fn restore(app: &AppHandle) {
    let o = options(app);
    let Ok(wmi) = app.state::<AppState>().wmi() else { return };
    if let Some(limit) = o.charge_limit {
        if read_charge_limit(wmi.as_ref()).ok() != Some(limit) {
            if let Err(e) = write_charge_limit(wmi.as_ref(), limit) {
                eprintln!("charge limit not restored: {e}");
            }
        }
    }
    tick(app);
}

/// Locks or unlocks the Windows key as games come and go. Runs on every
/// background tick; writes only on a change.
pub fn tick(app: &AppHandle) {
    let o = options(app);
    let ctl = app.state::<HardwareCtl>();
    let written = *ctl.win_written.lock().unwrap();
    // Nothing to do for someone who never used the lock.
    if written.is_none() && !o.win_lock && !o.win_lock_in_games {
        return;
    }
    let fullscreen = o.win_lock_in_games && crate::os::foreground_fullscreen();
    let want = want_win_lock(&o, fullscreen);
    if written == Some(want) {
        return;
    }
    let Ok(wmi) = app.state::<AppState>().wmi() else { return };
    if write_toggle(wmi.as_ref(), Toggle::WinLock, want).is_ok() {
        *ctl.win_written.lock().unwrap() = Some(want);
    }
}

/// Before quitting: a lock left on by a game would outlive the app.
pub fn release(app: &AppHandle) {
    let o = options(app);
    if *app.state::<HardwareCtl>().win_written.lock().unwrap() == Some(true) && !o.win_lock {
        if let Ok(wmi) = app.state::<AppState>().wmi() {
            let _ = write_toggle(wmi.as_ref(), Toggle::WinLock, false);
        }
    }
}

pub fn set_charge_limit(app: &AppHandle, limit: u32) -> Result<HardwareView, String> {
    let wmi = app.state::<AppState>().wmi()?;
    write_charge_limit(wmi.as_ref(), limit).map_err(|e| e.to_string())?;
    save(app, |o| o.charge_limit = Some(limit));
    Ok(current_view(app))
}

pub fn set_toggle(app: &AppHandle, toggle: Toggle, on: bool) -> Result<HardwareView, String> {
    let wmi = app.state::<AppState>().wmi()?;
    write_toggle(wmi.as_ref(), toggle, on).map_err(|e| e.to_string())?;
    if toggle == Toggle::WinLock {
        save(app, |o| o.win_lock = on);
        *app.state::<HardwareCtl>().win_written.lock().unwrap() = Some(on);
    }
    Ok(current_view(app))
}

pub fn set_win_lock_in_games(app: &AppHandle, on: bool) -> HardwareView {
    save(app, |o| o.win_lock_in_games = on);
    tick(app);
    current_view(app)
}

pub fn set_gpu_mode(app: &AppHandle, mode: GpuMode) -> Result<HardwareView, String> {
    let wmi = app.state::<AppState>().wmi()?;
    write_gpu_mode(wmi.as_ref(), mode).map_err(|e| e.to_string())?;
    Ok(current_view(app))
}

fn save(app: &AppHandle, change: impl FnOnce(&mut HardwareOptions)) {
    let st = app.state::<AppState>();
    let mut cfg = st.config.lock().unwrap();
    change(&mut cfg.hardware);
    let _ = crate::config::save(&cfg);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hw::fake::FakeWmi;

    #[test]
    fn full_charge_turns_the_policy_off() {
        let w = FakeWmi::default();
        w.put("GetChargePolicy", 0);
        write_charge_limit(&w, 100).unwrap();
        assert_eq!(w.calls(), vec![("SetChargePolicy".to_string(), 0)]);
    }

    #[test]
    fn a_limit_is_written_before_the_policy() {
        let w = FakeWmi::default();
        w.put("GetChargePolicy", 1);
        w.put("GetChargeStop", 80);
        write_charge_limit(&w, 80).unwrap();
        assert_eq!(w.calls(), vec![("SetChargeStop".to_string(), 80), ("SetChargePolicy".to_string(), 1)]);
    }

    #[test]
    fn a_limit_that_does_not_hold_is_an_error() {
        // The fake ignores writes, so the policy still reads 0 (full charge).
        let w = FakeWmi::default();
        assert!(write_charge_limit(&w, 60).is_err());
    }

    #[test]
    fn odd_limits_are_refused_without_a_write() {
        let w = FakeWmi::default();
        assert!(write_charge_limit(&w, 55).is_err());
        assert!(w.calls().is_empty());
    }

    #[test]
    fn charge_limit_reads_back() {
        let w = FakeWmi::default();
        assert_eq!(read_charge_limit(&w).unwrap(), 100);
        w.put("GetChargePolicy", 1);
        w.put("GetChargeStop", 60);
        assert_eq!(read_charge_limit(&w).unwrap(), 60);
        // A stop value of 0 with the policy on charges fully.
        w.put("GetChargeStop", 0);
        assert_eq!(read_charge_limit(&w).unwrap(), 100);
    }

    #[test]
    fn the_touchpad_bit_is_inverted() {
        let w = FakeWmi::default();
        w.put("getTouchPadStatus", 1);
        write_toggle(&w, Toggle::Touchpad, false).unwrap();
        assert_eq!(w.calls(), vec![("SetTouchPadStatus".to_string(), 1)]);
        w.put("getTouchPadStatus", 0);
        assert_eq!(read_toggle(&w, Toggle::Touchpad).unwrap().unwrap(), true);
    }

    #[test]
    fn the_windows_key_lock_cannot_be_read() {
        let w = FakeWmi::default();
        assert!(read_toggle(&w, Toggle::WinLock).is_none());
        write_toggle(&w, Toggle::WinLock, true).unwrap();
        assert_eq!(w.calls(), vec![("SetWinKeyLockStatus".to_string(), 1)]);
    }

    #[test]
    fn gpu_mode_round_trips() {
        let w = FakeWmi::default();
        w.put("GetPEG2orSG2", 1);
        assert_eq!(read_gpu_mode(&w).unwrap(), Some(GpuMode::Discrete));
        assert!(write_gpu_mode(&w, GpuMode::Hybrid).is_err(), "fake keeps 1, so the write must not count");
        w.put("GetPEG2orSG2", 0);
        write_gpu_mode(&w, GpuMode::Hybrid).unwrap();
        assert_eq!(read_gpu_mode(&w).unwrap(), Some(GpuMode::Hybrid));
    }

    #[test]
    fn win_lock_follows_games_when_asked() {
        let mut o = HardwareOptions::default();
        assert!(!want_win_lock(&o, true));
        o.win_lock_in_games = true;
        assert!(want_win_lock(&o, true));
        assert!(!want_win_lock(&o, false));
        o.win_lock = true;
        assert!(want_win_lock(&o, false));
    }

    #[test]
    fn unsupported_firmware_shows_nothing() {
        let w = FakeWmi::default();
        let v = view(Some(&w), false, &HardwareOptions::default(), None, None, None);
        assert!(v.toggles.is_empty() && v.charge_limit.is_none() && v.gpu_mode.is_none());
    }
}
