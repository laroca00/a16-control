//! App side of the battery watcher and EcoQoS.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};

use crate::battery::{self, BatteryView, Sample, Summary, Watch, WatchEvent};
use crate::config;
use crate::os;
use crate::state::AppState;

const ECO_EVERY: Duration = Duration::from_secs(60);

struct Run {
    watch: Watch,
    samples: Vec<Sample>,
    before: HashMap<u32, (String, u64)>,
    prev_times: Option<(u64, u64, u64)>,
    last_sample: Instant,
    progress: f32,
}

#[derive(Default)]
pub struct BatteryCtl {
    run: Mutex<Option<Run>>,
    summary: Mutex<Option<Summary>>,
    throttled: Mutex<HashSet<u32>>,
    last_eco: Mutex<Option<Instant>>,
}

pub fn view(app: &AppHandle) -> BatteryView {
    let ctl = app.state::<BatteryCtl>();
    let progress = ctl.run.lock().unwrap().as_ref().map(|r| r.progress);
    let (eco, targets) = {
        let st = app.state::<AppState>();
        let cfg = st.config.lock().unwrap();
        (cfg.eco_enabled, cfg.eco_targets.clone())
    };
    let summary = ctl.summary.lock().unwrap().clone();
    battery::view(progress, summary, eco, &targets)
}

fn emit(app: &AppHandle) {
    let _ = app.emit("battery", view(app));
}

pub fn start(app: &AppHandle) -> Result<BatteryView, String> {
    let on_ac = os::power_status().0 != Some(false);
    let now = Instant::now();
    let watch = Watch::start(on_ac, now).map_err(str::to_string)?;
    *app.state::<BatteryCtl>().run.lock().unwrap() = Some(Run {
        watch,
        samples: Vec::new(),
        before: os::process_cpu_snapshot(),
        prev_times: os::system_times(),
        last_sample: now,
        progress: 0.0,
    });
    let v = view(app);
    emit(app);
    Ok(v)
}

pub fn cancel(app: &AppHandle) -> BatteryView {
    app.state::<BatteryCtl>().run.lock().unwrap().take();
    emit(app);
    view(app)
}

fn finish(app: &AppHandle, run: Run, now: Instant) {
    let summary = battery::summarize(&run.samples, &run.before, &os::process_cpu_snapshot(), now.duration_since(run.watch.started()));
    {
        let st = app.state::<AppState>();
        let mut cfg = st.config.lock().unwrap();
        cfg.eco_targets = summary.targets.clone();
        let _ = config::save(&cfg);
    }
    *app.state::<BatteryCtl>().summary.lock().unwrap() = Some(summary);
}

/// Called on every background tick.
pub fn tick(app: &AppHandle) {
    let ctl = app.state::<BatteryCtl>();
    let now = Instant::now();
    let on_battery = os::power_status().0 == Some(false);

    let mut changed = false;
    let finished = {
        let mut guard = ctl.run.lock().unwrap();
        let mut done = None;
        if let Some(run) = guard.as_mut() {
            if now.duration_since(run.last_sample) >= battery::SAMPLE_EVERY {
                run.last_sample = now;
                changed = true;
                match run.watch.sample(!on_battery, now) {
                    WatchEvent::Progress(p) => {
                        run.progress = p;
                        let times = os::system_times();
                        let cpu = match (run.prev_times, times) {
                            (Some(a), Some(b)) => os::cpu_percent(a, b),
                            _ => 0.0,
                        };
                        run.prev_times = times;
                        run.samples.push(Sample { watts: os::battery_watts(), cpu, idle: os::user_idle() >= battery::IDLE_AFTER });
                    }
                    WatchEvent::Cancelled => {
                        guard.take();
                    }
                    WatchEvent::Done => done = guard.take(),
                }
            }
        }
        done
    };
    if let Some(run) = finished {
        finish(app, run, now);
    }
    if changed {
        emit(app);
    }

    let due = ctl.last_eco.lock().unwrap().is_none_or(|t| now.duration_since(t) >= ECO_EVERY);
    if due {
        *ctl.last_eco.lock().unwrap() = Some(now);
        eco_tick(app, on_battery);
    }
}

fn eco_tick(app: &AppHandle, on_battery: bool) {
    let (enabled, targets) = {
        let st = app.state::<AppState>();
        let cfg = st.config.lock().unwrap();
        (cfg.eco_enabled, cfg.eco_targets.clone())
    };
    if !on_battery || !enabled || targets.is_empty() {
        release_all(app);
        return;
    }
    let running: Vec<(u32, String)> = os::process_cpu_snapshot()
        .into_iter()
        .filter(|(_, (name, _))| targets.iter().any(|t| t.eq_ignore_ascii_case(name)))
        .map(|(pid, (name, _))| (pid, name))
        .collect();
    let ctl = app.state::<BatteryCtl>();
    let mut throttled = ctl.throttled.lock().unwrap();
    let alive: HashSet<u32> = running.iter().map(|(p, _)| *p).collect();
    throttled.retain(|p| alive.contains(p));
    let plan = battery::eco_plan(&running, os::foreground_pid(), &throttled);
    for pid in plan.throttle {
        if os::set_eco(pid, true) {
            throttled.insert(pid);
        }
    }
    for pid in plan.release {
        os::set_eco(pid, false);
        throttled.remove(&pid);
    }
}

/// Lifts EcoQoS from every process this app throttled.
pub fn release_all(app: &AppHandle) {
    let ctl = app.state::<BatteryCtl>();
    let mut throttled = ctl.throttled.lock().unwrap();
    for pid in throttled.drain() {
        os::set_eco(pid, false);
    }
}
