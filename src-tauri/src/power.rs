//! Power modes. Profiles hold the firmware and OS settings for one mode; the
//! controller decides which profile applies from the user's choice and the
//! power source.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::fan::{self, FanMode};
use crate::hw::{GbWmi, HwError};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub enum ProfileId {
    Battery,
    BalancedAc,
    BalancedDc,
    Performance,
}

/// APU power limits in milliwatts.
#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Limits {
    pub fppt: u32,
    pub sppt: u32,
    pub spl: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Refresh {
    Leave,
    Hz(u32),
    Max,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Profile {
    pub id: ProfileId,
    pub limits: Limits,
    pub nv_boost: u32,
    pub fan: FanMode,
    pub overlay: &'static str,
    pub refresh: Refresh,
    pub brightness: Option<u32>,
}

// Windows power-mode slider ("power overlay") schemes.
pub const OVERLAY_EFFICIENCY: &str = "961cc777-2547-4f9d-8174-7d86181b8a7a";
pub const OVERLAY_BALANCED: &str = "00000000-0000-0000-0000-000000000000";
pub const OVERLAY_PERFORMANCE: &str = "ded574b5-45a0-4f42-8737-46345c09c238";

// The envelope GiMATE itself uses on this model (its GA63VH power table):
// nothing outside it is ever written.
const MIN_MW: u32 = 15_000;
const MAX_FPPT: u32 = 80_000;
const MAX_SPPT: u32 = 75_000;
const MAX_SPL: u32 = 40_000;

pub fn clamp(l: Limits) -> Limits {
    Limits {
        fppt: l.fppt.clamp(MIN_MW, MAX_FPPT),
        sppt: l.sppt.clamp(MIN_MW, MAX_SPPT),
        spl: l.spl.clamp(MIN_MW, MAX_SPL),
    }
}

pub fn profile(id: ProfileId) -> Profile {
    // Limits are GiMATE's own rows for this model: eco1, normal0, normal1
    // and turbo with the dGPU active.
    match id {
        ProfileId::Battery => Profile {
            id,
            limits: Limits { fppt: 30_000, sppt: 30_000, spl: 15_000 },
            nv_boost: 0,
            fan: FanMode::Auto,
            overlay: OVERLAY_EFFICIENCY,
            refresh: Refresh::Hz(60),
            brightness: Some(40),
        },
        ProfileId::BalancedAc => Profile {
            id,
            limits: Limits { fppt: 80_000, sppt: 65_000, spl: 25_000 },
            nv_boost: 0,
            fan: FanMode::Auto,
            overlay: OVERLAY_BALANCED,
            refresh: Refresh::Leave,
            brightness: None,
        },
        ProfileId::BalancedDc => Profile {
            id,
            limits: Limits { fppt: 54_000, sppt: 54_000, spl: 20_000 },
            nv_boost: 0,
            fan: FanMode::Auto,
            overlay: OVERLAY_BALANCED,
            refresh: Refresh::Leave,
            brightness: None,
        },
        ProfileId::Performance => Profile {
            id,
            limits: Limits { fppt: 80_000, sppt: 65_000, spl: 40_000 },
            nv_boost: 1,
            fan: FanMode::Max,
            overlay: OVERLAY_PERFORMANCE,
            refresh: Refresh::Max,
            brightness: None,
        },
    }
}

/// Writes the firmware half of a profile. Every lever is attempted even if an
/// earlier one fails, so a rejected power limit still leaves the fans right.
pub fn apply_firmware(wmi: &dyn GbWmi, p: &Profile) -> Result<(), HwError> {
    let l = clamp(p.limits);
    let mut first_err = None;
    for (method, value) in [
        ("SetApuParameter1", l.fppt),
        ("SetApuParameter2", l.sppt),
        ("SetApuParameter3", l.spl),
        ("SetNvPowerConfig", p.nv_boost),
    ] {
        if let Err(e) = wmi.set(method, value) {
            first_err.get_or_insert(e);
        }
    }
    if let Err(e) = fan::apply(wmi, p.fan) {
        first_err.get_or_insert(e);
    }
    first_err.map_or(Ok(()), Err)
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum UiMode {
    Battery,
    #[default]
    Auto,
    Performance,
}

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Reason {
    UserBattery,
    UserPerformance,
    AutoAc,
    AutoDc,
    AutoLowBattery,
    AutoGame,
}

#[derive(Clone, Copy, Debug)]
pub struct Inputs {
    pub now: Instant,
    pub on_ac: Option<bool>,
    pub battery_pct: Option<u8>,
    pub fullscreen: bool,
    pub gpu_util: Option<f32>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    pub auto_game: bool,
    pub acdc_switch: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Decision {
    pub profile: ProfileId,
    /// Whether refresh rate and brightness follow the profile too.
    pub displays: bool,
    pub reason: Reason,
}

// Docks and loose plugs chatter; only a power source that holds this long counts.
const ACDC_SETTLE: Duration = Duration::from_secs(4);
const LOW_BATTERY_PCT: u8 = 20;
const GAME_GPU_PCT: f32 = 40.0;
// Ticks without a game before Auto drops back, so a loading screen does not flip it.
const GAME_MISSES: u8 = 2;

pub struct Controller {
    mode: UiMode,
    on_ac: Option<bool>,
    pending_ac: Option<(bool, Instant)>,
    game_on: bool,
    game_misses: u8,
    last: Option<(ProfileId, bool)>,
}

impl Controller {
    pub fn new(mode: UiMode) -> Self {
        Controller { mode, on_ac: None, pending_ac: None, game_on: false, game_misses: 0, last: None }
    }

    pub fn mode(&self) -> UiMode {
        self.mode
    }

    /// An explicit choice always re-applies, even if the profile is unchanged.
    pub fn set_mode(&mut self, mode: UiMode) {
        self.mode = mode;
        self.last = None;
    }

    fn ac(&self) -> bool {
        self.on_ac.unwrap_or(true)
    }

    /// Reading GPU load wakes a sleeping dGPU, so it is only worth it when a
    /// fullscreen window might be a game Auto would boost for.
    pub fn wants_gpu_counter(&self, i: &Inputs, o: &Options) -> bool {
        self.mode == UiMode::Auto && o.auto_game && self.ac() && i.fullscreen
    }

    fn settle_power_source(&mut self, i: &Inputs, o: &Options) {
        let Some(ac) = i.on_ac else { return };
        match self.on_ac {
            None => self.on_ac = Some(ac),
            Some(current) if current == ac => self.pending_ac = None,
            Some(_) => match self.pending_ac {
                Some((pending, since)) if pending == ac => {
                    if i.now.duration_since(since) >= ACDC_SETTLE {
                        self.on_ac = Some(ac);
                        self.pending_ac = None;
                        if o.acdc_switch {
                            self.mode = if ac { UiMode::Auto } else { UiMode::Battery };
                        }
                    }
                }
                _ => self.pending_ac = Some((ac, i.now)),
            },
        }
    }

    fn track_game(&mut self, i: &Inputs, o: &Options) {
        if self.mode != UiMode::Auto || !o.auto_game || !self.ac() {
            self.game_on = false;
            self.game_misses = 0;
            return;
        }
        let gaming = i.fullscreen && i.gpu_util.is_some_and(|u| u >= GAME_GPU_PCT);
        if gaming {
            self.game_on = true;
            self.game_misses = 0;
        } else if self.game_on {
            self.game_misses += 1;
            if self.game_misses >= GAME_MISSES {
                self.game_on = false;
                self.game_misses = 0;
            }
        }
    }

    fn decide(&self, i: &Inputs) -> Decision {
        let d = |profile, displays, reason| Decision { profile, displays, reason };
        match self.mode {
            UiMode::Battery => d(ProfileId::Battery, true, Reason::UserBattery),
            UiMode::Performance => d(ProfileId::Performance, true, Reason::UserPerformance),
            UiMode::Auto if !self.ac() => {
                if i.battery_pct.is_some_and(|p| p <= LOW_BATTERY_PCT) {
                    d(ProfileId::Battery, true, Reason::AutoLowBattery)
                } else {
                    d(ProfileId::Battery, false, Reason::AutoDc)
                }
            }
            UiMode::Auto if self.game_on => d(ProfileId::Performance, true, Reason::AutoGame),
            UiMode::Auto => d(ProfileId::BalancedAc, false, Reason::AutoAc),
        }
    }

    /// Returns a decision only when what should be applied has changed.
    pub fn tick(&mut self, i: &Inputs, o: &Options) -> Option<Decision> {
        self.settle_power_source(i, o);
        self.track_game(i, o);
        let d = self.decide(i);
        if self.last == Some((d.profile, d.displays)) {
            return None;
        }
        self.last = Some((d.profile, d.displays));
        Some(d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hw::fake::FakeWmi;

    const ALL: [ProfileId; 4] = [ProfileId::Battery, ProfileId::BalancedAc, ProfileId::BalancedDc, ProfileId::Performance];

    mod controller {
        use super::super::*;
        use std::time::{Duration, Instant};

        fn inputs(now: Instant, ac: bool, pct: u8) -> Inputs {
            Inputs { now, on_ac: Some(ac), battery_pct: Some(pct), fullscreen: false, gpu_util: None }
        }
        const OFF: Options = Options { auto_game: false, acdc_switch: false };
        const GAME: Options = Options { auto_game: true, acdc_switch: false };

        #[test]
        fn auto_on_ac_is_balanced() {
            let mut c = Controller::new(UiMode::Auto);
            let d = c.tick(&inputs(Instant::now(), true, 80), &OFF).unwrap();
            assert_eq!((d.profile, d.displays, d.reason), (ProfileId::BalancedAc, false, Reason::AutoAc));
        }

        #[test]
        fn auto_on_battery_is_battery_without_displays() {
            let mut c = Controller::new(UiMode::Auto);
            let d = c.tick(&inputs(Instant::now(), false, 80), &OFF).unwrap();
            assert_eq!((d.profile, d.displays, d.reason), (ProfileId::Battery, false, Reason::AutoDc));
        }

        #[test]
        fn auto_low_battery_touches_displays() {
            let mut c = Controller::new(UiMode::Auto);
            let d = c.tick(&inputs(Instant::now(), false, 20), &OFF).unwrap();
            assert_eq!((d.profile, d.displays, d.reason), (ProfileId::Battery, true, Reason::AutoLowBattery));
        }

        #[test]
        fn user_modes_override_auto() {
            let now = Instant::now();
            let mut c = Controller::new(UiMode::Performance);
            assert_eq!(c.tick(&inputs(now, false, 50), &OFF).unwrap().profile, ProfileId::Performance);
            c.set_mode(UiMode::Battery);
            let d = c.tick(&inputs(now, true, 50), &OFF).unwrap();
            assert_eq!((d.profile, d.displays), (ProfileId::Battery, true));
        }

        #[test]
        fn game_detected_on_ac_boosts() {
            let mut c = Controller::new(UiMode::Auto);
            let now = Instant::now();
            c.tick(&inputs(now, true, 90), &GAME);
            let gaming = Inputs { fullscreen: true, gpu_util: Some(75.0), ..inputs(now, true, 90) };
            let d = c.tick(&gaming, &GAME).unwrap();
            assert_eq!((d.profile, d.reason), (ProfileId::Performance, Reason::AutoGame));
        }

        #[test]
        fn game_end_needs_two_misses() {
            let mut c = Controller::new(UiMode::Auto);
            let now = Instant::now();
            let gaming = Inputs { fullscreen: true, gpu_util: Some(75.0), ..inputs(now, true, 90) };
            c.tick(&gaming, &GAME);
            assert!(c.tick(&inputs(now, true, 90), &GAME).is_none());
            assert_eq!(c.tick(&inputs(now, true, 90), &GAME).unwrap().profile, ProfileId::BalancedAc);
        }

        #[test]
        fn acdc_change_is_debounced() {
            let mut c = Controller::new(UiMode::Auto);
            let t0 = Instant::now();
            c.tick(&inputs(t0, true, 90), &OFF);
            assert!(c.tick(&inputs(t0 + Duration::from_secs(1), false, 90), &OFF).is_none());
            assert!(c.tick(&inputs(t0 + Duration::from_secs(2), true, 90), &OFF).is_none());
            assert!(c.tick(&inputs(t0 + Duration::from_secs(3), false, 90), &OFF).is_none());
            let d = c.tick(&inputs(t0 + Duration::from_secs(8), false, 90), &OFF).unwrap();
            assert_eq!(d.profile, ProfileId::Battery);
        }

        #[test]
        fn acdc_switch_moves_mode() {
            let opts = Options { auto_game: false, acdc_switch: true };
            let mut c = Controller::new(UiMode::Auto);
            let t0 = Instant::now();
            c.tick(&inputs(t0, true, 90), &opts);
            c.tick(&inputs(t0 + Duration::from_secs(1), false, 90), &opts);
            c.tick(&inputs(t0 + Duration::from_secs(6), false, 90), &opts);
            assert_eq!(c.mode(), UiMode::Battery);
            c.tick(&inputs(t0 + Duration::from_secs(7), true, 90), &opts);
            c.tick(&inputs(t0 + Duration::from_secs(12), true, 90), &opts);
            assert_eq!(c.mode(), UiMode::Auto);
        }

        #[test]
        fn same_profile_is_not_reapplied() {
            let mut c = Controller::new(UiMode::Performance);
            let now = Instant::now();
            assert!(c.tick(&inputs(now, true, 90), &OFF).is_some());
            assert!(c.tick(&inputs(now, true, 90), &OFF).is_none());
            c.set_mode(UiMode::Performance);
            assert!(c.tick(&inputs(now, true, 90), &OFF).is_some(), "an explicit choice re-applies");
        }

        #[test]
        fn gpu_counter_only_read_when_fullscreen() {
            let mut c = Controller::new(UiMode::Auto);
            let now = Instant::now();
            c.tick(&inputs(now, true, 90), &GAME);
            assert!(!c.wants_gpu_counter(&inputs(now, true, 90), &GAME));
            let fs = Inputs { fullscreen: true, ..inputs(now, true, 90) };
            assert!(c.wants_gpu_counter(&fs, &GAME));
            assert!(!c.wants_gpu_counter(&fs, &OFF));
        }
    }

    #[test]
    fn limits_are_clamped_to_envelope() {
        assert_eq!(
            clamp(Limits { fppt: 100_000, sppt: 90_000, spl: 50_000 }),
            Limits { fppt: 80_000, sppt: 75_000, spl: 40_000 }
        );
        assert_eq!(clamp(Limits { fppt: 1000, sppt: 1000, spl: 1000 }), Limits { fppt: 15_000, sppt: 15_000, spl: 15_000 });
    }

    #[test]
    fn profiles_stay_inside_envelope() {
        for id in ALL {
            let l = profile(id).limits;
            assert_eq!(clamp(l), l, "{id:?}");
        }
    }

    #[test]
    fn performance_writes_limits_then_boost_then_max_fan() {
        let w = FakeWmi::default();
        apply_firmware(&w, &profile(ProfileId::Performance)).unwrap();
        let calls = w.calls();
        assert_eq!(
            &calls[..4],
            &[
                ("SetApuParameter1".to_string(), 80_000),
                ("SetApuParameter2".to_string(), 65_000),
                ("SetApuParameter3".to_string(), 40_000),
                ("SetNvPowerConfig".to_string(), 1),
            ]
        );
        assert_eq!(calls.last().unwrap(), &("SetFixedFanStatus".to_string(), 1));
    }

    #[test]
    fn apply_continues_after_apu_failure() {
        let w = FakeWmi::failing_on("SetApuParameter2");
        assert!(apply_firmware(&w, &profile(ProfileId::Battery)).is_err());
        let names: Vec<String> = w.calls().into_iter().map(|c| c.0).collect();
        assert!(names.contains(&"SetApuParameter3".to_string()));
        assert!(names.contains(&"SetNvPowerConfig".to_string()));
        assert_eq!(names.last().unwrap(), "SetAutoFanStatus");
    }
}
