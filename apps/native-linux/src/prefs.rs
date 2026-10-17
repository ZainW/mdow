//! Preferences and session state, persisted as JSON in the XDG config directory.
//! Field names and values follow the desktop app's store (`apps/desktop/src/main/store.ts`).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ContentFont {
    #[default]
    Inter,
    Charter,
    SystemSans,
    Georgia,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
#[allow(clippy::enum_variant_names)]
pub enum CodeFont {
    #[default]
    GeistMono,
    SystemMono,
    SfMono,
    JetbrainsMono,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum InterfaceScale {
    #[default]
    Compact,
    Comfortable,
    Large,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ReadingWidth {
    #[default]
    Standard,
    Comfortable,
    Wide,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SidebarMode {
    #[default]
    Recents,
    Folder,
    Outline,
}

impl ThemeMode {
    pub const ALL: [ThemeMode; 3] = [ThemeMode::System, ThemeMode::Light, ThemeMode::Dark];
    pub fn id(self) -> &'static str {
        match self {
            ThemeMode::System => "system",
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
        }
    }
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.id() == id)
    }
}

impl ContentFont {
    pub const ALL: [ContentFont; 4] =
        [ContentFont::Inter, ContentFont::Charter, ContentFont::SystemSans, ContentFont::Georgia];
    pub fn id(self) -> &'static str {
        match self {
            ContentFont::Inter => "inter",
            ContentFont::Charter => "charter",
            ContentFont::SystemSans => "system-sans",
            ContentFont::Georgia => "georgia",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            ContentFont::Inter => "Inter",
            ContentFont::Charter => "Charter",
            ContentFont::SystemSans => "System",
            ContentFont::Georgia => "Georgia",
        }
    }
    /// A CSS font-family list.
    pub fn family(self) -> &'static str {
        match self {
            ContentFont::Inter => "\"Inter Variable\", \"Inter\", sans-serif",
            ContentFont::Charter => "\"Charter\", \"Bitstream Charter\", \"Georgia\", serif",
            ContentFont::SystemSans => "sans-serif",
            ContentFont::Georgia => "\"Georgia\", \"Times New Roman\", serif",
        }
    }
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.id() == id)
    }
}

impl CodeFont {
    pub const ALL: [CodeFont; 4] =
        [CodeFont::GeistMono, CodeFont::SystemMono, CodeFont::SfMono, CodeFont::JetbrainsMono];
    pub fn id(self) -> &'static str {
        match self {
            CodeFont::GeistMono => "geist-mono",
            CodeFont::SystemMono => "system-mono",
            CodeFont::SfMono => "sf-mono",
            CodeFont::JetbrainsMono => "jetbrains-mono",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            CodeFont::GeistMono => "Geist",
            CodeFont::SystemMono => "System",
            CodeFont::SfMono => "SF Mono",
            CodeFont::JetbrainsMono => "JetBrains",
        }
    }
    pub fn family(self) -> &'static str {
        match self {
            CodeFont::GeistMono => "\"Geist Mono\", monospace",
            CodeFont::SystemMono => "monospace",
            CodeFont::SfMono => "\"SF Mono\", \"SFMono-Regular\", monospace",
            CodeFont::JetbrainsMono => "\"JetBrains Mono\", monospace",
        }
    }
    /// The first family, for Pango markup (which takes a comma list too).
    pub fn pango_family(self) -> &'static str {
        match self {
            CodeFont::GeistMono => "Geist Mono, monospace",
            CodeFont::SystemMono => "monospace",
            CodeFont::SfMono => "SF Mono, monospace",
            CodeFont::JetbrainsMono => "JetBrains Mono, monospace",
        }
    }
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.id() == id)
    }
}

impl InterfaceScale {
    pub const ALL: [InterfaceScale; 3] =
        [InterfaceScale::Compact, InterfaceScale::Comfortable, InterfaceScale::Large];
    pub fn id(self) -> &'static str {
        match self {
            InterfaceScale::Compact => "compact",
            InterfaceScale::Comfortable => "comfortable",
            InterfaceScale::Large => "large",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            InterfaceScale::Compact => "Compact",
            InterfaceScale::Comfortable => "Comfortable",
            InterfaceScale::Large => "Large",
        }
    }
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.id() == id)
    }
}

impl ReadingWidth {
    pub const ALL: [ReadingWidth; 3] =
        [ReadingWidth::Standard, ReadingWidth::Comfortable, ReadingWidth::Wide];
    pub fn id(self) -> &'static str {
        match self {
            ReadingWidth::Standard => "standard",
            ReadingWidth::Comfortable => "comfortable",
            ReadingWidth::Wide => "wide",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            ReadingWidth::Standard => "Standard",
            ReadingWidth::Comfortable => "Comfortable",
            ReadingWidth::Wide => "Wide",
        }
    }
    /// The reading column, side padding included (48rem / 56rem / 68rem on desktop).
    pub fn px(self) -> i32 {
        match self {
            ReadingWidth::Standard => 768,
            ReadingWidth::Comfortable => 896,
            ReadingWidth::Wide => 1088,
        }
    }
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.id() == id)
    }
}

impl SidebarMode {
    pub const ALL: [SidebarMode; 3] =
        [SidebarMode::Recents, SidebarMode::Folder, SidebarMode::Outline];
    pub fn id(self) -> &'static str {
        match self {
            SidebarMode::Recents => "recents",
            SidebarMode::Folder => "folder",
            SidebarMode::Outline => "outline",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            SidebarMode::Recents => "Recents",
            SidebarMode::Folder => "Folder",
            SidebarMode::Outline => "Outline",
        }
    }
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.id() == id)
    }
}

pub const ZOOM_MIN: u32 = 60;
pub const ZOOM_MAX: u32 = 200;
pub const ZOOM_STEP: u32 = 10;
pub const MAX_RECENTS: usize = 20;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct WindowSize {
    pub width: i32,
    pub height: i32,
    pub maximized: bool,
}

impl Default for WindowSize {
    fn default() -> Self {
        WindowSize { width: 1120, height: 760, maximized: false }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Prefs {
    pub recents: Vec<String>,
    pub last_folder: Option<String>,
    pub zoom_level: u32,
    pub window: WindowSize,
    pub session_tabs: Vec<String>,
    pub session_active_tab_path: Option<String>,
    pub session_split_view: bool,
    pub session_secondary_pane_path: Option<String>,
    pub content_font: ContentFont,
    pub code_font: CodeFont,
    pub theme: ThemeMode,
    pub wide_mode: bool,
    pub interface_scale: InterfaceScale,
    pub reading_width: ReadingWidth,
    pub sidebar_mode: SidebarMode,
    pub sidebar_open: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs {
            recents: Vec::new(),
            last_folder: None,
            zoom_level: 100,
            window: WindowSize::default(),
            session_tabs: Vec::new(),
            session_active_tab_path: None,
            session_split_view: false,
            session_secondary_pane_path: None,
            content_font: ContentFont::default(),
            code_font: CodeFont::default(),
            theme: ThemeMode::default(),
            wide_mode: false,
            interface_scale: InterfaceScale::default(),
            reading_width: ReadingWidth::default(),
            sidebar_mode: SidebarMode::default(),
            sidebar_open: true,
        }
    }
}

impl Prefs {
    pub fn path() -> PathBuf {
        gtk::glib::user_config_dir().join("mdow-linux").join("state.json")
    }

    pub fn load_from(path: &Path) -> Prefs {
        let mut prefs: Prefs = std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        prefs.zoom_level = clamp_zoom(prefs.zoom_level);
        prefs
    }

    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        // Write then rename, so a crash mid-write never leaves a truncated file.
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(tmp, path)
    }

    /// Recents that still exist on disk, newest first.
    pub fn existing_recents(&mut self) -> Vec<String> {
        self.recents.retain(|path| Path::new(path).exists());
        self.recents.clone()
    }

    pub fn push_recent(&mut self, path: &str) {
        self.recents.retain(|p| p != path);
        self.recents.insert(0, path.to_string());
        self.recents.truncate(MAX_RECENTS);
    }

    pub fn remove_recent(&mut self, path: &str) {
        self.recents.retain(|p| p != path);
    }

    /// The desktop's "Reset to defaults": appearance only, never the session.
    pub fn reset_appearance(&mut self) {
        let d = Prefs::default();
        self.theme = d.theme;
        self.content_font = d.content_font;
        self.code_font = d.code_font;
        self.interface_scale = d.interface_scale;
        self.reading_width = d.reading_width;
    }
}

pub fn clamp_zoom(percent: u32) -> u32 {
    let stepped = ((percent as f64 / ZOOM_STEP as f64).round() as u32) * ZOOM_STEP;
    stepped.clamp(ZOOM_MIN, ZOOM_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_clamps_and_steps() {
        assert_eq!(clamp_zoom(0), 60);
        assert_eq!(clamp_zoom(104), 100);
        assert_eq!(clamp_zoom(106), 110);
        assert_eq!(clamp_zoom(900), 200);
    }

    #[test]
    fn recents_dedupe_and_cap() {
        let mut prefs = Prefs::default();
        for i in 0..30 {
            prefs.push_recent(&format!("/tmp/{i}.md"));
        }
        prefs.push_recent("/tmp/5.md");
        assert_eq!(prefs.recents.len(), MAX_RECENTS);
        assert_eq!(prefs.recents[0], "/tmp/5.md");
        assert_eq!(prefs.recents.iter().filter(|p| *p == "/tmp/5.md").count(), 1);
    }

    #[test]
    fn round_trips_through_json() {
        let dir = std::env::temp_dir().join(format!("mdow-prefs-{}", std::process::id()));
        let path = dir.join("state.json");
        let prefs =
            Prefs { theme: ThemeMode::Dark, reading_width: ReadingWidth::Wide, ..Prefs::default() };
        prefs.save_to(&path).unwrap();
        assert_eq!(Prefs::load_from(&path), prefs);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"readingWidth\": \"wide\""));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn unknown_or_partial_files_fall_back_to_defaults() {
        let dir = std::env::temp_dir().join(format!("mdow-prefs-partial-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        std::fs::write(&path, r#"{"zoomLevel": 130, "theme": "dark", "extra": 1}"#).unwrap();
        let prefs = Prefs::load_from(&path);
        assert_eq!(prefs.zoom_level, 130);
        assert_eq!(prefs.theme, ThemeMode::Dark);
        assert!(prefs.sidebar_open);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
