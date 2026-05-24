use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeNode {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<TreeNode>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub tree: Vec<TreeNode>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileResult {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenFolderResult {
    pub path: String,
    pub tree: Vec<TreeNode>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTab {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub sidebar_width: u32,
    pub zoom_level: u32,
    pub last_folder: Option<String>,
    pub window_bounds: Option<WindowBounds>,
    pub session_tabs: Vec<SessionTab>,
    pub session_active_tab_path: Option<String>,
    pub content_font: String,
    pub code_font: String,
    pub font_size: f64,
    pub line_height: f64,
    pub theme: String,
    pub auto_update_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChangedPayload {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreSchema {
    #[serde(default)]
    pub recents: Vec<String>,
    #[serde(default)]
    pub last_folder: Option<String>,
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: u32,
    #[serde(default = "default_zoom_level")]
    pub zoom_level: u32,
    #[serde(default)]
    pub window_bounds: Option<WindowBounds>,
    #[serde(default)]
    pub session_tabs: Vec<SessionTab>,
    #[serde(default)]
    pub session_active_tab_path: Option<String>,
    #[serde(default = "default_content_font")]
    pub content_font: String,
    #[serde(default = "default_code_font")]
    pub code_font: String,
    #[serde(default = "default_font_size")]
    pub font_size: f64,
    #[serde(default = "default_line_height")]
    pub line_height: f64,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_auto_update_enabled")]
    pub auto_update_enabled: bool,
}

impl Default for StoreSchema {
    fn default() -> Self {
        default_schema()
    }
}

fn default_sidebar_width() -> u32 {
    260
}

fn default_zoom_level() -> u32 {
    100
}

fn default_content_font() -> String {
    "inter".into()
}

fn default_code_font() -> String {
    "geist-mono".into()
}

fn default_font_size() -> f64 {
    15.5
}

fn default_line_height() -> f64 {
    1.65
}

fn default_theme() -> String {
    "system".into()
}

fn default_auto_update_enabled() -> bool {
    true
}

pub fn default_schema() -> StoreSchema {
    StoreSchema {
        recents: vec![],
        last_folder: None,
        sidebar_width: default_sidebar_width(),
        zoom_level: default_zoom_level(),
        window_bounds: None,
        session_tabs: vec![],
        session_active_tab_path: None,
        content_font: default_content_font(),
        code_font: default_code_font(),
        font_size: default_font_size(),
        line_height: default_line_height(),
        theme: default_theme(),
        auto_update_enabled: default_auto_update_enabled(),
    }
}
