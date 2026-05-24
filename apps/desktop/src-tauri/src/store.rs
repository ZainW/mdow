use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::Deserialize;
use tauri::{AppHandle, Manager};
use thiserror::Error;

use crate::models::{default_schema, AppState, SessionTab, StoreSchema, WindowBounds};

const MAX_RECENTS: usize = 20;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("failed to resolve app data directory: {0}")]
    Path(String),
    #[error("failed to read store: {0}")]
    Read(#[from] std::io::Error),
    #[error("failed to parse store: {0}")]
    Parse(#[from] serde_json::Error),
}

pub struct AppStore {
    path: PathBuf,
    data: Mutex<StoreSchema>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppStatePatch {
    sidebar_width: Option<u32>,
    zoom_level: Option<u32>,
    last_folder: Option<Option<String>>,
    window_bounds: Option<Option<WindowBounds>>,
    recents: Option<Vec<String>>,
    session_tabs: Option<Vec<SessionTab>>,
    session_active_tab_path: Option<Option<String>>,
    content_font: Option<String>,
    code_font: Option<String>,
    font_size: Option<f64>,
    line_height: Option<f64>,
    theme: Option<String>,
    auto_update_enabled: Option<bool>,
}

impl AppStore {
    pub fn load(app: &AppHandle) -> Result<Self, StoreError> {
        let app_data = app
            .path()
            .app_data_dir()
            .map_err(|err| StoreError::Path(err.to_string()))?;
        let store_dir = app_data.join("mdow");
        fs::create_dir_all(&store_dir)?;
        let path = store_dir.join("store.json");
        Self::load_from_path(path)
    }

    #[cfg(test)]
    fn store_path(&self) -> &std::path::Path {
        &self.path
    }

    fn load_from_path(path: PathBuf) -> Result<Self, StoreError> {
        let data = if path.exists() {
            let raw = fs::read_to_string(&path)?;
            serde_json::from_str(&raw).unwrap_or_else(|_| default_schema())
        } else {
            default_schema()
        };

        let store = Self {
            path,
            data: Mutex::new(data),
        };
        store.persist()?;
        Ok(store)
    }

    fn persist(&self) -> Result<(), StoreError> {
        let data = self.data.lock().expect("store mutex poisoned");
        let json = serde_json::to_string_pretty(&*data)?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.path, json)?;
        Ok(())
    }

    pub fn get_recents(&self) -> Vec<String> {
        self.data
            .lock()
            .expect("store mutex poisoned")
            .recents
            .clone()
    }

    pub fn add_recent(&self, file_path: &str) {
        let mut data = self.data.lock().expect("store mutex poisoned");
        data.recents.retain(|recent| recent != file_path);
        data.recents.insert(0, file_path.to_string());
        data.recents.truncate(MAX_RECENTS);
        drop(data);
        let _ = self.persist();
    }

    pub fn get_app_state(&self) -> AppState {
        let data = self.data.lock().expect("store mutex poisoned");
        AppState {
            sidebar_width: data.sidebar_width,
            zoom_level: data.zoom_level,
            last_folder: data.last_folder.clone(),
            window_bounds: data.window_bounds.clone(),
            session_tabs: data.session_tabs.clone(),
            session_active_tab_path: data.session_active_tab_path.clone(),
            content_font: data.content_font.clone(),
            code_font: data.code_font.clone(),
            font_size: data.font_size,
            line_height: data.line_height,
            theme: data.theme.clone(),
            auto_update_enabled: data.auto_update_enabled,
        }
    }

    pub fn save_app_state(&self, patch: serde_json::Value) -> Result<(), StoreError> {
        let patch: AppStatePatch = serde_json::from_value(patch)?;
        let mut data = self.data.lock().expect("store mutex poisoned");

        if let Some(value) = patch.sidebar_width {
            data.sidebar_width = value;
        }
        if let Some(value) = patch.zoom_level {
            data.zoom_level = value;
        }
        if let Some(value) = patch.last_folder {
            data.last_folder = value;
        }
        if let Some(value) = patch.window_bounds {
            data.window_bounds = value;
        }
        if let Some(value) = patch.recents {
            data.recents = value;
        }
        if let Some(value) = patch.session_tabs {
            data.session_tabs = value;
        }
        if let Some(value) = patch.session_active_tab_path {
            data.session_active_tab_path = value;
        }
        if let Some(value) = patch.content_font {
            data.content_font = value;
        }
        if let Some(value) = patch.code_font {
            data.code_font = value;
        }
        if let Some(value) = patch.font_size {
            data.font_size = value;
        }
        if let Some(value) = patch.line_height {
            data.line_height = value;
        }
        if let Some(value) = patch.theme {
            data.theme = value;
        }
        if let Some(value) = patch.auto_update_enabled {
            data.auto_update_enabled = value;
        }

        drop(data);
        self.persist()
    }

    pub fn set_last_folder(&self, folder: Option<&str>) {
        let mut data = self.data.lock().expect("store mutex poisoned");
        data.last_folder = folder.map(str::to_string);
        drop(data);
        let _ = self.persist();
    }

    pub fn set_theme(&self, theme: &str) {
        let mut data = self.data.lock().expect("store mutex poisoned");
        data.theme = theme.to_string();
        drop(data);
        let _ = self.persist();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_store() -> AppStore {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("mdow-store-test-{stamp}.json"));
        AppStore::load_from_path(path).expect("store")
    }

    #[test]
    fn default_schema_matches_electron_defaults() {
        let store = temp_store();
        let state = store.get_app_state();
        assert_eq!(state.sidebar_width, 260);
        assert_eq!(state.zoom_level, 100);
        assert!(state.last_folder.is_none());
        assert_eq!(state.content_font, "inter");
        assert_eq!(state.code_font, "geist-mono");
        assert!((state.font_size - 15.5).abs() < f64::EPSILON);
        assert!((state.line_height - 1.65).abs() < f64::EPSILON);
        assert_eq!(state.theme, "system");
        assert!(state.auto_update_enabled);
        assert!(store.get_recents().is_empty());
    }

    #[test]
    fn add_recent_moves_to_front_and_caps_at_20() {
        let store = temp_store();

        for index in 0..25 {
            store.add_recent(&format!("/tmp/file-{index}.md"));
        }

        let recents = store.get_recents();
        assert_eq!(recents.len(), 20);
        assert_eq!(recents[0], "/tmp/file-24.md");
        assert_eq!(recents[19], "/tmp/file-5.md");

        store.add_recent("/tmp/file-10.md");
        let recents = store.get_recents();
        assert_eq!(recents[0], "/tmp/file-10.md");
        assert_eq!(recents.len(), 20);
        assert_eq!(
            recents
                .iter()
                .filter(|path| *path == "/tmp/file-10.md")
                .count(),
            1
        );

        store.add_recent("/tmp/file-new.md");
        let recents = store.get_recents();
        assert_eq!(recents[0], "/tmp/file-new.md");
        assert!(!recents.contains(&"/tmp/file-5.md".to_string()));
    }

    #[test]
    fn save_app_state_partial_patch() {
        let store = temp_store();

        store
            .save_app_state(serde_json::json!({
                "sidebarWidth": 320,
                "theme": "dark",
                "fontSize": 16.0,
            }))
            .expect("save");

        let state = store.get_app_state();
        assert_eq!(state.sidebar_width, 320);
        assert_eq!(state.theme, "dark");
        assert!((state.font_size - 16.0).abs() < f64::EPSILON);
        assert_eq!(state.zoom_level, 100);
        assert_eq!(state.content_font, "inter");
    }

    #[test]
    fn persisted_store_uses_camel_case_keys() {
        let store = temp_store();
        store.add_recent("/tmp/readme.md");

        let raw = fs::read_to_string(store.store_path()).expect("read store");
        assert!(raw.contains("\"recents\""));
        assert!(raw.contains("\"sidebarWidth\""));
        assert!(raw.contains("\"autoUpdateEnabled\""));
    }
}
