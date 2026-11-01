//! Keyboard backlight. The A16 keyboard is a HID LampArray with a single
//! lamp; colour goes in a LampRangeUpdate feature report and brightness is
//! applied by scaling the colour, because the lamp's intensity is on/off.

use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

const REPORT_LEN: usize = 51;
const CONTROL_REPORT: u8 = 6;
const RANGE_UPDATE_REPORT: u8 = 5;
const STEP: u8 = 10;
// A tick gap this long means the machine slept; the firmware then falls back
// to its own lighting effect and the colour must be written again.
const RESUME_GAP: Duration = Duration::from_secs(10);

pub const PRESETS: [(&str, [u8; 3]); 8] = [
    ("white", [255, 255, 255]),
    ("red", [255, 0, 0]),
    ("orange", [255, 128, 0]),
    ("green", [0, 255, 0]),
    ("cyan", [0, 255, 255]),
    ("blue", [0, 96, 255]),
    ("purple", [160, 32, 240]),
    ("pink", [255, 0, 170]),
];

/// LampArrayControl with AutonomousMode off: the host drives the lamp.
pub fn control_report() -> Vec<u8> {
    vec![CONTROL_REPORT, 0]
}

pub fn scale(rgb: [u8; 3], brightness: u8) -> [u8; 3] {
    let b = brightness.min(100) as u32;
    rgb.map(|c| ((c as u32 * b + 50) / 100) as u8)
}

/// LampRangeUpdate for lamp 0..0 with the "update complete" flag set.
pub fn color_report(rgb: [u8; 3], brightness: u8) -> Vec<u8> {
    let [r, g, b] = scale(rgb, brightness);
    let mut report = vec![RANGE_UPDATE_REPORT, 1, 0, 0, 0, 0, r, g, b, 255];
    report.resize(REPORT_LEN, 0);
    report
}

/// Fully saturated colour for a hue in degrees.
pub fn hue_rgb(deg: f32) -> [u8; 3] {
    let h = deg.rem_euclid(360.0) / 60.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();
    let (r, g, b) = match h as u32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    [r, g, b].map(|c: f32| (c * 255.0).round() as u8)
}

/// Next brightness step in the pressed direction; a value between steps
/// first lands on the nearest step that way.
pub fn step_brightness(current: u8, up: bool) -> u8 {
    let c = current.min(100);
    let rem = c % STEP;
    let next = match (up, rem) {
        (true, 0) => c.saturating_add(STEP),
        (true, r) => c + (STEP - r),
        (false, 0) => c.saturating_sub(STEP),
        (false, r) => c - r,
    };
    next.min(100)
}

pub fn is_resume(gap: Duration) -> bool {
    gap >= RESUME_GAP
}

pub trait LampDevice: Send {
    fn write(&mut self, report: &[u8]) -> Result<(), String>;
}

type Request = ([u8; 3], u8);
type Shared = Arc<(Mutex<Option<Request>>, Condvar)>;

const RETRY_CAP: Duration = Duration::from_secs(15);

/// Writes colours from a background thread so HID I/O never blocks the UI.
/// Only the newest request matters: a slider drag queues nothing.
pub struct Lamp {
    shared: Shared,
}

impl Lamp {
    pub fn start(opener: impl Fn() -> Option<Box<dyn LampDevice>> + Send + 'static) -> Lamp {
        Self::start_with(opener, Duration::from_secs(1))
    }

    pub fn start_with(opener: impl Fn() -> Option<Box<dyn LampDevice>> + Send + 'static, retry: Duration) -> Lamp {
        let shared: Shared = Arc::new((Mutex::new(None), Condvar::new()));
        let worker = shared.clone();
        thread::Builder::new()
            .name("lamp".into())
            .spawn(move || run(worker, opener, retry))
            .expect("lamp thread");
        Lamp { shared }
    }

    pub fn set(&self, rgb: [u8; 3], brightness: u8) {
        let (lock, cvar) = &*self.shared;
        *lock.lock().unwrap() = Some((rgb, brightness));
        cvar.notify_one();
    }
}

fn run(shared: Shared, opener: impl Fn() -> Option<Box<dyn LampDevice>>, retry: Duration) {
    let (lock, cvar) = &*shared;
    let mut device: Option<Box<dyn LampDevice>> = None;
    loop {
        let mut req = {
            let mut pending = lock.lock().unwrap();
            while pending.is_none() {
                pending = cvar.wait(pending).unwrap();
            }
            pending.take().unwrap()
        };
        // The keyboard can be missing for a while after sleep or a driver
        // reload: keep retrying with backoff, but jump to a newer request.
        let mut delay = retry;
        while !write(&mut device, &opener, req) {
            let pending = lock.lock().unwrap();
            let (mut pending, _) = cvar.wait_timeout(pending, delay).unwrap();
            if let Some(newer) = pending.take() {
                req = newer;
            }
            delay = (delay * 2).min(RETRY_CAP);
        }
    }
}

fn write(device: &mut Option<Box<dyn LampDevice>>, opener: &impl Fn() -> Option<Box<dyn LampDevice>>, req: Request) -> bool {
    // A handle from before sleep can be stale; drop it and try once fresh.
    for _ in 0..2 {
        if device.is_none() {
            *device = opener();
        }
        let Some(dev) = device.as_mut() else { return false };
        if dev.write(&control_report()).and_then(|_| dev.write(&color_report(req.0, req.1))).is_ok() {
            return true;
        }
        *device = None;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    mod lamp {
        use super::super::*;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::{Arc, Mutex};
        use std::time::Instant;

        type Log = Arc<Mutex<Vec<Vec<u8>>>>;

        struct FakeLamp(Log);
        impl LampDevice for FakeLamp {
            fn write(&mut self, report: &[u8]) -> Result<(), String> {
                self.0.lock().unwrap().push(report.to_vec());
                Ok(())
            }
        }

        fn wait_for(log: &Log, n: usize) -> Vec<Vec<u8>> {
            let t = Instant::now();
            while log.lock().unwrap().len() < n && t.elapsed() < Duration::from_secs(3) {
                std::thread::sleep(Duration::from_millis(5));
            }
            std::thread::sleep(Duration::from_millis(50));
            log.lock().unwrap().clone()
        }

        #[test]
        fn lamp_writes_control_then_color() {
            let log: Log = Default::default();
            let l = log.clone();
            let lamp = Lamp::start_with(move || Some(Box::new(FakeLamp(l.clone())) as Box<dyn LampDevice>), Duration::from_millis(10));
            lamp.set([255, 0, 0], 100);
            let written = wait_for(&log, 2);
            assert_eq!(written, vec![control_report(), color_report([255, 0, 0], 100)]);
        }

        #[test]
        fn lamp_applies_only_latest_request() {
            let log: Log = Default::default();
            let present = Arc::new(AtomicUsize::new(0));
            let (l, p) = (log.clone(), present.clone());
            let lamp = Lamp::start_with(
                move || (p.load(Ordering::SeqCst) == 1).then(|| Box::new(FakeLamp(l.clone())) as Box<dyn LampDevice>),
                Duration::from_millis(20),
            );
            lamp.set([1, 1, 1], 100);
            lamp.set([2, 2, 2], 100);
            lamp.set([3, 3, 3], 100);
            present.store(1, Ordering::SeqCst);
            let written = wait_for(&log, 2);
            assert_eq!(written, vec![control_report(), color_report([3, 3, 3], 100)]);
        }

        #[test]
        fn lamp_retries_until_device_appears() {
            let log: Log = Default::default();
            let opens = Arc::new(AtomicUsize::new(0));
            let (l, o) = (log.clone(), opens.clone());
            let lamp = Lamp::start_with(
                move || (o.fetch_add(1, Ordering::SeqCst) >= 2).then(|| Box::new(FakeLamp(l.clone())) as Box<dyn LampDevice>),
                Duration::from_millis(10),
            );
            lamp.set([0, 0, 255], 50);
            let written = wait_for(&log, 2);
            assert_eq!(written.last().unwrap(), &color_report([0, 0, 255], 50));
            assert!(opens.load(Ordering::SeqCst) >= 3);
        }
    }

    /// Writes a colour to the real keyboard:
    /// `cargo test keyboard::tests::hid_probe -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn hid_probe() {
        let mut dev = crate::hw::hid::open_lamp().expect("lamp interface");
        println!("control {:?}", dev.write(&control_report()));
        println!("color {:?}", dev.write(&color_report([64, 0, 64], 53)));
    }

    #[test]
    fn control_report_takes_host_control() {
        assert_eq!(control_report(), vec![6, 0]);
    }

    #[test]
    fn color_report_layout() {
        let r = color_report([255, 128, 0], 100);
        assert_eq!(r.len(), 51);
        assert_eq!(&r[..10], &[5, 1, 0, 0, 0, 0, 255, 128, 0, 255]);
        assert!(r[10..].iter().all(|b| *b == 0));
    }

    #[test]
    fn brightness_scales_rgb() {
        assert_eq!(scale([255, 128, 10], 50), [128, 64, 5]);
        assert_eq!(scale([255, 255, 255], 100), [255, 255, 255]);
    }

    #[test]
    fn hue_wheel_primaries() {
        assert_eq!(hue_rgb(0.0), [255, 0, 0]);
        assert_eq!(hue_rgb(120.0), [0, 255, 0]);
        assert_eq!(hue_rgb(240.0), [0, 0, 255]);
        assert_eq!(hue_rgb(360.0), [255, 0, 0]);
    }

    #[test]
    fn step_brightness_snaps_and_clamps() {
        assert_eq!(step_brightness(47, true), 50);
        assert_eq!(step_brightness(47, false), 40);
        assert_eq!(step_brightness(50, true), 60);
        assert_eq!(step_brightness(100, true), 100);
        assert_eq!(step_brightness(0, false), 0);
    }

    #[test]
    fn zero_brightness_is_black_even_in_rainbow() {
        assert_eq!(&color_report(hue_rgb(200.0), 0)[6..9], &[0, 0, 0]);
    }

    #[test]
    fn long_gap_means_resume() {
        assert!(is_resume(Duration::from_secs(30)));
        assert!(!is_resume(Duration::from_secs(2)));
    }
}
