use std::sync::{Arc, Mutex};

use crate::config::Config;
use crate::fan::{self, FanMode};
use crate::hw::{GbWmi, HwError};
use crate::telemetry::{self, Telemetry};

pub struct AppState {
    pub wmi: Option<Arc<dyn GbWmi>>,
    pub fan_lock: Mutex<()>,
    pub config: Mutex<Config>,
}

/// Error text the frontend recognises as "run setup first".
pub const SETUP_NEEDED: &str = "setup";

impl AppState {
    pub fn telemetry(&self) -> Result<Telemetry, String> {
        let wmi = self.wmi.as_ref().ok_or(SETUP_NEEDED)?;
        telemetry::read(wmi.as_ref()).map_err(|e| e.to_string())
    }
}

pub fn set_fan(state: &AppState, mode: FanMode) -> Result<Telemetry, String> {
    let wmi = state.wmi.as_ref().ok_or(SETUP_NEEDED)?;
    let _guard = state.fan_lock.lock().unwrap();
    fan::apply(wmi.as_ref(), mode).map_err(|e| e.to_string())?;
    telemetry::read(wmi.as_ref()).map_err(|e| e.to_string())
}

pub fn exit_plan(t: Option<&Telemetry>) -> bool {
    matches!(t, Some(t) if t.fan_mode == FanMode::Max)
}

/// The firmware keeps the fixed lock after we exit, so never leave the fans
/// pinned at full speed without the app that could release them.
pub fn restore_before_exit(wmi: Option<&dyn GbWmi>, t: Option<&Telemetry>) -> Result<(), HwError> {
    match wmi {
        Some(w) if exit_plan(t) => fan::apply(w, FanMode::Auto),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hw::fake::FakeWmi;

    fn telemetry(mode: FanMode) -> Telemetry {
        Telemetry { rpm1: 0, rpm2: 0, cpu_temp: 0, gpu_temp: 0, cpu_duty: 0, gpu_duty: 0, fan_mode: mode }
    }

    #[test]
    fn set_fan_without_wmi_returns_setup_error() {
        let state = AppState { wmi: None, fan_lock: Mutex::new(()), config: Mutex::new(Config::default()) };
        assert_eq!(set_fan(&state, FanMode::Max), Err("setup".to_string()));
    }

    #[test]
    fn exit_restores_fan_when_max() {
        assert!(exit_plan(Some(&telemetry(FanMode::Max))));
        assert!(!exit_plan(Some(&telemetry(FanMode::Auto))));
        assert!(!exit_plan(None));
    }

    #[test]
    fn exit_proceeds_when_restore_fails() {
        let w = FakeWmi::failing_on("SetFixedFanStatus");
        assert!(restore_before_exit(Some(&w), Some(&telemetry(FanMode::Max))).is_err());
        let ok = FakeWmi::default();
        assert!(restore_before_exit(Some(&ok), Some(&telemetry(FanMode::Max))).is_ok());
        assert_eq!(ok.calls().last().unwrap(), &("SetAutoFanStatus".to_string(), 1));
    }
}
