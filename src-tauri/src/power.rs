//! Power modes. Profiles hold the firmware and OS settings for one mode; the
//! controller decides which profile applies from the user's choice and the
//! power source.

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hw::fake::FakeWmi;

    const ALL: [ProfileId; 4] = [ProfileId::Battery, ProfileId::BalancedAc, ProfileId::BalancedDc, ProfileId::Performance];

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
