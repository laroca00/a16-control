use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub lang: Option<String>,
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

    #[test]
    fn config_roundtrip() {
        let dir = std::env::temp_dir().join(format!("gienemy-test-{}", std::process::id()));
        let path = dir.join("config.json");
        save_to(&path, &Config { lang: Some("tr".into()) }).unwrap();
        assert_eq!(load_from(&path), Config { lang: Some("tr".into()) });
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_config_is_default() {
        assert_eq!(load_from(Path::new(r"C:\does\not\exist\config.json")), Config::default());
    }
}
