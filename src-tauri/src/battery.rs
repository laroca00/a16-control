//! Battery watcher and EcoQoS. The watcher profiles ten minutes of real use
//! on battery; the processes that burned the most CPU are then throttled
//! with EcoQoS whenever the laptop runs on battery.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use serde::Serialize;

pub const WATCH_FOR: Duration = Duration::from_secs(600);
pub const SAMPLE_EVERY: Duration = Duration::from_secs(5);
pub const IDLE_AFTER: Duration = Duration::from_secs(60);
const TARGET_CPU_SECONDS: u64 = 15;
const HUNDRED_NS_PER_SEC: u64 = 10_000_000;
const NEVER: [&str; 7] = ["gienemy", "explorer", "dwm", "csrss", "winlogon", "system", "idle"];

#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub watts: Option<f32>,
    pub cpu: f32,
    pub idle: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub avg_watts: Option<f32>,
    pub avg_cpu: f32,
    pub idle_pct: f32,
    pub targets: Vec<String>,
    pub minutes: u64,
}

fn mean(values: impl Iterator<Item = f32>) -> Option<f32> {
    let (sum, n) = values.fold((0.0, 0u32), |(s, n), v| (s + v, n + 1));
    (n > 0).then(|| sum / n as f32)
}

/// `before`/`after` map pid to (image name, CPU time in 100 ns units).
pub fn summarize(
    samples: &[Sample],
    before: &HashMap<u32, (String, u64)>,
    after: &HashMap<u32, (String, u64)>,
    elapsed: Duration,
) -> Summary {
    let mut by_name: HashMap<String, u64> = HashMap::new();
    for (pid, (name, t)) in after {
        let start = before.get(pid).map_or(0, |(_, t0)| *t0);
        *by_name.entry(name.to_lowercase()).or_default() += t.saturating_sub(start);
    }
    let mut targets: Vec<(String, u64)> = by_name
        .into_iter()
        .filter(|(name, t)| *t >= TARGET_CPU_SECONDS * HUNDRED_NS_PER_SEC && !NEVER.contains(&name.as_str()))
        .collect();
    targets.sort_by(|a, b| b.1.cmp(&a.1));
    let idle = samples.iter().filter(|s| s.idle).count();
    Summary {
        avg_watts: mean(samples.iter().filter_map(|s| s.watts)),
        avg_cpu: mean(samples.iter().map(|s| s.cpu)).unwrap_or(0.0),
        idle_pct: if samples.is_empty() { 0.0 } else { idle as f32 * 100.0 / samples.len() as f32 },
        targets: targets.into_iter().map(|(n, _)| n).collect(),
        minutes: (elapsed.as_secs() + 30) / 60,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct EcoPlan {
    pub throttle: Vec<u32>,
    pub release: Vec<u32>,
}

/// `targets` are running processes whose names are on the list. The
/// foreground app always runs at full speed.
pub fn eco_plan(targets: &[(u32, String)], foreground: Option<u32>, throttled: &HashSet<u32>) -> EcoPlan {
    let mut plan = EcoPlan { throttle: Vec::new(), release: Vec::new() };
    for (pid, _) in targets {
        let is_fg = foreground == Some(*pid);
        match (is_fg, throttled.contains(pid)) {
            (true, true) => plan.release.push(*pid),
            (false, false) => plan.throttle.push(*pid),
            _ => {}
        }
    }
    plan
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BatteryView {
    pub watching: bool,
    pub progress: f32,
    pub summary: Option<Summary>,
    pub eco_enabled: bool,
    pub targets: Vec<String>,
}

pub fn view(progress: Option<f32>, summary: Option<Summary>, eco_enabled: bool, targets: &[String]) -> BatteryView {
    BatteryView { watching: progress.is_some(), progress: progress.unwrap_or(0.0), summary, eco_enabled, targets: targets.to_vec() }
}

#[derive(Debug, PartialEq)]
pub enum WatchEvent {
    Progress(f32),
    Cancelled,
    Done,
}

pub struct Watch {
    started: Instant,
}

impl Watch {
    /// Measuring on AC says nothing about battery use.
    pub fn start(on_ac: bool, now: Instant) -> Result<Watch, &'static str> {
        if on_ac {
            Err("unplug from power first")
        } else {
            Ok(Watch { started: now })
        }
    }

    pub fn started(&self) -> Instant {
        self.started
    }

    pub fn sample(&mut self, on_ac: bool, now: Instant) -> WatchEvent {
        if on_ac {
            return WatchEvent::Cancelled;
        }
        let elapsed = now.duration_since(self.started);
        if elapsed >= WATCH_FOR {
            WatchEvent::Done
        } else {
            WatchEvent::Progress(elapsed.as_secs_f32() / WATCH_FOR.as_secs_f32())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(items: &[(u32, &str, u64)]) -> HashMap<u32, (String, u64)> {
        items.iter().map(|(p, n, t)| (*p, (n.to_string(), *t))).collect()
    }
    const SEC: u64 = 10_000_000; // 100 ns units

    #[test]
    fn summary_averages_and_idle_share() {
        let samples = vec![
            Sample { watts: Some(10.0), cpu: 20.0, idle: false },
            Sample { watts: Some(14.0), cpu: 40.0, idle: true },
            Sample { watts: None, cpu: 30.0, idle: true },
            Sample { watts: Some(12.0), cpu: 10.0, idle: false },
        ];
        let s = summarize(&samples, &HashMap::new(), &HashMap::new(), Duration::from_secs(600));
        assert_eq!(s.avg_watts, Some(12.0));
        assert_eq!(s.avg_cpu, 25.0);
        assert_eq!(s.idle_pct, 50.0);
        assert_eq!(s.minutes, 10);
    }

    #[test]
    fn targets_need_fifteen_cpu_seconds_and_skip_shell() {
        let before = snap(&[(1, "chrome", 0), (2, "explorer", 0), (3, "spotify", 0), (4, "chrome", 5 * SEC)]);
        let after = snap(&[(1, "chrome", 10 * SEC), (2, "explorer", 90 * SEC), (3, "spotify", 3 * SEC), (4, "chrome", 12 * SEC)]);
        let s = summarize(&[], &before, &after, Duration::from_secs(600));
        assert_eq!(s.targets, vec!["chrome".to_string()]);
    }

    #[test]
    fn foreground_is_released() {
        let targets = vec![(10, "chrome".to_string()), (11, "chrome".to_string())];
        let throttled: HashSet<u32> = [10, 11].into();
        let plan = eco_plan(&targets, Some(10), &throttled);
        assert_eq!(plan.release, vec![10]);
        assert!(plan.throttle.is_empty());
    }

    #[test]
    fn new_target_pids_are_throttled_once() {
        let targets = vec![(10, "chrome".to_string()), (12, "slack".to_string())];
        let throttled: HashSet<u32> = [10, 99].into();
        let plan = eco_plan(&targets, None, &throttled);
        assert_eq!(plan.throttle, vec![12]);
        // 99 exited; nothing to release for a gone process.
        assert!(plan.release.is_empty());
    }

    #[test]
    fn battery_view_reports_progress() {
        let v = view(Some(0.25), None, true, &["chrome".to_string()]);
        assert!(v.watching);
        assert_eq!(v.progress, 0.25);
        assert!(v.eco_enabled);
        assert_eq!(v.targets, vec!["chrome".to_string()]);
        assert!(!view(None, None, false, &[]).watching);
    }

    #[test]
    fn watch_refuses_to_start_on_ac() {
        assert!(Watch::start(true, Instant::now()).is_err());
        assert!(Watch::start(false, Instant::now()).is_ok());
    }

    #[test]
    fn watch_cancels_when_plugged_in() {
        let t0 = Instant::now();
        let mut w = Watch::start(false, t0).unwrap();
        assert!(matches!(w.sample(false, t0 + Duration::from_secs(5)), WatchEvent::Progress(_)));
        assert!(matches!(w.sample(true, t0 + Duration::from_secs(10)), WatchEvent::Cancelled));
    }

    #[test]
    fn watch_finishes_after_ten_minutes() {
        let t0 = Instant::now();
        let mut w = Watch::start(false, t0).unwrap();
        match w.sample(false, t0 + Duration::from_secs(300)) {
            WatchEvent::Progress(p) => assert!((p - 0.5).abs() < 0.01),
            _ => panic!("expected progress"),
        }
        assert!(matches!(w.sample(false, t0 + Duration::from_secs(601)), WatchEvent::Done));
    }
}
