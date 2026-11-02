use serde::{Deserialize, Serialize};

use crate::hw::{GbWmi, HwError};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum FanMode {
    Max,
    Auto,
    /// The fixed lock without step mode, about 2400-2700 RPM; `Governor`
    /// hands it to the firmware when the chip gets hot.
    Quiet,
}

// The governor must be off before the fixed duty is written, and the fixed
// lock goes last; with the governor still on, this firmware ignores the duty
// and the fans slow down instead.
pub const MAX_SEQUENCE: [(&str, u32); 6] = [
    ("SetCurrentFanStep", 0),
    ("SetAutoFanStatus", 0),
    ("SetFixedFanSpeed", 100),
    ("SetGPUFanDuty", 100),
    ("SetStepFanStatus", 1),
    ("SetFixedFanStatus", 1),
];

// Measured on the A16 (2026-11-02): with step mode off the fixed lock settles
// near 2400/2700 RPM whatever duty is written (30-137 all read the same), and
// with step mode on it runs flat out. The duty is still written as 100 so a
// BIOS that does honour it errs loud, not hot.
pub const QUIET_SEQUENCE: [(&str, u32); 6] = [
    ("SetCurrentFanStep", 0),
    ("SetStepFanStatus", 0),
    ("SetAutoFanStatus", 0),
    ("SetFixedFanSpeed", 100),
    ("SetGPUFanDuty", 100),
    ("SetFixedFanStatus", 1),
];

// On the A16 the stock state is AutoFanStatus=1; writing 0 here would leave
// the fans without a governor.
pub const AUTO_SEQUENCE: [(&str, u32); 4] = [
    ("SetFixedFanStatus", 0),
    ("SetStepFanStatus", 0),
    ("SetCurrentFanStep", 0),
    ("SetAutoFanStatus", 1),
];

/// Quiet hands the fans to the firmware at this temperature (°C)...
pub const QUIET_UNTIL: u32 = 80;
/// ...and takes them back once the chip has cooled to this.
pub const QUIET_AGAIN: u32 = 72;

pub fn apply(wmi: &dyn GbWmi, mode: FanMode) -> Result<(), HwError> {
    let sequence: &[(&str, u32)] = match mode {
        FanMode::Max => &MAX_SEQUENCE,
        FanMode::Quiet => &QUIET_SEQUENCE,
        FanMode::Auto => {
            // Every step is attempted so the governor always gets switched back on.
            let mut first_err = None;
            for (method, value) in AUTO_SEQUENCE {
                if let Err(e) = wmi.set(method, value) {
                    first_err.get_or_insert(e);
                }
            }
            return first_err.map_or(Ok(()), Err);
        }
    };
    for (method, value) in sequence {
        if let Err(e) = wmi.set(method, *value) {
            // A half-applied lock can leave the governor off with no fixed
            // duty, which slows the fans; hand them back.
            let _ = apply(wmi, FanMode::Auto);
            return Err(e);
        }
    }
    Ok(())
}

/// Hotter of the CPU and GPU, the one the fans have to answer to.
pub fn read_temp(wmi: &dyn GbWmi) -> Result<u32, HwError> {
    Ok(wmi.get("getCpuTemp")?.max(wmi.get("getGpuTemp1")?))
}

/// Keeps Quiet safe between applies: the lock does not follow temperature,
/// so above QUIET_UNTIL the firmware's own curve takes over until it cools.
#[derive(Default, Debug)]
pub struct Governor {
    mode: Option<FanMode>,
    /// What is in the firmware now; None when unknown or a write failed.
    written: Option<FanMode>,
}

impl Governor {
    /// Called after a mode was written.
    pub fn reset(&mut self, mode: FanMode) {
        self.mode = Some(mode);
        self.written = Some(mode);
    }

    /// Whether ticks need the temperature at all.
    pub fn active(&self) -> bool {
        self.mode == Some(FanMode::Quiet)
    }

    /// The firmware holds a lock we may have to lift.
    pub fn locked(&self) -> bool {
        self.written == Some(FanMode::Quiet)
    }

    /// The mode to write for this temperature, or None to leave the fans be.
    pub fn tick(&mut self, temp: u32) -> Option<FanMode> {
        if self.mode != Some(FanMode::Quiet) {
            return None;
        }
        let want = match self.written {
            Some(FanMode::Auto) if temp > QUIET_AGAIN => FanMode::Auto,
            _ if temp >= QUIET_UNTIL => FanMode::Auto,
            _ => FanMode::Quiet,
        };
        if self.written == Some(want) {
            return None;
        }
        self.written = Some(want);
        Some(want)
    }

    /// The last write did not land; the next tick tries again.
    pub fn failed(&mut self) {
        self.written = None;
    }

    /// The fans were handed to the firmware outside a tick.
    pub fn released(&mut self) {
        self.written = Some(FanMode::Auto);
    }
}

/// Hands the fans back to the firmware unless they are already in auto. The
/// fixed lock outlives our process, so this runs before quitting and
/// uninstalling; when the state cannot be read, Auto is applied anyway
/// because it is always safe.
pub fn release(wmi: &dyn GbWmi) -> Result<(), HwError> {
    match crate::telemetry::read(wmi) {
        Ok(t) if t.fan_mode == FanMode::Auto => Ok(()),
        _ => apply(wmi, FanMode::Auto),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hw::fake::FakeWmi;

    #[test]
    fn max_sequence_is_exact() {
        let w = FakeWmi::default();
        apply(&w, FanMode::Max).unwrap();
        assert_eq!(
            w.calls(),
            vec![
                ("SetCurrentFanStep".into(), 0),
                ("SetAutoFanStatus".into(), 0),
                ("SetFixedFanSpeed".into(), 100),
                ("SetGPUFanDuty".into(), 100),
                ("SetStepFanStatus".into(), 1),
                ("SetFixedFanStatus".into(), 1),
            ]
        );
    }

    #[test]
    fn quiet_locks_without_step_mode() {
        let w = FakeWmi::default();
        apply(&w, FanMode::Quiet).unwrap();
        let calls = w.calls();
        assert_eq!(calls[1], ("SetStepFanStatus".into(), 0));
        assert!(!calls.contains(&("SetStepFanStatus".into(), 1)));
        assert_eq!(calls.last().unwrap(), &("SetFixedFanStatus".to_string(), 1));
    }

    #[test]
    fn auto_sequence_hands_back_to_governor() {
        let w = FakeWmi::default();
        apply(&w, FanMode::Auto).unwrap();
        assert_eq!(
            w.calls(),
            vec![
                ("SetFixedFanStatus".into(), 0),
                ("SetStepFanStatus".into(), 0),
                ("SetCurrentFanStep".into(), 0),
                ("SetAutoFanStatus".into(), 1),
            ]
        );
    }

    #[test]
    fn release_applies_auto_when_max() {
        let w = FakeWmi::default();
        w.put("GetFixedFanStatus", 1);
        w.put("GetAutoFanStatus", 0);
        release(&w).unwrap();
        assert_eq!(w.calls().last().unwrap(), &("SetAutoFanStatus".to_string(), 1));
    }

    #[test]
    fn release_applies_auto_when_state_unreadable() {
        let w = FakeWmi::failing_on("getRpm1");
        release(&w).unwrap();
        assert_eq!(w.calls().last().unwrap(), &("SetAutoFanStatus".to_string(), 1));
    }

    #[test]
    fn release_leaves_auto_alone() {
        let w = FakeWmi::default();
        w.put("GetAutoFanStatus", 1);
        release(&w).unwrap();
        assert!(w.calls().is_empty());
    }

    #[test]
    fn failed_max_falls_back_to_auto() {
        let w = FakeWmi::failing_on("SetFixedFanSpeed");
        assert!(apply(&w, FanMode::Max).is_err());
        // The governor was already off; it must not be left that way.
        assert_eq!(w.calls().last().unwrap(), &("SetAutoFanStatus".to_string(), 1));
    }

    #[test]
    fn failed_quiet_falls_back_to_auto() {
        let w = FakeWmi::failing_on("SetFixedFanStatus");
        assert!(apply(&w, FanMode::Quiet).is_err());
        assert_eq!(w.calls().last().unwrap(), &("SetAutoFanStatus".to_string(), 1));
    }

    #[test]
    fn governor_leaves_auto_and_max_alone() {
        let mut g = Governor::default();
        assert_eq!(g.tick(95), None);
        g.reset(FanMode::Max);
        assert!(!g.active());
        assert_eq!(g.tick(95), None);
    }

    #[test]
    fn governor_hands_hot_quiet_to_firmware_and_takes_it_back_cool() {
        let mut g = Governor::default();
        g.reset(FanMode::Quiet);
        assert_eq!(g.tick(70), None);
        assert_eq!(g.tick(QUIET_UNTIL), Some(FanMode::Auto));
        assert!(!g.locked());
        // Between the two thresholds it stays with the firmware.
        assert_eq!(g.tick(QUIET_AGAIN + 1), None);
        assert_eq!(g.tick(QUIET_AGAIN), Some(FanMode::Quiet));
        assert!(g.locked());
    }

    #[test]
    fn governor_retries_after_a_failed_write() {
        let mut g = Governor::default();
        g.reset(FanMode::Quiet);
        g.failed();
        assert_eq!(g.tick(50), Some(FanMode::Quiet));
    }

    #[test]
    fn auto_reaches_governor_even_if_a_step_fails() {
        let w = FakeWmi::failing_on("SetStepFanStatus");
        assert!(apply(&w, FanMode::Auto).is_err());
        assert_eq!(w.calls().last().unwrap(), &("SetAutoFanStatus".to_string(), 1));
        assert_eq!(w.calls().len(), 3);
    }
}
