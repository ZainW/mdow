//! App settings, shared by every window and saved as JSON in Application Support.
//!
//! Windows hold a copy and compare [`revision`] in `poll`, so a change made in one window (or in
//! Settings) reaches the others on their next frame.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

pub const DEFAULT_TEXT_SIZE: f32 = 16.0;
pub const ZOOM_STEPS: [f32; 13] =
    [0.5, 0.67, 0.75, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0];
const MAX_RECENTS: usize = 30;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

impl Appearance {
    pub const ALL: [Appearance; 3] = [Appearance::System, Appearance::Light, Appearance::Dark];
    pub fn name(self) -> &'static str {
        match self {
            Appearance::System => "Automatic",
            Appearance::Light => "Light",
            Appearance::Dark => "Dark",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BodyFont {
    #[default]
    Charter,
    Inter,
    System,
    NewYork,
    Georgia,
    Avenir,
}

impl BodyFont {
    pub const ALL: [BodyFont; 6] = [
        BodyFont::Charter,
        BodyFont::Inter,
        BodyFont::System,
        BodyFont::NewYork,
        BodyFont::Georgia,
        BodyFont::Avenir,
    ];
    pub fn name(self) -> &'static str {
        match self {
            BodyFont::Charter => "Charter",
            BodyFont::Inter => "Inter",
            BodyFont::System => "San Francisco",
            BodyFont::NewYork => "New York",
            BodyFont::Georgia => "Georgia",
            BodyFont::Avenir => "Avenir Next",
        }
    }
    /// The CSS family list Rune loads: bundled faces first, installed ones on first use.
    pub fn family(self) -> &'static str {
        match self {
            BodyFont::Charter => "Charter",
            BodyFont::Inter => "Inter Variable",
            BodyFont::System => "SF Pro Text, system-ui, Inter Variable",
            BodyFont::NewYork => "New York, Charter",
            BodyFont::Georgia => "Georgia, Charter",
            BodyFont::Avenir => "Avenir Next, Inter Variable",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CodeFont {
    #[default]
    GeistMono,
    SfMono,
    Menlo,
    Monaco,
}

impl CodeFont {
    pub const ALL: [CodeFont; 4] =
        [CodeFont::GeistMono, CodeFont::SfMono, CodeFont::Menlo, CodeFont::Monaco];
    pub fn name(self) -> &'static str {
        match self {
            CodeFont::GeistMono => "Geist Mono",
            CodeFont::SfMono => "SF Mono",
            CodeFont::Menlo => "Menlo",
            CodeFont::Monaco => "Monaco",
        }
    }
    pub fn family(self) -> rune::Family {
        match self {
            CodeFont::GeistMono => rune::Family::Named("Geist Mono"),
            CodeFont::SfMono => rune::Family::SystemMono,
            CodeFont::Menlo => rune::Family::Named("Menlo, Geist Mono"),
            CodeFont::Monaco => rune::Family::Named("Monaco, Geist Mono"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LineSpacing {
    Compact,
    #[default]
    Standard,
    Relaxed,
}

impl LineSpacing {
    pub const ALL: [LineSpacing; 3] =
        [LineSpacing::Compact, LineSpacing::Standard, LineSpacing::Relaxed];
    pub fn name(self) -> &'static str {
        match self {
            LineSpacing::Compact => "Compact",
            LineSpacing::Standard => "Standard",
            LineSpacing::Relaxed => "Relaxed",
        }
    }
    /// Line height as a multiple of the font size. TextKit's line height multiple applies to
    /// the font's own line height (about 1.2 em), so these read as the Swift app's values.
    pub fn multiple(self) -> f32 {
        match self {
            LineSpacing::Compact => 1.18 * 1.2,
            LineSpacing::Standard => 1.32 * 1.2,
            LineSpacing::Relaxed => 1.5 * 1.2,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReadingWidth {
    Narrow,
    #[default]
    Standard,
    Wide,
    Full,
}

impl ReadingWidth {
    pub const ALL: [ReadingWidth; 4] =
        [ReadingWidth::Narrow, ReadingWidth::Standard, ReadingWidth::Wide, ReadingWidth::Full];
    pub fn name(self) -> &'static str {
        match self {
            ReadingWidth::Narrow => "Narrow",
            ReadingWidth::Standard => "Standard",
            ReadingWidth::Wide => "Wide",
            ReadingWidth::Full => "Full Width",
        }
    }
    /// The text column's maximum width in points at the default text size.
    pub fn points(self) -> Option<f32> {
        match self {
            ReadingWidth::Narrow => Some(600.0),
            ReadingWidth::Standard => Some(720.0),
            ReadingWidth::Wide => Some(920.0),
            ReadingWidth::Full => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SidebarMode {
    #[default]
    Outline,
    Files,
    Recents,
}

impl SidebarMode {
    pub fn name(self) -> &'static str {
        match self {
            SidebarMode::Outline => "Outline",
            SidebarMode::Files => "Files",
            SidebarMode::Recents => "Recents",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Prefs {
    pub appearance: Appearance,
    pub body_font: BodyFont,
    pub code_font: CodeFont,
    pub line_spacing: LineSpacing,
    pub reading_width: ReadingWidth,
    pub text_size: f32,
    pub zoom: f32,
    pub open_in_tabs: bool,
    pub reload_on_change: bool,
    /// Darken HTML pages that have no dark appearance of their own, in dark mode.
    pub darken_html: bool,
    /// The companion panel's width.
    pub companion_width: f32,
    pub sidebar_mode: SidebarMode,
    pub sidebar_open: bool,
    pub sidebar_width: f32,
    /// The companion's model, `providerID/modelID`; None uses OpenCode's default.
    pub companion_model: Option<String>,
    pub recents: Vec<PathBuf>,
    /// The tabs of each window at the last quit, for restoring them.
    pub session: Vec<Vec<PathBuf>>,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs {
            appearance: Appearance::System,
            body_font: BodyFont::Charter,
            code_font: CodeFont::GeistMono,
            line_spacing: LineSpacing::Standard,
            reading_width: ReadingWidth::Standard,
            text_size: DEFAULT_TEXT_SIZE,
            zoom: 1.0,
            open_in_tabs: true,
            reload_on_change: true,
            darken_html: true,
            companion_width: 380.0,
            sidebar_mode: SidebarMode::Outline,
            sidebar_open: true,
            sidebar_width: 240.0,
            companion_model: None,
            recents: Vec::new(),
            session: Vec::new(),
        }
    }
}

impl Prefs {
    /// Body text size in points, zoom included.
    pub fn font_size(&self) -> f32 {
        (self.text_size * self.zoom).round()
    }

    /// The text column's width; it grows with the text so lines keep their length when zoomed.
    pub fn column_width(&self, available: f32) -> f32 {
        match self.reading_width.points() {
            None => available,
            Some(points) => available.min(points * self.text_size * self.zoom / DEFAULT_TEXT_SIZE),
        }
    }

    pub fn can_zoom_in(&self) -> bool {
        self.zoom < *ZOOM_STEPS.last().unwrap()
    }
    pub fn can_zoom_out(&self) -> bool {
        self.zoom > ZOOM_STEPS[0]
    }
    pub fn zoom_in(&mut self) {
        self.zoom =
            ZOOM_STEPS.iter().copied().find(|z| *z > self.zoom + 0.001).unwrap_or(self.zoom);
    }
    pub fn zoom_out(&mut self) {
        self.zoom =
            ZOOM_STEPS.iter().rev().copied().find(|z| *z < self.zoom - 0.001).unwrap_or(self.zoom);
    }

    pub fn reset_reading(&mut self) {
        let d = Prefs::default();
        self.body_font = d.body_font;
        self.code_font = d.code_font;
        self.line_spacing = d.line_spacing;
        self.reading_width = d.reading_width;
        self.text_size = d.text_size;
        self.zoom = d.zoom;
    }

    pub fn note_recent(&mut self, path: &Path) {
        let path = crate::files::normalize(path);
        self.recents.retain(|p| *p != path);
        self.recents.insert(0, path);
        self.recents.truncate(MAX_RECENTS);
    }
}

static PREFS: Mutex<Option<Prefs>> = Mutex::new(None);
static REVISION: AtomicU64 = AtomicU64::new(1);

fn file() -> PathBuf {
    crate::platform::settings_file()
}

fn load() -> Prefs {
    // Headless runs (snapshots, scripts, tests) start from the defaults, so they're repeatable.
    if rune::is_headless() || cfg!(test) {
        return Prefs::default();
    }
    std::fs::read(file()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

/// The settings as they stand.
pub fn get() -> Prefs {
    PREFS.lock().unwrap().get_or_insert_with(load).clone()
}

/// Changes the settings, saves them and bumps the revision every window watches.
pub fn update(f: impl FnOnce(&mut Prefs)) -> Prefs {
    let mut guard = PREFS.lock().unwrap();
    let prefs = guard.get_or_insert_with(load);
    let before = prefs.clone();
    f(prefs);
    let after = prefs.clone();
    drop(guard);
    if after != before {
        REVISION.fetch_add(1, Ordering::Relaxed);
        // Like `load`, tests and headless runs keep to memory and leave the real file alone.
        if !rune::is_headless() && !cfg!(test) {
            let path = file();
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Ok(json) = serde_json::to_vec_pretty(&after) {
                let _ = std::fs::write(path, json);
            }
        }
        crate::windows::wake_all();
    }
    after
}

pub fn revision() -> u64 {
    REVISION.load(Ordering::Relaxed)
}
