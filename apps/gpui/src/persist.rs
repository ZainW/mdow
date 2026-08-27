//! The only module that touches disk or Electron wire strings.

use crate::prefs::{
    CodeFont, ColumnWidth, ContentFont, InterfaceScale, PrefEdit, Prefs, ReaderWidth, SidebarMode,
    ThemeMode, ZoomLevel,
};
use crate::session::{Recents, SavedWindowBounds, Session, SessionTabs};
use serde::Serialize;
use serde_json::Value;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Restored {
    pub prefs: Prefs,
    pub session: Session,
}

pub struct StateStore {
    path: PathBuf,
}

impl StateStore {
    pub fn open_default() -> Self {
        Self {
            path: default_state_path(),
        }
    }

    pub fn open_at(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn in_memory() -> Self {
        Self {
            path: PathBuf::new(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Restored {
        let Ok(text) = fs::read_to_string(&self.path) else {
            return Restored::default();
        };
        let value = serde_json::from_str(&text).unwrap_or(Value::Null);
        decode(&value)
    }

    fn save(&self, prefs: &Prefs, session: &Session) {
        if self.path.as_os_str().is_empty() {
            return;
        }
        let encoded = encode(prefs, session);
        let Ok(mut bytes) = serde_json::to_vec_pretty(&encoded) else {
            return;
        };
        bytes.push(b'\n');
        if fs::read(&self.path).ok().as_deref() == Some(bytes.as_slice()) {
            return;
        }
        if let Err(error) = atomic_write(&self.path, &bytes) {
            eprintln!("mdow: failed to save state: {error}");
        }
    }
}

pub struct StoredPrefs {
    prefs: Prefs,
    store: StateStore,
}

impl StoredPrefs {
    pub fn restore(prefs: Prefs, store: StateStore) -> Self {
        Self { prefs, store }
    }

    pub fn get(&self) -> &Prefs {
        &self.prefs
    }

    pub fn apply(&mut self, edit: PrefEdit, session: &Session) -> bool {
        if !self.prefs.apply(edit) {
            return false;
        }
        self.store.save(&self.prefs, session);
        true
    }

    pub fn save_session(&self, session: &Session) {
        self.store.save(&self.prefs, session);
    }
}

fn default_state_path() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"));
    home.join("Library/Application Support/Mdow Native/state.json")
}

fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    {
        let mut file = File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(tmp, path)?;
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WireState {
    theme: String,
    content_font: String,
    code_font: String,
    interface_scale: String,
    reading_width: String,
    wide_mode: bool,
    zoom_level: u16,
    sidebar_mode: String,
    recents: Vec<String>,
    last_folder: Option<String>,
    session_tabs: Vec<WireTab>,
    session_active_tab_path: Option<String>,
    window_bounds: Option<WireBounds>,
}

#[derive(Serialize)]
struct WireTab {
    path: String,
}

#[derive(Serialize)]
struct WireBounds {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

fn encode(prefs: &Prefs, session: &Session) -> WireState {
    let (paths, active) = match &session.tabs {
        Some(tabs) => (
            tabs.iter()
                .map(|path| WireTab {
                    path: path.to_string_lossy().into_owned(),
                })
                .collect(),
            Some(tabs.active().to_string_lossy().into_owned()),
        ),
        None => (Vec::new(), None),
    };

    WireState {
        theme: theme_wire(prefs.theme_mode).to_owned(),
        content_font: content_font_wire(prefs.content_font).to_owned(),
        code_font: code_font_wire(prefs.code_font).to_owned(),
        interface_scale: interface_scale_wire(prefs.interface_scale).to_owned(),
        reading_width: column_width_wire(prefs.reader_width.column()).to_owned(),
        wide_mode: prefs.reader_width.is_full(),
        zoom_level: prefs.zoom.percent(),
        sidebar_mode: sidebar_mode_wire(prefs.sidebar_mode).to_owned(),
        recents: session
            .recents
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
        last_folder: session
            .last_folder
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        session_tabs: paths,
        session_active_tab_path: active,
        window_bounds: session.window.map(|bounds| WireBounds {
            x: bounds.x,
            y: bounds.y,
            width: bounds.width,
            height: bounds.height,
        }),
    }
}

fn decode(value: &Value) -> Restored {
    let object = value.as_object();
    let theme_mode = string_field(object, "theme")
        .map(parse_theme)
        .unwrap_or_default();
    let content_font = string_field(object, "contentFont")
        .map(parse_content_font)
        .unwrap_or_default();
    let code_font = string_field(object, "codeFont")
        .map(parse_code_font)
        .unwrap_or_default();
    let interface_scale = string_field(object, "interfaceScale")
        .map(parse_interface_scale)
        .unwrap_or_default();
    let column = string_field(object, "readingWidth")
        .map(parse_column_width)
        .unwrap_or_default();
    let wide_mode = bool_field(object, "wideMode").unwrap_or(false);
    let reader_width = if wide_mode {
        ReaderWidth::Full { returns_to: column }
    } else {
        ReaderWidth::Column(column)
    };
    let zoom = number_field(object, "zoomLevel")
        .map(ZoomLevel::from_percent)
        .unwrap_or_default();
    let sidebar_mode = string_field(object, "sidebarMode")
        .map(parse_sidebar_mode)
        .unwrap_or_default();

    let recents = Recents::from_paths(
        array_field(object, "recents")
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(PathBuf::from)
                    .collect()
            })
            .unwrap_or_default(),
    );
    let last_folder = string_field(object, "lastFolder").map(PathBuf::from);
    let tab_paths = array_field(object, "sessionTabs")
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("path").and_then(Value::as_str))
                .map(PathBuf::from)
                .collect()
        })
        .unwrap_or_default();
    let active = string_field(object, "sessionActiveTabPath").map(PathBuf::from);
    let window = object
        .and_then(|map| map.get("windowBounds"))
        .and_then(parse_window_bounds);

    Restored {
        prefs: Prefs {
            theme_mode,
            content_font,
            code_font,
            interface_scale,
            reader_width,
            zoom,
            sidebar_mode,
        },
        session: Session {
            tabs: SessionTabs::new(tab_paths, active),
            last_folder,
            recents,
            window,
        },
    }
}

fn string_field<'a>(
    object: Option<&'a serde_json::Map<String, Value>>,
    key: &str,
) -> Option<&'a str> {
    object.and_then(|map| map.get(key)).and_then(Value::as_str)
}

fn bool_field(object: Option<&serde_json::Map<String, Value>>, key: &str) -> Option<bool> {
    object.and_then(|map| map.get(key)).and_then(Value::as_bool)
}

fn number_field(object: Option<&serde_json::Map<String, Value>>, key: &str) -> Option<f64> {
    object.and_then(|map| map.get(key)).and_then(Value::as_f64)
}

fn array_field<'a>(
    object: Option<&'a serde_json::Map<String, Value>>,
    key: &str,
) -> Option<&'a Vec<Value>> {
    object
        .and_then(|map| map.get(key))
        .and_then(Value::as_array)
}

fn parse_window_bounds(value: &Value) -> Option<SavedWindowBounds> {
    let x = value.get("x")?.as_f64()? as f32;
    let y = value.get("y")?.as_f64()? as f32;
    let width = value.get("width")?.as_f64()? as f32;
    let height = value.get("height")?.as_f64()? as f32;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    Some(SavedWindowBounds {
        x,
        y,
        width,
        height,
    })
}

fn theme_wire(mode: ThemeMode) -> &'static str {
    match mode {
        ThemeMode::System => "system",
        ThemeMode::Light => "light",
        ThemeMode::Dark => "dark",
    }
}

fn parse_theme(value: &str) -> ThemeMode {
    match value {
        "light" => ThemeMode::Light,
        "dark" => ThemeMode::Dark,
        "system" => ThemeMode::System,
        _ => ThemeMode::System,
    }
}

fn content_font_wire(font: ContentFont) -> &'static str {
    match font {
        ContentFont::Inter => "inter",
        ContentFont::Charter => "charter",
        ContentFont::SystemSans => "system-sans",
        ContentFont::Georgia => "georgia",
    }
}

fn parse_content_font(value: &str) -> ContentFont {
    match value {
        "inter" => ContentFont::Inter,
        "charter" => ContentFont::Charter,
        "system-sans" => ContentFont::SystemSans,
        "georgia" => ContentFont::Georgia,
        _ => ContentFont::Inter,
    }
}

fn code_font_wire(font: CodeFont) -> &'static str {
    match font {
        CodeFont::GeistMono => "geist-mono",
        CodeFont::SystemMono => "system-mono",
        CodeFont::SfMono => "sf-mono",
        CodeFont::JetBrainsMono => "jetbrains-mono",
    }
}

fn parse_code_font(value: &str) -> CodeFont {
    match value {
        "geist-mono" => CodeFont::GeistMono,
        "system-mono" => CodeFont::SystemMono,
        "sf-mono" => CodeFont::SfMono,
        "jetbrains-mono" => CodeFont::JetBrainsMono,
        _ => CodeFont::GeistMono,
    }
}

fn interface_scale_wire(scale: InterfaceScale) -> &'static str {
    match scale {
        InterfaceScale::Compact => "compact",
        InterfaceScale::Comfortable => "comfortable",
        InterfaceScale::Large => "large",
    }
}

fn parse_interface_scale(value: &str) -> InterfaceScale {
    match value {
        "compact" => InterfaceScale::Compact,
        "comfortable" => InterfaceScale::Comfortable,
        "large" => InterfaceScale::Large,
        _ => InterfaceScale::Compact,
    }
}

fn column_width_wire(column: ColumnWidth) -> &'static str {
    match column {
        ColumnWidth::Standard => "standard",
        ColumnWidth::Comfortable => "comfortable",
        ColumnWidth::Wide => "wide",
    }
}

fn parse_column_width(value: &str) -> ColumnWidth {
    match value {
        "standard" => ColumnWidth::Standard,
        "comfortable" => ColumnWidth::Comfortable,
        "wide" => ColumnWidth::Wide,
        _ => ColumnWidth::Standard,
    }
}

fn sidebar_mode_wire(mode: SidebarMode) -> &'static str {
    match mode {
        SidebarMode::Recents => "recents",
        SidebarMode::Folder => "folder",
        SidebarMode::Outline => "outline",
    }
}

fn parse_sidebar_mode(value: &str) -> SidebarMode {
    match value {
        "recents" => SidebarMode::Recents,
        "folder" => SidebarMode::Folder,
        "outline" => SidebarMode::Outline,
        _ => SidebarMode::Recents,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prefs::PrefEdit;
    use std::fs;

    fn sample_prefs() -> Prefs {
        let mut prefs = Prefs::default();
        prefs.apply(PrefEdit::Theme(ThemeMode::Dark));
        prefs.apply(PrefEdit::ContentFont(ContentFont::Georgia));
        prefs.apply(PrefEdit::CodeFont(CodeFont::JetBrainsMono));
        prefs.apply(PrefEdit::InterfaceScale(InterfaceScale::Large));
        prefs.apply(PrefEdit::Column(ColumnWidth::Comfortable));
        prefs.apply(PrefEdit::ToggleFull);
        prefs.apply(PrefEdit::ZoomIn);
        prefs.apply(PrefEdit::Sidebar(SidebarMode::Outline));
        prefs
    }

    fn sample_session() -> Session {
        Session::from_parts(
            [
                PathBuf::from("/notes/a.md"),
                PathBuf::from("/notes/b.md"),
                PathBuf::from("/notes/c.md"),
            ],
            Some(PathBuf::from("/notes/b.md")),
            Some(PathBuf::from("/notes")),
            Recents::from_paths(vec![
                PathBuf::from("/notes/b.md"),
                PathBuf::from("/notes/a.md"),
            ]),
            Some(SavedWindowBounds {
                x: 12.0,
                y: 24.0,
                width: 1120.0,
                height: 760.0,
            }),
        )
    }

    #[test]
    fn decode_of_encode_is_identity() {
        let prefs = sample_prefs();
        let session = sample_session();
        let restored = decode(&serde_json::to_value(encode(&prefs, &session)).unwrap());
        assert_eq!(restored.prefs, prefs);
        assert_eq!(restored.session, session);
    }

    #[test]
    fn wide_mode_and_reading_width_merge_into_one_value() {
        let full = decode(&serde_json::json!({
            "wideMode": true,
            "readingWidth": "comfortable"
        }));
        assert_eq!(
            full.prefs.reader_width,
            ReaderWidth::Full {
                returns_to: ColumnWidth::Comfortable
            }
        );

        let column = decode(&serde_json::json!({
            "wideMode": false,
            "readingWidth": "wide"
        }));
        assert_eq!(
            column.prefs.reader_width,
            ReaderWidth::Column(ColumnWidth::Wide)
        );
    }

    #[test]
    fn one_bad_field_does_not_discard_the_rest() {
        let restored = decode(&serde_json::json!({
            "theme": 12,
            "contentFont": "georgia",
            "zoomLevel": "huge",
            "sidebarMode": "outline",
            "recents": ["/kept.md", 3],
            "sessionTabs": [{ "path": "/kept.md" }, "nope"],
            "sessionActiveTabPath": "/kept.md"
        }));

        assert_eq!(restored.prefs.theme_mode, ThemeMode::System);
        assert_eq!(restored.prefs.content_font, ContentFont::Georgia);
        assert_eq!(restored.prefs.zoom, ZoomLevel::default());
        assert_eq!(restored.prefs.sidebar_mode, SidebarMode::Outline);
        assert_eq!(
            restored.session.recents.iter().collect::<Vec<_>>(),
            vec![Path::new("/kept.md")]
        );
        assert_eq!(
            restored.session.tabs.as_ref().unwrap().active(),
            Path::new("/kept.md")
        );
    }

    #[test]
    fn missing_or_corrupt_file_loads_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        assert_eq!(
            StateStore::open_at(path.clone()).load(),
            Restored::default()
        );

        fs::write(&path, "not-json").unwrap();
        assert_eq!(StateStore::open_at(path).load(), Restored::default());
    }

    #[test]
    fn save_writes_electron_keys_and_omits_sidebar_open() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        let store = StateStore::open_at(path.clone());
        let prefs = sample_prefs();
        let session = sample_session();
        store.save(&prefs, &session);

        let json: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(json["theme"], "dark");
        assert_eq!(json["contentFont"], "georgia");
        assert_eq!(json["codeFont"], "jetbrains-mono");
        assert_eq!(json["interfaceScale"], "large");
        assert_eq!(json["readingWidth"], "comfortable");
        assert_eq!(json["wideMode"], true);
        assert_eq!(json["zoomLevel"], 110);
        assert_eq!(json["sidebarMode"], "outline");
        assert_eq!(json["lastFolder"], "/notes");
        assert_eq!(json["sessionActiveTabPath"], "/notes/b.md");
        assert_eq!(json["sessionTabs"][1]["path"], "/notes/b.md");
        assert!(json.get("sidebarOpen").is_none());
        assert!(json.get("companionLastModel").is_none());

        let restored = StateStore::open_at(path).load();
        assert_eq!(restored.prefs, prefs);
        assert_eq!(restored.session, session);
    }

    #[test]
    fn stored_prefs_write_through_and_skip_noop_edits() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        let mut stored = StoredPrefs::restore(Prefs::default(), StateStore::open_at(path.clone()));
        let session = Session::default();

        assert!(stored.apply(PrefEdit::Theme(ThemeMode::Light), &session));
        assert!(!stored.apply(PrefEdit::Theme(ThemeMode::Light), &session));
        assert_eq!(stored.get().theme_mode, ThemeMode::Light);

        let restored = StateStore::open_at(path).load();
        assert_eq!(restored.prefs.theme_mode, ThemeMode::Light);
    }
}
