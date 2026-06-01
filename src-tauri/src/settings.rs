use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppSettings {
    pub gallery_dir: Option<String>,   // None => bundled default
    pub snapshot_dir: Option<String>,  // None => app data /snapshots
    pub fullscreen: bool,
    pub child_label: String,           // e.g. "Leo · Round 5"
}

impl Default for AppSettings {
    fn default() -> Self {
        Self { gallery_dir: None, snapshot_dir: None, fullscreen: false, child_label: String::new() }
    }
}

pub fn settings_path(app_data: &PathBuf) -> PathBuf {
    app_data.join("settings.json")
}

pub fn load(app_data: &PathBuf) -> AppSettings {
    let p = settings_path(app_data);
    std::fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(app_data: &PathBuf, s: &AppSettings) -> Result<(), String> {
    std::fs::create_dir_all(app_data).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    std::fs::write(settings_path(app_data), json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_then_roundtrip() {
        let dir = std::env::temp_dir().join("kg_settings_test");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(load(&dir), AppSettings::default());
        let mut s = AppSettings::default();
        s.fullscreen = true;
        s.child_label = "Leo".into();
        save(&dir, &s).unwrap();
        assert_eq!(load(&dir), s);
    }
}
