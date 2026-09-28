//! Companion-only settings that are not simple preference values: the custom executable,
//! the last confirmed model and the panel width. Stored next to `state.json` as
//! `companion.json` (Electron keeps the same values in its store as `companionCustomCommand`
//! and `companionLastModel`). "Enabled" and the preferred provider are regular prefs.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const DEFAULT_PANEL_WIDTH: f32 = 360.0;
pub const MIN_PANEL_WIDTH: f32 = 300.0;
pub const MAX_PANEL_WIDTH: f32 = 640.0;
/// The reader keeps at least this much room beside the panel.
pub const MIN_MAIN_WIDTH: f32 = 360.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CompanionSettings {
    pub custom_command: Option<PathBuf>,
    pub last_model: Option<String>,
    pub panel_width: f32,
}

impl Default for CompanionSettings {
    fn default() -> Self {
        Self {
            custom_command: None,
            last_model: None,
            panel_width: DEFAULT_PANEL_WIDTH,
        }
    }
}

/// The panel width for a window: the saved width, clamped so the reader keeps its room.
pub fn clamp_panel_width(width: f32, window_width: f32) -> f32 {
    let room = (window_width - MIN_MAIN_WIDTH).max(MIN_PANEL_WIDTH);
    width.clamp(MIN_PANEL_WIDTH, MAX_PANEL_WIDTH.min(room))
}

pub struct CompanionStore {
    path: Option<PathBuf>,
}

impl CompanionStore {
    /// Next to the app's `state.json`; `None` keeps everything in memory (tests, smoke runs).
    pub fn beside(state_path: &Path) -> Self {
        let path = (!state_path.as_os_str().is_empty())
            .then(|| state_path.with_file_name("companion.json"));
        Self { path }
    }

    pub fn in_memory() -> Self {
        Self { path: None }
    }

    pub fn load(&self) -> CompanionSettings {
        let Some(path) = self.path.as_ref() else {
            return CompanionSettings::default();
        };
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<CompanionSettings>(&text).ok())
            .map(|mut settings| {
                if !settings.panel_width.is_finite() {
                    settings.panel_width = DEFAULT_PANEL_WIDTH;
                }
                settings
            })
            .unwrap_or_default()
    }

    pub fn save(&self, settings: &CompanionSettings) {
        let Some(path) = self.path.as_ref() else {
            return;
        };
        let Ok(mut bytes) = serde_json::to_vec_pretty(settings) else {
            return;
        };
        bytes.push(b'\n');
        if std::fs::read(path).ok().as_deref() == Some(bytes.as_slice()) {
            return;
        }
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, &bytes).is_ok() {
            let _ = std::fs::rename(tmp, path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_next_to_the_state_file_and_tolerates_garbage() {
        let dir = tempfile::tempdir().unwrap();
        let store = CompanionStore::beside(&dir.path().join("state.json"));
        assert_eq!(store.load(), CompanionSettings::default());
        let settings = CompanionSettings {
            custom_command: Some("/usr/local/bin/agent".into()),
            last_model: Some("openai/gpt-5.4".into()),
            panel_width: 420.0,
        };
        store.save(&settings);
        assert_eq!(store.load(), settings);
        let text = std::fs::read_to_string(dir.path().join("companion.json")).unwrap();
        assert!(text.contains("customCommand"));
        std::fs::write(dir.path().join("companion.json"), "{ nope").unwrap();
        assert_eq!(store.load(), CompanionSettings::default());
        assert_eq!(
            CompanionStore::beside(Path::new("")).load(),
            CompanionSettings::default()
        );
    }

    #[test]
    fn panel_width_is_clamped_to_leave_the_reader_room() {
        assert_eq!(clamp_panel_width(100.0, 1400.0), MIN_PANEL_WIDTH);
        assert_eq!(clamp_panel_width(900.0, 1400.0), MAX_PANEL_WIDTH);
        assert_eq!(clamp_panel_width(500.0, 800.0), 440.0);
        assert_eq!(clamp_panel_width(500.0, 500.0), MIN_PANEL_WIDTH);
    }
}
