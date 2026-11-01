use serde::Serialize;

use crate::fan::FanMode;
use crate::hw::{GbWmi, HwError};

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Telemetry {
    pub rpm1: u32,
    pub rpm2: u32,
    pub cpu_temp: u32,
    pub gpu_temp: u32,
    pub cpu_duty: u32,
    pub gpu_duty: u32,
    pub fan_mode: FanMode,
}

pub fn read(wmi: &dyn GbWmi) -> Result<Telemetry, HwError> {
    // Derived from the firmware, never from what we last wrote: GiMATE's
    // service or a reboot can change the fan state behind our back.
    let fixed = wmi.get("GetFixedFanStatus")?;
    let auto = wmi.get("GetAutoFanStatus")?;
    Ok(Telemetry {
        rpm1: wmi.get("getRpm1")?,
        rpm2: wmi.get("getRpm2")?,
        cpu_temp: wmi.get("getCpuTemp")?,
        gpu_temp: wmi.get("getGpuTemp1")?,
        cpu_duty: wmi.get("GetCPUFanDuty")?,
        gpu_duty: wmi.get("GetGPUFanDuty")?,
        fan_mode: if fixed == 1 && auto == 0 { FanMode::Max } else { FanMode::Auto },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fan::apply;
    use crate::hw::fake::FakeWmi;

    fn seeded() -> FakeWmi {
        let w = FakeWmi::default();
        for (m, v) in [
            ("getRpm1", 5820), ("getRpm2", 6040), ("getCpuTemp", 71), ("getGpuTemp1", 64),
            ("GetCPUFanDuty", 100), ("GetGPUFanDuty", 100),
            ("GetFixedFanStatus", 1), ("GetAutoFanStatus", 0),
        ] {
            w.put(m, v);
        }
        w
    }

    #[test]
    fn read_maps_getters() {
        assert_eq!(
            read(&seeded()).unwrap(),
            Telemetry {
                rpm1: 5820, rpm2: 6040, cpu_temp: 71, gpu_temp: 64,
                cpu_duty: 100, gpu_duty: 100, fan_mode: FanMode::Max,
            }
        );
    }

    #[test]
    fn fan_mode_follows_firmware_not_last_command() {
        let w = seeded();
        apply(&w, FanMode::Max).unwrap();
        w.put("GetFixedFanStatus", 0);
        w.put("GetAutoFanStatus", 1);
        assert_eq!(read(&w).unwrap().fan_mode, FanMode::Auto);
    }

    #[test]
    fn read_fails_if_any_getter_fails() {
        let w = FakeWmi::failing_on("getRpm2");
        assert!(read(&w).is_err());
    }
}
