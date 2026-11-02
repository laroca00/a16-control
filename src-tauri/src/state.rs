use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::fan::{self, FanMode};
use crate::hw::{GbWmi, HwError};
use crate::power::{self, Controller, Decision, PowerView};
use crate::telemetry::{self, Telemetry};

pub type Connector = Box<dyn Fn() -> Result<Arc<dyn GbWmi>, HwError> + Send + Sync>;

/// Error text the frontend recognises as "run setup first".
pub const SETUP_NEEDED: &str = "setup";

// WMI or the ACPI provider can still be starting when the logon task fires.
const RETRY_EVERY: Duration = Duration::from_secs(5);

pub struct AppState {
    /// None when setup has not run; then there is nothing to connect to.
    connector: Option<Connector>,
    wmi: Mutex<Option<Arc<dyn GbWmi>>>,
    last_attempt: Mutex<Option<(Instant, String)>>,
    retry_every: Duration,
    pub fan_lock: Mutex<()>,
    pub config: Mutex<Config>,
    pub power: Mutex<PowerState>,
    /// Fan mode the user picked by hand; automatic power decisions keep it.
    pub manual_fan: Mutex<Option<FanMode>>,
    pub fan_gov: Mutex<fan::Governor>,
}

pub struct PowerState {
    pub controller: Controller,
    pub last: Option<Decision>,
    pub error: Option<String>,
    /// When the last apply failed; retries wait a little so a broken lever
    /// does not respawn powercfg every tick.
    pub failed_at: Option<std::time::Instant>,
}

impl AppState {
    pub fn new(connector: Option<Connector>, config: Config) -> Self {
        Self::with_retry(connector, config, RETRY_EVERY)
    }

    fn with_retry(connector: Option<Connector>, config: Config, retry_every: Duration) -> Self {
        AppState {
            connector,
            wmi: Mutex::new(None),
            last_attempt: Mutex::new(None),
            retry_every,
            fan_lock: Mutex::new(()),
            manual_fan: Mutex::new(config.fan_mode),
            fan_gov: Mutex::new(fan::Governor::default()),
            power: Mutex::new(PowerState { controller: Controller::new(config.power_mode), last: None, error: None, failed_at: None }),
            config: Mutex::new(config),
        }
    }

    /// The firmware connection, made on first use and retried after a failure.
    pub fn wmi(&self) -> Result<Arc<dyn GbWmi>, String> {
        let mut wmi = self.wmi.lock().unwrap();
        if let Some(w) = wmi.as_ref() {
            return Ok(w.clone());
        }
        let connect = self.connector.as_ref().ok_or(SETUP_NEEDED)?;
        let mut last = self.last_attempt.lock().unwrap();
        if let Some((at, err)) = last.as_ref() {
            if at.elapsed() < self.retry_every {
                return Err(err.clone());
            }
        }
        match connect() {
            Ok(w) => {
                *wmi = Some(w.clone());
                *last = None;
                Ok(w)
            }
            Err(e) => {
                let msg = e.to_string();
                *last = Some((Instant::now(), msg.clone()));
                Err(msg)
            }
        }
    }

    pub fn telemetry(&self) -> Result<Telemetry, String> {
        let wmi = self.wmi()?;
        let mut t = telemetry::read(wmi.as_ref()).map_err(|e| e.to_string())?;
        // Quiet handed to the firmware while hot is still the mode the user picked.
        if t.fan_mode == FanMode::Auto && self.fan_gov.lock().unwrap().active() {
            t.fan_mode = FanMode::Quiet;
        }
        Ok(t)
    }

    fn remember_fan(&self, mode: Option<FanMode>) {
        *self.manual_fan.lock().unwrap() = mode;
        let mut cfg = self.config.lock().unwrap();
        if cfg.fan_mode != mode {
            cfg.fan_mode = mode;
            let _ = crate::config::save(&cfg);
        }
    }
}

impl AppState {
    /// An explicit power mode choice; it also ends any manual fan override.
    pub fn choose_mode(&self, mode: power::UiMode) {
        self.remember_fan(None);
        let mut p = self.power.lock().unwrap();
        p.failed_at = None;
        p.controller.set_mode(mode);
    }

    pub fn power_view(&self) -> PowerView {
        let options = self.config.lock().unwrap().power_options();
        let p = self.power.lock().unwrap();
        power::view(p.controller.mode(), p.last.as_ref(), options, p.error.as_deref())
    }
}

pub fn set_fan(state: &AppState, mode: FanMode) -> Result<Telemetry, String> {
    let wmi = state.wmi()?;
    {
        let _guard = state.fan_lock.lock().unwrap();
        state.remember_fan(Some(mode));
        let result = fan::apply(wmi.as_ref(), mode);
        let mut gov = state.fan_gov.lock().unwrap();
        gov.reset(mode);
        if result.is_err() {
            gov.failed();
        }
        result.map_err(|e| e.to_string())?;
    }
    state.telemetry()
}

/// One governor step: re-reads the temperature and moves Quiet between its
/// lock and the firmware when it has to. Runs on every background tick.
pub fn fan_tick(state: &AppState) {
    if !state.fan_gov.lock().unwrap().active() {
        return;
    }
    let Ok(wmi) = state.wmi() else { return };
    let _guard = state.fan_lock.lock().unwrap();
    let mut gov = state.fan_gov.lock().unwrap();
    match fan::read_temp(wmi.as_ref()) {
        Ok(temp) => {
            if let Some(mode) = gov.tick(temp) {
                if fan::apply(wmi.as_ref(), mode).is_err() {
                    gov.failed();
                }
            }
        }
        // Blind, the quiet lock is the one unsafe state: hand the fans to
        // the firmware until the temperature can be read again.
        Err(_) => {
            if gov.locked() {
                if fan::apply(wmi.as_ref(), FanMode::Auto).is_ok() {
                    gov.released();
                } else {
                    gov.failed();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hw::fake::FakeWmi;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn set_fan_without_wmi_returns_setup_error() {
        let state = AppState::new(None, Config::default());
        assert_eq!(set_fan(&state, FanMode::Max).unwrap_err(), "setup");
    }

    #[test]
    fn wmi_retries_after_failure() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let counter = attempts.clone();
        let connector: Connector = Box::new(move || {
            if counter.fetch_add(1, Ordering::SeqCst) == 0 {
                Err(HwError::Unavailable("not ready".into()))
            } else {
                Ok(Arc::new(FakeWmi::default()) as Arc<dyn GbWmi>)
            }
        });
        let state = AppState::with_retry(Some(connector), Config::default(), Duration::ZERO);
        assert!(state.wmi().err().unwrap().contains("not ready"));
        assert!(state.wmi().is_ok());
        assert!(state.wmi().is_ok());
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn wmi_does_not_hammer_a_failing_connection() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let counter = attempts.clone();
        let connector: Connector = Box::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
            Err(HwError::Unavailable("no instance".into()))
        });
        let state = AppState::with_retry(Some(connector), Config::default(), Duration::from_secs(60));
        assert!(state.wmi().is_err());
        assert!(state.wmi().is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }
}
