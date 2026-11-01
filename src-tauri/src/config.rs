use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::keyboard::KeyboardSettings;
use crate::power::{Options, UiMode};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub lang: Option<String>,
    pub power_mode: UiMode,
    pub auto_game: bool,
    pub acdc_switch: bool,
    pub kb_color: [u8; 3],
    pub kb_brightness: u8,
    pub kb_rainbow: bool,
    pub igpu_on_battery: bool,
    pub gpu_auto_reset: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            lang: None,
            power_mode: UiMode::Auto,
            auto_game: false,
            acdc_switch: true,
            kb_color: [255, 255, 255],
            kb_brightness: 100,
            kb_rainbow: false,
            igpu_on_battery: true,
            gpu_auto_reset: false,
        }
    }
}

impl Config {
    pub fn power_options(&self) -> Options {
        Options { auto_game: self.auto_game, acdc_switch: self.acdc_switch }
    }

    pub fn keyboard(&self) -> KeyboardSettings {
        KeyboardSettings { rgb: self.kb_color, brightness: self.kb_brightness, rainbow: self.kb_rainbow }
    }

    pub fn set_keyboard(&mut self, k: &KeyboardSettings) {
        self.kb_color = k.rgb;
        self.kb_brightness = k.brightness;
        self.kb_rainbow = k.rainbow;
    }
}

pub fn dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("GiENEMY")
}

fn path() -> PathBuf {
    dir().join("config.json")
}

/// Dropped by a non-elevated launch to ask the running (elevated) instance
/// to show its window; the task itself cannot carry that intent.
pub fn show_request_path() -> PathBuf {
    dir().join("show.request")
}

pub fn load_from(path: &Path) -> Config {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_to(path: &Path, config: &Config) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

pub fn load() -> Config {
    load_from(&path())
}

pub fn save(config: &Config) -> Result<(), String> {
    save_to(&path(), config)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("gienemy-test-{}-{name}", std::process::id())).join("config.json")
    }

    #[test]
    fn config_roundtrip() {
        let path = temp("lang");
        let cfg = Config { lang: Some("tr".into()), ..Config::default() };
        save_to(&path, &cfg).unwrap();
        assert_eq!(load_from(&path), cfg);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn config_roundtrip_keeps_power_fields() {
        let path = temp("power");
        let cfg = Config { power_mode: UiMode::Performance, auto_game: true, acdc_switch: false, ..Config::default() };
        save_to(&path, &cfg).unwrap();
        assert_eq!(load_from(&path), cfg);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn older_config_gets_power_defaults() {
        let path = temp("old");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, r#"{"lang":"en"}"#).unwrap();
        let cfg = load_from(&path);
        assert_eq!(cfg.lang.as_deref(), Some("en"));
        assert!(cfg.acdc_switch);
        assert_eq!(cfg.power_mode, UiMode::Auto);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn config_keeps_keyboard_fields() {
        let path = temp("kb");
        let cfg = Config { kb_color: [10, 20, 30], kb_brightness: 40, kb_rainbow: true, ..Config::default() };
        save_to(&path, &cfg).unwrap();
        assert_eq!(load_from(&path), cfg);
        assert_eq!(Config::default().kb_brightness, 100);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn missing_config_is_default() {
        assert_eq!(load_from(Path::new(r"C:\does\not\exist\config.json")), Config::default());
    }
}
