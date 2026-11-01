use serde::{Deserialize, Serialize};

use crate::hw::{GbWmi, HwError};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum FanMode {
    Max,
    Auto,
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

// On the A16 the stock state is AutoFanStatus=1; writing 0 here would leave
// the fans without a governor.
pub const AUTO_SEQUENCE: [(&str, u32); 4] = [
    ("SetFixedFanStatus", 0),
    ("SetStepFanStatus", 0),
    ("SetCurrentFanStep", 0),
    ("SetAutoFanStatus", 1),
];

pub fn apply(wmi: &dyn GbWmi, mode: FanMode) -> Result<(), HwError> {
    match mode {
        FanMode::Max => {
            for (method, value) in MAX_SEQUENCE {
                if let Err(e) = wmi.set(method, value) {
                    // A half-applied Max can leave the governor off with no
                    // fixed duty, which slows the fans; hand them back.
                    let _ = apply(wmi, FanMode::Auto);
                    return Err(e);
                }
            }
            Ok(())
        }
        FanMode::Auto => {
            // Every step is attempted so the governor always gets switched back on.
            let mut first_err = None;
            for (method, value) in AUTO_SEQUENCE {
                if let Err(e) = wmi.set(method, value) {
                    first_err.get_or_insert(e);
                }
            }
            first_err.map_or(Ok(()), Err)
        }
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
    fn failed_max_falls_back_to_auto() {
        let w = FakeWmi::failing_on("SetFixedFanSpeed");
        assert!(apply(&w, FanMode::Max).is_err());
        // The governor was already off; it must not be left that way.
        assert_eq!(w.calls().last().unwrap(), &("SetAutoFanStatus".to_string(), 1));
    }

    #[test]
    fn auto_reaches_governor_even_if_a_step_fails() {
        let w = FakeWmi::failing_on("SetStepFanStatus");
        assert!(apply(&w, FanMode::Auto).is_err());
        assert_eq!(w.calls().last().unwrap(), &("SetAutoFanStatus".to_string(), 1));
        assert_eq!(w.calls().len(), 3);
    }
}
