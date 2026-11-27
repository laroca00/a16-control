//! Watches temperatures in the background: warns when the chip stays hot and,
//! if asked, turns Max Fan on. Also sends the short notices hotkeys answer with.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::commands::lang;
use crate::fan::FanMode;
use crate::state::{self, AppState};
use crate::telemetry::Telemetry;

/// How long the temperature must stay at or over the limit before a warning.
const SUSTAIN: Duration = Duration::from_secs(20);
/// After a warning, the next one waits until the chip has cooled this far below the limit...
const REARM_BELOW: u32 = 5;
/// ...and at least this long, so a game hovering at the limit does not nag.
const MIN_GAP: Duration = Duration::from_secs(10 * 60);

pub const ALERT_TEMPS: std::ops::RangeInclusive<u32> = 70..=100;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AlertOptions {
    pub enabled: bool,
    /// °C, for the hotter of CPU and GPU.
    pub temp: u32,
    /// Turn Max Fan on along with the warning.
    pub max_fan: bool,
}

impl Default for AlertOptions {
    fn default() -> Self {
        AlertOptions { enabled: true, temp: 90, max_fan: false }
    }
}

/// Decides when a sustained high temperature should warn.
#[derive(Debug)]
pub struct TempAlert {
    above_since: Option<Instant>,
    armed: bool,
    last_fired: Option<Instant>,
}

impl Default for TempAlert {
    fn default() -> Self {
        TempAlert { above_since: None, armed: true, last_fired: None }
    }
}

impl TempAlert {
    /// Feeds one reading; true when this reading should raise the warning.
    pub fn tick(&mut self, now: Instant, temp: u32, limit: u32) -> bool {
        if !self.armed {
            if temp + REARM_BELOW <= limit {
                self.armed = true;
            }
            return false;
        }
        if temp < limit {
            self.above_since = None;
            return false;
        }
        let since = *self.above_since.get_or_insert(now);
        let gap_ok = self.last_fired.map_or(true, |t| now.duration_since(t) >= MIN_GAP);
        if now.duration_since(since) >= SUSTAIN && gap_ok {
            self.armed = false;
            self.above_since = None;
            self.last_fired = Some(now);
            return true;
        }
        false
    }

    /// Forget a hot spell in progress, for when the warning is switched off.
    pub fn reset(&mut self) {
        self.above_since = None;
        self.armed = true;
    }
}

#[derive(Default)]
pub struct Monitor {
    pub alert: Mutex<TempAlert>,
}

/// Shows a Windows notification; a failure only loses the notice.
pub fn notify(app: &AppHandle, title: &str, body: &str) {
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        eprintln!("notification failed: {e}");
    }
}

fn tr(app: &AppHandle) -> bool {
    lang(app) == "tr"
}

/// Runs on every telemetry reading the background loop makes.
pub fn check_temp(app: &AppHandle, t: &Telemetry) {
    let options = app.state::<AppState>().config.lock().unwrap().alerts;
    let monitor = app.state::<Monitor>();
    if !options.enabled {
        monitor.alert.lock().unwrap().reset();
        return;
    }
    let hottest = t.cpu_temp.max(t.gpu_temp);
    if !monitor.alert.lock().unwrap().tick(Instant::now(), hottest, options.temp) {
        return;
    }
    let source = if t.cpu_temp >= t.gpu_temp { "CPU" } else { "GPU" };
    let turn_on = options.max_fan && t.fan_mode != FanMode::Max;
    if turn_on {
        let st = app.state::<AppState>();
        let now = state::set_fan(&st, FanMode::Max).ok();
        crate::tray::sync_fan_check(app, now.as_ref());
    }
    let (title, body) = match (tr(app), turn_on) {
        (true, false) => (format!("{source} {hottest}°C"), format!("{} saniyedir {}°C üstünde.", SUSTAIN.as_secs(), options.temp)),
        (true, true) => (format!("{source} {hottest}°C"), "Fanlar soğutmak için tam devire alındı.".to_string()),
        (false, false) => (format!("{source} at {hottest}°C"), format!("Above {}°C for {} seconds.", options.temp, SUSTAIN.as_secs())),
        (false, true) => (format!("{source} at {hottest}°C"), "Max Fan is on to cool it down.".to_string()),
    };
    notify(app, &title, &body);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(start: Instant, s: u64) -> Instant {
        start + Duration::from_secs(s)
    }

    #[test]
    fn warns_only_after_the_heat_lasts() {
        let start = Instant::now();
        let mut a = TempAlert::default();
        assert!(!a.tick(start, 92, 90));
        assert!(!a.tick(secs(start, 19), 93, 90));
        assert!(a.tick(secs(start, 20), 93, 90));
    }

    #[test]
    fn a_dip_below_the_limit_restarts_the_wait() {
        let start = Instant::now();
        let mut a = TempAlert::default();
        a.tick(start, 92, 90);
        a.tick(secs(start, 15), 88, 90);
        assert!(!a.tick(secs(start, 25), 92, 90));
        assert!(a.tick(secs(start, 45), 92, 90));
    }

    #[test]
    fn stays_quiet_until_the_chip_cools() {
        let start = Instant::now();
        let mut a = TempAlert::default();
        a.tick(start, 95, 90);
        assert!(a.tick(secs(start, 20), 95, 90));
        // Still hot long after: no second warning.
        assert!(!a.tick(secs(start, 2000), 95, 90));
        // Cooling only to the limit is not enough to re-arm.
        a.tick(secs(start, 2010), 88, 90);
        assert!(!a.tick(secs(start, 2020), 95, 90));
        assert!(!a.tick(secs(start, 2050), 95, 90));
    }

    #[test]
    fn warns_again_after_cooling_and_the_gap() {
        let start = Instant::now();
        let mut a = TempAlert::default();
        a.tick(start, 95, 90);
        assert!(a.tick(secs(start, 20), 95, 90));
        a.tick(secs(start, 60), 84, 90);
        // Re-armed, but within ten minutes of the last warning.
        a.tick(secs(start, 100), 95, 90);
        assert!(!a.tick(secs(start, 200), 95, 90));
        // The hot spell continues past the gap.
        assert!(a.tick(secs(start, 620), 95, 90));
    }

    #[test]
    fn reset_drops_a_hot_spell() {
        let start = Instant::now();
        let mut a = TempAlert::default();
        a.tick(start, 95, 90);
        a.reset();
        assert!(!a.tick(secs(start, 20), 95, 90));
        assert!(a.tick(secs(start, 40), 95, 90));
    }
}
