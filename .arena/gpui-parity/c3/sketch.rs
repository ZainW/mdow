//! Candidate c3 type sketch. Not compiled into apps/gpui.
//!
//! Module map (intended landing sites under apps/gpui/src/):
//!
//!   boot.rs            gpui_component::init + first Theme write
//!   theme.rs           existing Mdow tokens, renamed in this sketch to MdowTokens
//!   theme_bridge.rs    MdowTokens -> gpui_component::theme::Theme (the only Global writer)
//!   prefs.rs           ThemePreference, ReadingWidth, InterfaceScale, Zoom
//!   session.rs         electron-store shaped persistence
//!   overlay.rs         exclusive chrome overlay
//!   open.rs            path kind + AppModel open arms
//!   html.rs            HTML sanitize + relative rewrite
//!   chrome/mod.rs      Input / List entities the shell owns
//!   chrome/lists.rs    private ListDelegate impls
//!   chrome/find.rs     find bar over Input
//!   chrome/palette.rs  command palette dialog
//!   chrome/settings.rs settings dialog
//!   ui/html_reader.rs  TextView::html only
//!   ui/reader.rs       unchanged DocumentBlock renderer
//!   document.rs        unchanged pulldown-cmark path
//!   syntax.rs          unchanged syntect path
//!
//! Cargo pins this sketch assumes:
//!   gpui = "=0.2.2"
//!   gpui-component = "=0.5.1"          // crates.io, gpui ^0.2.2
//!   notify = "=8.2.0"                  // Mdow FileWatcher; crate pulls notify ^7 as a second copy
//!   features off: webview, tree-sitter-languages
//!
//! Invariants encoded here:
//!   Overlay is one enum. Dual-open is unrepresentable.
//!   OpenDocument is one enum. A tab is Markdown or HTML, never both.
//!   ThemePreference resolves to ColorScheme. Tokens are derived, not stored twice.
//!   Only ThemeBridge writes the gpui-component Theme global.

#![allow(dead_code, unused_variables)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui::{
    AnyElement, App, AppContext, Context, FocusHandle, SharedString, Window, WindowAppearance,
    div, prelude::*, px,
};
use gpui_component::Root;
use gpui_component::dialog::{Dialog, DialogFooter};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::list::{List, ListDelegate, ListItem, ListState};
use gpui_component::text::{TextView, TextViewStyle};
use gpui_component::theme::{Theme, ThemeColor, ThemeMode};

use crate::app::{AppModel, AppOpenError, MdowApp, UserFacingError};
use crate::document::{ParsedDocument, load_source, parse_document};
use crate::syntax::prepare_document;
use crate::tabs::TabSet;
use crate::theme::{ColorScheme, Metrics, Theme as TokenColors};
use crate::workspace::WorkspaceTree;

// -----------------------------------------------------------------------------
// boot.rs
// -----------------------------------------------------------------------------

/// First call inside `Application::run`. Safe to call twice.
pub mod boot {
    use super::*;

    pub fn install(cx: &mut App) {
        gpui_component::init(cx);
        // Overwrite Longbridge defaults before any window paints.
        crate::theme_bridge::install(&crate::prefs::ResolvedUi::boot_default(), cx);
    }

    /// Visual tests that construct `MdowApp` must call this once on the test app.
    pub fn install_for_test(cx: &mut App) {
        install(cx);
    }
}

// -----------------------------------------------------------------------------
// prefs.rs
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePreference {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadingWidth {
    Standard,
    Comfortable,
    Wide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceScale {
    Compact,
    Comfortable,
    Large,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarMode {
    Recents,
    Folder,
    Outline,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Preferences {
    pub theme: ThemePreference,
    pub content_font: String,
    pub code_font: String,
    pub interface_scale: InterfaceScale,
    pub reading_width: ReadingWidth,
    pub zoom_level: f32,
    pub sidebar_mode: SidebarMode,
    pub sidebar_open: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: ThemePreference::System,
            content_font: Metrics::FONT_SANS.into(),
            code_font: Metrics::FONT_MONO.into(),
            interface_scale: InterfaceScale::Compact,
            reading_width: ReadingWidth::Standard,
            zoom_level: 0.0,
            sidebar_mode: SidebarMode::Folder,
            sidebar_open: true,
        }
    }
}

impl Preferences {
    pub fn color_scheme(self, appearance: WindowAppearance) -> ColorScheme {
        match self.theme {
            ThemePreference::Light => ColorScheme::Light,
            ThemePreference::Dark => ColorScheme::Dark,
            ThemePreference::System => match appearance {
                WindowAppearance::Light | WindowAppearance::VibrantLight => ColorScheme::Light,
                WindowAppearance::Dark | WindowAppearance::VibrantDark => ColorScheme::Dark,
            },
        }
    }

    pub fn reader_max_width(self) -> Option<f32> {
        match self.reading_width {
            ReadingWidth::Standard => Some(Metrics::READER_MAX_WIDTH),
            ReadingWidth::Comfortable => Some(896.0),
            ReadingWidth::Wide => None,
        }
    }

    pub fn chrome_scale(self) -> f32 {
        match self.interface_scale {
            InterfaceScale::Compact => 1.0,
            InterfaceScale::Comfortable => 1.125,
            InterfaceScale::Large => 1.25,
        }
    }
}

// -----------------------------------------------------------------------------
// theme.rs additions: tokens the reader and tests already own
// -----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MdowTokens {
    pub colors: TokenColors,
    pub chrome_font_px: f32,
    pub reader_font_px: f32,
    pub radius_px: f32,
}

impl MdowTokens {
    pub fn resolve(preference: ThemePreference, appearance: WindowAppearance) -> Self {
        let scheme = Preferences {
            theme: preference,
            ..Preferences::default()
        }
        .color_scheme(appearance);
        let colors = match scheme {
            ColorScheme::Light => TokenColors::for_appearance(WindowAppearance::Light),
            ColorScheme::Dark => TokenColors::for_appearance(WindowAppearance::Dark),
        };
        Self {
            colors,
            chrome_font_px: Metrics::APP_FONT_SIZE,
            reader_font_px: 15.5,
            radius_px: Metrics::RADIUS,
        }
    }
}

// -----------------------------------------------------------------------------
// theme_bridge.rs
// -----------------------------------------------------------------------------

/// The only module allowed to write `gpui_component::theme::Theme`.
pub mod theme_bridge {
    use super::*;

    pub struct ResolvedUi {
        pub tokens: MdowTokens,
        pub scale: f32,
        pub content_font: SharedString,
        pub code_font: SharedString,
    }

    impl ResolvedUi {
        pub fn boot_default() -> Self {
            Self {
                tokens: MdowTokens::resolve(ThemePreference::System, WindowAppearance::Light),
                scale: 1.0,
                content_font: Metrics::FONT_SANS.into(),
                code_font: Metrics::FONT_MONO.into(),
            }
        }

        pub fn from_prefs(prefs: &Preferences, appearance: WindowAppearance) -> Self {
            let mut tokens = MdowTokens::resolve(prefs.theme, appearance);
            tokens.chrome_font_px *= prefs.chrome_scale();
            tokens.reader_font_px *= prefs.chrome_scale();
            Self {
                tokens,
                scale: prefs.chrome_scale(),
                content_font: prefs.content_font.clone().into(),
                code_font: prefs.code_font.clone().into(),
            }
        }
    }

    pub fn install(ui: &ResolvedUi, cx: &mut App) {
        let colors = project_colors(&ui.tokens);
        let theme = Theme::global_mut(cx);
        theme.colors = colors;
        theme.mode = match ui.tokens.colors.color_scheme {
            ColorScheme::Light => ThemeMode::Light,
            ColorScheme::Dark => ThemeMode::Dark,
        };
        theme.font_family = ui.content_font.clone();
        theme.mono_font_family = ui.code_font.clone();
        theme.font_size = px(ui.tokens.chrome_font_px);
        theme.radius = px(ui.tokens.radius_px);
        theme.shadow = false;
        // TODO: radius_lg, scrollbar_show, highlight_theme stay crate defaults unless they
        // fight compact chrome in the first visual pass.
    }

    pub fn sync(prefs: &Preferences, appearance: WindowAppearance, cx: &mut App) {
        install(&ResolvedUi::from_prefs(prefs, appearance), cx);
    }

    /// Map the 13 Electron colors onto ThemeColor, then derive the rest.
    /// Unused slots (charts, bullish/bearish) reuse muted or primary so a
    /// default Longbridge accent cannot appear.
    pub fn project_colors(tokens: &MdowTokens) -> ThemeColor {
        let c = tokens.colors;
        let mut colors = match c.color_scheme {
            ColorScheme::Light => *ThemeColor::light(),
            ColorScheme::Dark => *ThemeColor::dark(),
        };
        colors.background = c.background;
        colors.foreground = c.foreground;
        colors.card = c.card;
        colors.muted = c.muted;
        colors.muted_foreground = c.muted_foreground;
        colors.primary = c.primary;
        colors.accent = c.accent;
        colors.danger = c.destructive;
        colors.border = c.border;
        colors.sidebar = c.sidebar;
        colors.sidebar_accent = c.sidebar_accent;
        colors.sidebar_border = c.border_subtle;
        colors.sidebar_foreground = c.foreground;
        colors.tab_bar = c.background;
        colors.tab_active = c.card;
        colors.tab = c.background;
        colors.popover = c.card;
        colors.popover_foreground = c.foreground;
        colors.input = c.card;
        colors.ring = c.primary;
        colors.link = c.primary;
        colors.overlay = c.background.opacity(0.55);
        colors.drop_target = c.primary.opacity(0.06);
        colors.drag_border = c.primary.opacity(0.46);
        colors.list = c.sidebar;
        colors.list_hover = c.sidebar_accent;
        colors.list_active = c.sidebar_accent;
        colors.list_active_border = c.accent;
        colors.title_bar = c.background;
        colors.title_bar_border = c.border_subtle;
        colors.secondary = c.muted;
        colors.secondary_foreground = c.foreground;
        colors.primary_foreground = c.background;
        colors.accent_foreground = c.background;
        colors.danger_foreground = c.background;
        colors.primary_hover = c.primary.opacity(0.88);
        colors.primary_active = c.primary.opacity(0.76);
        colors.danger_hover = c.destructive.opacity(0.88);
        colors.danger_active = c.destructive.opacity(0.76);
        colors.scrollbar_thumb = c.muted_foreground.opacity(0.35);
        colors.scrollbar_thumb_hover = c.muted_foreground.opacity(0.5);
        colors.table = c.card;
        colors.table_head = c.muted;
        colors.table_row_border = c.border_subtle;
        // Fill leftover slots from Mdow colors, never from the crate default palette.
        colors.chart_1 = c.primary;
        colors.chart_2 = c.accent;
        colors.chart_3 = c.muted_foreground;
        colors.chart_4 = c.destructive;
        colors.chart_5 = c.border;
        colors.success = c.primary;
        colors.warning = c.accent;
        colors.info = c.primary;
        colors
    }
}

// -----------------------------------------------------------------------------
// session.rs
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Session {
    pub recents: Vec<PathBuf>,
    pub last_folder: Option<PathBuf>,
    pub tabs: Vec<PathBuf>,
    pub active_tab: Option<PathBuf>,
}

impl Session {
    pub fn load() -> Self {
        unimplemented!("read JSON from app-support, ignore companion and split-view keys")
    }

    pub fn save(&self, prefs: &Preferences) {
        let _ = (self, prefs);
        unimplemented!("write electron-store key names: recents, lastFolder, sessionTabs, theme, fonts, scales")
    }

    /// Opening an existing path moves it to the front. Running twice is a no-op.
    pub fn record_recent(&mut self, path: PathBuf) {
        self.recents.retain(|existing| existing != &path);
        self.recents.insert(0, path);
        self.recents.truncate(20);
    }
}

// -----------------------------------------------------------------------------
// open.rs
// -----------------------------------------------------------------------------

pub mod open {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum OpenKind {
        Markdown,
        Html,
        Folder,
        Unsupported,
    }

    pub fn classify(path: &Path) -> OpenKind {
        if path.is_dir() {
            return OpenKind::Folder;
        }
        match path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref()
        {
            Some("md" | "markdown" | "mdx") => OpenKind::Markdown,
            Some("html" | "htm") => OpenKind::Html,
            _ => OpenKind::Unsupported,
        }
    }
}

pub use open::{OpenKind, classify};

#[derive(Debug, Clone)]
pub enum OpenDocument {
    Markdown(Arc<crate::syntax::PreparedDocument>),
    Html(Arc<PreparedHtml>),
}

impl OpenDocument {
    pub fn path(&self) -> &Path {
        match self {
            Self::Markdown(document) => &document.path,
            Self::Html(document) => &document.path,
        }
    }

    pub fn title(&self) -> &str {
        match self {
            Self::Markdown(document) => document.title.as_str(),
            Self::Html(document) => document.title.as_str(),
        }
    }

    pub fn headings(&self) -> &[crate::document::Heading] {
        match self {
            Self::Markdown(document) => document.headings.as_slice(),
            Self::Html(document) => document.headings.as_slice(),
        }
    }
}

impl AppModel {
    pub fn open_markdown(&mut self, path: &Path) -> Result<(), AppOpenError> {
        let loaded = load_source(path)?;
        let parsed = parse_document(loaded.canonical_path, loaded.source);
        self.tabs.open_prepared(prepare_document(parsed));
        Ok(())
    }

    pub fn open_html(&mut self, path: &Path) -> Result<(), AppOpenError> {
        let prepared = crate::html::load(path)?;
        self.tabs.open_html(prepared);
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// html.rs
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedHtml {
    pub path: PathBuf,
    pub title: String,
    pub source: String,
    pub display_source: String,
    pub headings: Vec<crate::document::Heading>,
}

pub mod html {
    use super::*;

    pub fn load(path: &Path) -> Result<PreparedHtml, AppOpenError> {
        let _ = path;
        unimplemented!("UTF-8 read, then prepare")
    }

    /// Rewrite relative src/href against `path.parent`.
    /// Strip script, iframe, object, embed, and on* attributes.
    /// Collect heading text from h1-h6 for the outline list.
    pub fn prepare(path: PathBuf, source: String) -> PreparedHtml {
        let _ = (path, source);
        unimplemented!("html5ever is already in the gpui-component tree; use it, do not add a second parser")
    }
}

// -----------------------------------------------------------------------------
// overlay.rs
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayKind {
    Find,
    CommandPalette,
    Settings,
    Shortcuts,
}

/// Exclusive overlay. Showing one kind replaces another.
pub struct Overlay {
    kind: Option<OverlayKind>,
}

impl Overlay {
    pub fn new() -> Self {
        Self { kind: None }
    }

    pub fn kind(&self) -> Option<OverlayKind> {
        self.kind
    }

    pub fn show(&mut self, kind: OverlayKind, window: &mut Window, cx: &mut Context<MdowApp>) {
        if self.kind == Some(kind) {
            // TODO: focus the existing input or dialog. Do not stack.
            return;
        }
        self.dismiss(window, cx);
        self.kind = Some(kind);
        match kind {
            OverlayKind::Find => {
                // Find is in-tree, not a Dialog. Focus chrome.find_input.
            }
            OverlayKind::CommandPalette => crate::chrome::palette::open(window, cx),
            OverlayKind::Settings => crate::chrome::settings::open(window, cx),
            OverlayKind::Shortcuts => crate::chrome::settings::open_shortcuts(window, cx),
        }
    }

    pub fn dismiss(&mut self, window: &mut Window, cx: &mut Context<MdowApp>) {
        let _ = (window, cx);
        self.kind = None;
        // TODO: close an open Root dialog if the crate tracks one.
    }

    pub fn render_find_bar(
        &self,
        window: &mut Window,
        cx: &mut Context<MdowApp>,
    ) -> Option<AnyElement> {
        if self.kind != Some(OverlayKind::Find) {
            return None;
        }
        Some(crate::chrome::find::render(window, cx))
    }
}

// -----------------------------------------------------------------------------
// chrome
// -----------------------------------------------------------------------------

pub struct Chrome {
    pub find_input: gpui::Entity<InputState>,
    pub palette_input: gpui::Entity<InputState>,
    pub sidebar: SidebarLists,
}

impl Chrome {
    pub fn new(window: &mut Window, cx: &mut Context<MdowApp>) -> Self {
        Self {
            find_input: cx.new(|cx| InputState::new(window, cx).placeholder("Find in document")),
            palette_input: cx.new(|cx| InputState::new(window, cx).placeholder("Type a command")),
            sidebar: SidebarLists::new(window, cx),
        }
    }

    pub fn render(&self, window: &mut Window, cx: &mut Context<MdowApp>) -> AnyElement {
        let _ = (window, cx);
        unimplemented!("existing shell: custom tab rail + breadcrumb + sidebar list + reader")
    }
}

pub struct SidebarLists {
    mode: SidebarMode,
    pub folder: gpui::Entity<ListState<FolderDelegate>>,
    pub recents: gpui::Entity<ListState<RecentsDelegate>>,
    pub outline: gpui::Entity<ListState<OutlineDelegate>>,
}

impl SidebarLists {
    pub fn new(window: &mut Window, cx: &mut Context<MdowApp>) -> Self {
        Self {
            mode: SidebarMode::Folder,
            folder: cx.new(|cx| ListState::new(FolderDelegate::default(), window, cx)),
            recents: cx.new(|cx| ListState::new(RecentsDelegate::default(), window, cx)),
            outline: cx.new(|cx| ListState::new(OutlineDelegate::default(), window, cx)),
        }
    }

    pub fn mode(&self) -> SidebarMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: SidebarMode, model: &AppModel, cx: &mut Context<MdowApp>) {
        self.mode = mode;
        match mode {
            SidebarMode::Folder => self.rebuild_folder(model, cx),
            SidebarMode::Recents => self.rebuild_recents(cx),
            SidebarMode::Outline => self.rebuild_outline(model, cx),
        }
    }

    fn rebuild_folder(&mut self, model: &AppModel, cx: &mut Context<MdowApp>) {
        let _ = (model, cx);
        unimplemented!("copy WorkspaceTree::visible_rows into the delegate, notify ListState")
    }

    fn rebuild_recents(&mut self, cx: &mut Context<MdowApp>) {
        let _ = cx;
        unimplemented!("copy Session.recents")
    }

    fn rebuild_outline(&mut self, model: &AppModel, cx: &mut Context<MdowApp>) {
        let _ = (model, cx);
        unimplemented!("copy active OpenDocument::headings")
    }
}

/// ListDelegate requires equal row height. Folder, recents, and outline rows
/// are all Metrics compact (~24 px). Depth is padding, not height.
#[derive(Default)]
pub struct FolderDelegate {
    rows: Vec<crate::workspace::WorkspaceRow>,
    selected: Option<usize>,
}

#[derive(Default)]
pub struct RecentsDelegate {
    rows: Vec<PathBuf>,
    selected: Option<usize>,
}

#[derive(Default)]
pub struct OutlineDelegate {
    rows: Vec<crate::document::Heading>,
    selected: Option<usize>,
}

impl ListDelegate for FolderDelegate {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.rows.len()
    }

    fn render_item(
        &mut self,
        ix: gpui_component::IndexPath,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let row = self.rows.get(ix.row)?;
        Some(
            ListItem::new(ix)
                // TODO: indent by row.depth * 10.0, folder/file icon, 12 px Inter,
                // debug_selector workspace-row-{ix}. Height must match every other row.
                .child(row.name.clone()),
        )
    }

    fn set_selected_index(
        &mut self,
        ix: Option<gpui_component::IndexPath>,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) {
        self.selected = ix.map(|ix| ix.row);
    }

    fn confirm(&mut self, _secondary: bool, _window: &mut Window, _cx: &mut Context<ListState<Self>>) {
        unimplemented!("file -> MdowApp::open_path; directory -> toggle_directory + rebuild")
    }
}

impl ListDelegate for RecentsDelegate {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.rows.len()
    }

    fn render_item(
        &mut self,
        ix: gpui_component::IndexPath,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let path = self.rows.get(ix.row)?;
        Some(ListItem::new(ix).child(path.display().to_string()))
    }

    fn set_selected_index(
        &mut self,
        ix: Option<gpui_component::IndexPath>,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) {
        self.selected = ix.map(|ix| ix.row);
    }

    fn confirm(&mut self, _secondary: bool, _window: &mut Window, _cx: &mut Context<ListState<Self>>) {
        unimplemented!("open the selected recent path")
    }
}

impl ListDelegate for OutlineDelegate {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.rows.len()
    }

    fn render_item(
        &mut self,
        ix: gpui_component::IndexPath,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let heading = self.rows.get(ix.row)?;
        Some(ListItem::new(ix).child(heading.text.clone()))
    }

    fn set_selected_index(
        &mut self,
        ix: Option<gpui_component::IndexPath>,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) {
        self.selected = ix.map(|ix| ix.row);
    }

    fn confirm(&mut self, _secondary: bool, _window: &mut Window, _cx: &mut Context<ListState<Self>>) {
        unimplemented!("scroll the DocumentBlock reader to heading, ignore HTML until TextView exposes anchors")
    }
}

pub mod find {
    use super::*;

    pub fn render(window: &mut Window, cx: &mut Context<MdowApp>) -> AnyElement {
        let _ = (window, cx);
        unimplemented!(
            "28-36 px bar, Input::new(&chrome.find_input).small().cleanable(true), prefix search icon"
        )
    }

    pub fn subscribe(input: &gpui::Entity<InputState>, cx: &mut Context<MdowApp>) {
        let _ = (input, cx);
        unimplemented!("InputEvent::Change searches ParsedDocument::plain_text; Html uses display_source stripped of tags")
    }
}

pub mod palette {
    use super::*;

    pub fn open(window: &mut Window, cx: &mut Context<MdowApp>) {
        window.open_dialog(cx, |dialog, _, _| {
            dialog.title("Commands").child("palette body")
        });
    }

    #[derive(Default)]
    pub struct PaletteDelegate {
        query: String,
        rows: Vec<PaletteCommand>,
        selected: Option<usize>,
    }

    #[derive(Debug, Clone)]
    pub struct PaletteCommand {
        pub title: SharedString,
        pub action: PaletteAction,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum PaletteAction {
        OpenFile,
        OpenFolder,
        ToggleSidebar,
        Find,
        Settings,
        ToggleWide,
    }

    impl ListDelegate for PaletteDelegate {
        type Item = ListItem;

        fn items_count(&self, _section: usize, _cx: &App) -> usize {
            self.rows.len()
        }

        fn render_item(
            &mut self,
            ix: gpui_component::IndexPath,
            _window: &mut Window,
            _cx: &mut Context<ListState<Self>>,
        ) -> Option<Self::Item> {
            let row = self.rows.get(ix.row)?;
            Some(ListItem::new(ix).child(row.title.clone()))
        }

        fn set_selected_index(
            &mut self,
            ix: Option<gpui_component::IndexPath>,
            _window: &mut Window,
            _cx: &mut Context<ListState<Self>>,
        ) {
            self.selected = ix.map(|ix| ix.row);
        }

        fn perform_search(
            &mut self,
            query: &str,
            _window: &mut Window,
            _cx: &mut Context<ListState<Self>>,
        ) -> gpui::Task<()> {
            self.query = query.to_owned();
            unimplemented!("filter the fixed command list; also offer recent files")
        }

        fn confirm(
            &mut self,
            _secondary: bool,
            _window: &mut Window,
            _cx: &mut Context<ListState<Self>>,
        ) {
            unimplemented!("dispatch PaletteAction, then Overlay::dismiss")
        }
    }
}

pub mod settings {
    use super::*;

    pub fn open(window: &mut Window, cx: &mut Context<MdowApp>) {
        window.open_dialog(cx, |dialog, _, _| {
            dialog
                .title("Settings")
                .child("theme, fonts, scale, reading width, zoom")
        });
    }

    pub fn open_shortcuts(window: &mut Window, cx: &mut Context<MdowApp>) {
        window.open_dialog(cx, |dialog, _, _| dialog.title("Shortcuts").child("keymap"));
    }
}

// -----------------------------------------------------------------------------
// ui/html_reader.rs
// -----------------------------------------------------------------------------

pub mod html_reader {
    use super::*;

    pub fn render(
        document: &PreparedHtml,
        tokens: &MdowTokens,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        TextView::html(
            ("html-document", document.path.to_string_lossy().into_owned()),
            document.display_source.clone(),
            window,
            cx,
        )
        .selectable(true)
        .scrollable(true)
        .style(style(tokens))
        .into_any_element()
    }

    pub fn style(tokens: &MdowTokens) -> TextViewStyle {
        let _ = tokens;
        unimplemented!(
            "heading_base_font_size = reader 15.5 * scale, paragraph_gap from 1.65 line-height, is_dark from tokens"
        )
    }
}

// -----------------------------------------------------------------------------
// MdowApp ownership (delta on the existing struct)
// -----------------------------------------------------------------------------

pub struct MdowAppChromeDelta {
    pub prefs: Preferences,
    pub session: Session,
    pub overlay: Overlay,
    pub chrome: Chrome,
    pub tokens: MdowTokens,
}

impl MdowApp {
    pub fn open_classified(&mut self, path: &Path, cx: &mut Context<Self>) {
        match classify(path) {
            OpenKind::Markdown => {
                if let Err(error) = self.model.open_markdown(path) {
                    self.open_error = Some(error.into_view());
                }
            }
            OpenKind::Html => {
                if let Err(error) = self.model.open_html(path) {
                    self.open_error = Some(error.into_view());
                }
            }
            OpenKind::Folder => {
                self.model.open_workspace(path).ok();
            }
            OpenKind::Unsupported => {
                self.open_error = Some(UserFacingError {
                    title: "Unsupported file type".into(),
                    body: "Mdow opens .md, .markdown, .mdx, .html, and .htm files.".into(),
                    path: path.to_owned(),
                });
            }
        }
        cx.notify();
    }
}

// TabSet grows one method. Markdown tabs keep PreparedDocument identity.
impl TabSet {
    pub fn open_html(&mut self, document: PreparedHtml) {
        let _ = document;
        unimplemented!("same path-identity rules as open_prepared, store OpenDocument::Html")
    }
}

// ShellLayout.for_width loses the wide_mode bool.
pub fn reader_width(window_width: f32, sidebar_open: bool, width: ReadingWidth) -> f32 {
    let sidebar = if sidebar_open { Metrics::SIDEBAR_WIDTH } else { 0.0 };
    let main = (window_width - sidebar).max(0.0);
    match width {
        ReadingWidth::Standard => Metrics::READER_MAX_WIDTH.min(main),
        ReadingWidth::Comfortable => 896.0_f32.min(main),
        ReadingWidth::Wide => (main - Metrics::READER_INSET * 2.0).max(0.0),
    }
}
