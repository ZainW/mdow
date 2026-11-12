//! A window's state: its tabs (each with a folder, one or two panes and their history), the
//! sidebar, find, Quick Open and the sheets, and every command the menus and keys run.

use crate::doc::{self, Block, Doc};
use crate::files::{self, ErrorKind, LinkTarget, Scan};
use crate::media::Media;
use crate::prefs::{self, Appearance, Prefs, ReadingWidth, SidebarMode};
use crate::reader::{self, Match};
use crate::watch::Watcher;
use crate::windows;
use rune::{TextEdit, Tx};
use rune_kit::shell::TabDragState;
use std::collections::HashSet;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

/// Everything a menu item, a key or another window can ask a window to do.
// Some of these are only sent by the Mac's menu bar and web views.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
#[derive(Clone, Debug, PartialEq)]
pub enum Cmd {
    Open(Vec<PathBuf>),
    OpenDialog,
    OpenFolderDialog,
    OpenRecent(usize),
    ClearRecents,
    NewWindow,
    NewTab,
    CloseTab,
    CloseWindow,
    /// Leave the app (Ctrl+Q on Linux; the Mac's menu bar has its own Quit).
    Quit,
    Reload,
    ShowInFinder,
    OpenInEditor,
    CopyPath,
    Find,
    FindNext,
    FindPrevious,
    UseSelectionForFind,
    ToggleSidebar,
    Sidebar(SidebarMode),
    ToggleCompanion,
    ToggleSplit,
    ActualSize,
    ZoomIn,
    ZoomOut,
    ReadingWidth(ReadingWidth),
    ToggleAppearance,
    FullScreen,
    Back,
    Forward,
    PreviousHeading,
    NextHeading,
    QuickOpen,
    NextTab,
    PreviousTab,
    SelectTab(usize),
    Settings,
    Shortcuts,
    JumpToSelection,
    Print,
    ExportPdf,
    PageSetup,
    Share,
    StartSpeaking,
    StopSpeaking,
    /// A link clicked in an HTML page: (pane, URL, ⌘ held).
    OpenUrl(u64, String, bool),
}

/// Where a pane is reading.
#[derive(Clone, Debug, PartialEq)]
pub struct Location {
    pub path: PathBuf,
    pub row: usize,
}

pub struct LoadedDoc {
    pub path: PathBuf,
    pub doc: Arc<Doc>,
    /// Gaps above each row, in units of the body size.
    pub gaps: Vec<f64>,
    /// An HTML file, shown by WebKit as the page itself.
    pub html: bool,
    pub error: Option<ErrorKind>,
    /// Bumped on every reload, so rows rebuild.
    pub version: u64,
    pub modified: Option<SystemTime>,
}

impl LoadedDoc {
    pub fn load(path: &Path, version: u64) -> LoadedDoc {
        let started = std::time::Instant::now();
        let path = files::normalize(path);
        let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        let html = files::is_html(&path);
        let (doc, error) = match files::read_document(&path) {
            Ok(text) if html => (doc::parse_html(&text, &path), None),
            Ok(text) => (doc::parse_markdown(&text, &path), None),
            Err(e) => (Doc::default(), Some(e)),
        };
        // The Mac shows the page itself in a web view. Elsewhere there's no WebKit to lean on,
        // so the page is read as a document: converted to markdown and drawn like any other.
        let html = html && crate::platform::MAC;
        let gaps = doc::gaps(&doc.blocks);
        crate::perf::note(&format!("parse {}", files::display_name(&path)), started);
        LoadedDoc { path, doc: Arc::new(doc), gaps, html, error, version, modified }
    }

    pub fn title(&self) -> String {
        self.doc.title.clone().unwrap_or_else(|| files::display_name(&self.path))
    }
}

pub struct Pane {
    pub id: u64,
    pub doc: Option<LoadedDoc>,
    pub back: Vec<Location>,
    pub forward: Vec<Location>,
    /// The rows on screen, from the virtual list.
    pub visible: Range<usize>,
    /// An anchor to scroll to once the document has rendered.
    pub pending_anchor: Option<String>,
    /// A row to scroll to once the document has rendered (Back and Forward).
    pub pending_row: Option<usize>,
}

impl Pane {
    fn new(id: u64) -> Pane {
        Pane {
            id,
            doc: None,
            back: Vec::new(),
            forward: Vec::new(),
            visible: 0..0,
            pending_anchor: None,
            pending_row: None,
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.doc.as_ref().map(|d| d.path.as_path())
    }

    /// The heading the reader is in: the last one at or above the top visible row.
    pub fn current_heading(&self) -> Option<usize> {
        let doc = self.doc.as_ref()?;
        let top = self.visible.start;
        let heads = &doc.doc.headings;
        let i = heads.partition_point(|h| h.row <= top);
        if i == 0 { (!heads.is_empty()).then_some(0) } else { Some(i - 1) }
    }
}

pub struct Tab {
    pub id: u64,
    pub panes: Vec<Pane>,
    /// The pane that has the keyboard (0, or 1 in a split).
    pub focus: usize,
    pub folder: Option<PathBuf>,
    pub scan: Option<Scan>,
    pub expanded: HashSet<PathBuf>,
    /// How much of the width the first pane of a split takes.
    pub split: f32,
}

impl Tab {
    pub fn pane(&self) -> &Pane {
        &self.panes[self.focus.min(self.panes.len() - 1)]
    }
    pub fn pane_mut(&mut self) -> &mut Pane {
        let i = self.focus.min(self.panes.len() - 1);
        &mut self.panes[i]
    }
    pub fn title(&self) -> String {
        match self.pane().doc.as_ref() {
            Some(d) => files::display_name(&d.path),
            None => "New Tab".into(),
        }
    }
}

pub struct FindState {
    pub query: TextEdit,
    pub pane: u64,
    pub matches: Vec<Match>,
    pub current: Option<usize>,
    /// For an HTML page, whether WebKit found the text (it gives no count).
    pub web_found: Option<bool>,
}

pub struct QuickOpen {
    pub query: TextEdit,
    pub selected: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sheet {
    Settings,
    Shortcuts,
}

pub struct Toast {
    pub text: String,
    pub until: f64,
}

pub struct App {
    /// This window's key in `windows`.
    pub key: String,
    /// A Settings or Keyboard Shortcuts window rather than a document window.
    pub panel: Option<Sheet>,
    pub tabs: Vec<Tab>,
    pub active: usize,
    pub tab_drag: TabDragState,
    pub prefs: Prefs,
    prefs_revision: u64,
    pub media: Media,
    /// Bumped when an image or diagram lands, so rows that show it rebuild.
    pub media_revision: u64,
    pub files_filter: TextEdit,
    pub find: Option<FindState>,
    pub find_revision: u64,
    pub quick_open: Option<QuickOpen>,
    pub sheet: Option<Sheet>,
    pub toast: Option<Toast>,
    pub companion: crate::companion::Companion,
    pub watcher: Watcher,
    pub copied_code: Option<(usize, f64)>,
    pub waker: Option<rune::Waker>,
    /// Where the ⋯ button is, for anchoring the Share menu.
    pub more_at: [f32; 2],
    /// Where each HTML pane is drawn, and in which appearance.
    pub web_rects: std::collections::HashMap<u64, ([f32; 4], bool)>,
    /// The `say` process reading the document aloud.
    speech: Option<std::process::Child>,
    /// The pane whose whole document ⌘A selected, until the selection changes.
    pub select_all: Option<u64>,
    select_all_rows: Range<usize>,
    pub clock: f64,
    pub focused: bool,
    next_id: u64,
    watch_dirty: bool,
    assets_dirty: bool,
    /// The theme assets were last requested for.
    assets_palette: Option<u32>,
    theme_revision: u64,
    assets_font: f32,
    started: bool,
    frame_saved: bool,
    submenus_dirty: bool,
    actions_done: usize,
    menu_recents: Vec<PathBuf>,
    saved_session: Vec<PathBuf>,
}

thread_local! {
    /// The appearance the last view was drawn in, so `poll` can make diagrams to match.
    pub static DRAWN_DARK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

impl App {
    pub fn new(key: String, paths: Vec<PathBuf>) -> App {
        let prefs = prefs::get();
        let mut app = App {
            key,
            panel: None,
            tabs: Vec::new(),
            active: 0,
            tab_drag: TabDragState::new(),
            prefs_revision: prefs::revision(),
            prefs,
            media: Media::default(),
            media_revision: 0,
            files_filter: TextEdit::new(""),
            find: None,
            find_revision: 0,
            quick_open: None,
            sheet: None,
            toast: None,
            companion: Default::default(),
            watcher: Watcher::default(),
            copied_code: None,
            waker: None,
            more_at: [0.0, 0.0],
            web_rects: Default::default(),
            speech: None,
            select_all: None,
            select_all_rows: 0..0,
            clock: 0.0,
            focused: false,
            next_id: 1,
            watch_dirty: true,
            assets_dirty: true,
            assets_palette: None,
            theme_revision: 0,
            assets_font: 0.0,
            started: false,
            frame_saved: false,
            submenus_dirty: true,
            actions_done: 0,
            menu_recents: Vec::new(),
            saved_session: Vec::new(),
        };
        app.add_tab();
        for path in paths {
            app.open_path(&path, OpenIn::Auto);
        }
        app
    }

    fn id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    pub fn tab(&self) -> &Tab {
        &self.tabs[self.active.min(self.tabs.len() - 1)]
    }
    pub fn tab_mut(&mut self) -> &mut Tab {
        let i = self.active.min(self.tabs.len() - 1);
        &mut self.tabs[i]
    }

    pub fn pane(&self, id: u64) -> Option<&Pane> {
        self.tabs.iter().flat_map(|t| t.panes.iter()).find(|p| p.id == id)
    }
    fn pane_mut(&mut self, id: u64) -> Option<&mut Pane> {
        self.tabs.iter_mut().flat_map(|t| t.panes.iter_mut()).find(|p| p.id == id)
    }

    pub fn dark(&self) -> bool {
        DRAWN_DARK.with(|d| d.get())
    }

    /// The palette the window draws in: the setting's, as `ui::view` picks it.
    pub fn palette(&self) -> crate::theme::Palette {
        crate::theme::current(self.prefs.appearance, self.dark())
    }

    // MARK: Setup and polling

    pub fn setup(&mut self, waker: rune::Waker) {
        windows::register(&self.key, waker.clone());
        self.waker = Some(waker.clone());
        self.media.setup(waker.clone());
        self.watcher.setup(waker.clone());
        self.companion.setup(waker);
        crate::highlight::preload();
    }

    pub fn poll(&mut self, now: f64, tx: &mut Tx) -> bool {
        self.clock = now;
        let mut changed = false;
        if !self.started {
            self.started = true;
            tx.set_title(self.window_title());
            crate::platform::set_appearance(self.prefs.appearance);
        }
        if self.key == windows::MAIN && self.submenus_dirty && !rune::is_headless() {
            // Rune makes its menus as the first window starts; the submenus go in after.
            if crate::platform::install_submenus() {
                self.submenus_dirty = false;
                crate::platform::refresh_submenus(&self.prefs.recents, self.prefs.reading_width);
            }
        }
        // MDOW_ACTIONS, one step a second after launch.
        if self.key == windows::MAIN && !rune::is_headless() {
            let steps = crate::actions::steps();
            if self.actions_done < steps.len() {
                if now > 1.0 + self.actions_done as f64 {
                    let (step, arg) = steps[self.actions_done].clone();
                    self.actions_done += 1;
                    crate::actions::run(self, &step, &arg, tx);
                    changed = true;
                }
                tx.wake_after(std::time::Duration::from_millis(250));
            }
        }
        if !self.frame_saved && !rune::is_headless() {
            self.frame_saved = rune_mac::remember_frame(&self.key);
        }
        for cmd in windows::take(&self.key) {
            self.run(cmd, tx);
            changed = true;
        }
        if prefs::revision() != self.prefs_revision {
            self.prefs_revision = prefs::revision();
            let recents_before = std::mem::take(&mut self.menu_recents);
            self.prefs = prefs::get();
            if self.key == windows::MAIN
                && let Some(newest) = self.prefs.recents.first()
                && recents_before.first() != Some(newest)
            {
                crate::platform::note_recent(newest);
            }
            self.submenus_dirty = true;
            self.menu_recents = self.prefs.recents.clone();
            crate::platform::set_appearance(self.prefs.appearance);
            self.refresh_find();
            changed = true;
        }
        if self.prefs.font_size() != self.assets_font {
            self.assets_font = self.prefs.font_size();
            self.assets_dirty = true;
        }
        if self.media.poll() {
            self.media_revision += 1;
            changed = true;
        }
        // A theme change (the setting, the system's, or `omarchy theme set`) redraws the window
        // and the diagrams and formulas in it.
        if crate::omarchy::revision() != self.theme_revision {
            self.theme_revision = crate::omarchy::revision();
            changed = true;
        }
        let palette = self.palette().id;
        if self.assets_palette != Some(palette) {
            self.assets_palette = Some(palette);
            self.assets_dirty = true;
        }
        if self.assets_dirty {
            self.assets_dirty = false;
            self.request_assets();
        }
        let changes = self.watcher.changes();
        if !changes.is_empty() && self.handle_changes(&changes, tx) {
            changed = true;
        }
        if self.watch_dirty {
            self.watch_dirty = false;
            self.update_watches();
        }
        if let Some((_, at)) = self.copied_code
            && now - at > 1.5
        {
            self.copied_code = None;
            changed = true;
        }
        if let Some(t) = &self.toast
            && now > t.until
        {
            self.toast = None;
            changed = true;
        }
        // Anchors wait for their rows to be laid out.
        let pending: Vec<(u64, String)> = self
            .tabs
            .iter()
            .flat_map(|t| t.panes.iter())
            .filter_map(|p| p.pending_anchor.clone().map(|a| (p.id, a)))
            .collect();
        for (pane, anchor) in pending {
            if let Some(p) = self.pane_mut(pane) {
                p.pending_anchor = None;
            }
            self.scroll_to_anchor(pane, &anchor, tx);
        }
        let rows: Vec<(u64, usize)> = self
            .tabs
            .iter()
            .flat_map(|t| t.panes.iter())
            .filter_map(|p| p.pending_row.map(|r| (p.id, r)))
            .collect();
        for (pane, row) in rows {
            if let Some(p) = self.pane_mut(pane) {
                p.pending_row = None;
            }
            self.scroll_to_row(pane, row, tx);
        }
        // The main window's tabs come back at the next launch.
        if self.key == windows::MAIN && !rune::is_headless() {
            let session = self.session();
            if session != self.saved_session {
                self.saved_session = session.clone();
                prefs::update(|p| p.session = vec![session]);
            }
        }
        self.sync_web_views();
        if let Some(f) = self.find.as_mut()
            && let Some(found) = crate::webview::found(&self.key, f.pane)
        {
            f.web_found = Some(found);
            changed = true;
        }
        if let Some(child) = self.speech.as_mut()
            && child.try_wait().ok().flatten().is_some()
        {
            self.speech = None;
        }
        if let Some(id) = self.select_all
            && let Some(p) = self.pane(id)
            && p.visible != self.select_all_rows
        {
            self.select_all_rows = p.visible.clone();
            tx.select_all_text(reader::rows_key(id));
        }
        if self.companion.poll(tx) {
            changed = true;
        }
        changed
    }

    fn request_assets(&mut self) {
        let p = self.palette();
        let size = self.prefs.font_size() * 1.1;
        let inline_size = self.prefs.font_size();
        let mut formulas: Vec<String> = Vec::new();
        let mut wanted: Vec<Block> = Vec::new();
        for tab in &self.tabs {
            for pane in &tab.panes {
                if let Some(d) = &pane.doc {
                    collect_assets(&d.doc.blocks, &mut wanted);
                    collect_formulas(&d.doc.blocks, &mut formulas);
                }
            }
        }
        formulas.sort();
        formulas.dedup();
        for tex in &formulas {
            self.media.request_inline_math(tex, &p, inline_size);
        }
        for b in &wanted {
            match b {
                Block::Mermaid(code) => self.media.request_mermaid(code, &p),
                Block::Math(tex) => self.media.request_math(tex, &p, size),
                Block::Images(images) => {
                    for image in images {
                        self.media.request_image(&image.src);
                    }
                }
                _ => {}
            }
        }
    }

    fn update_watches(&mut self) {
        let mut files = HashSet::new();
        let mut folders = HashSet::new();
        for tab in &self.tabs {
            for pane in &tab.panes {
                if let Some(p) = pane.path() {
                    files.insert(p.to_path_buf());
                }
            }
            if let Some(f) = &tab.folder {
                folders.insert(f.clone());
            }
        }
        self.watcher.set(files, folders);
    }

    fn handle_changes(&mut self, changes: &[PathBuf], tx: &mut Tx) -> bool {
        let mut changed = false;
        let reload = self.prefs.reload_on_change;
        // FSEvents reports resolved paths (/private/tmp, not /tmp), so compare those.
        let real = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        let changes: Vec<PathBuf> = changes.iter().map(|c| real(c)).collect();
        let mut rescans = Vec::new();
        for (ti, tab) in self.tabs.iter().enumerate() {
            if let Some(folder) = tab.folder.as_deref().map(real)
                && changes.iter().any(|c| {
                    c.starts_with(&folder) && (files::is_document(c) || c.extension().is_none())
                })
            {
                rescans.push(ti);
            }
        }
        for ti in rescans {
            if let Some(folder) = self.tabs[ti].folder.clone() {
                self.tabs[ti].scan = Some(files::scan_folder(&folder));
                changed = true;
            }
        }
        if !reload {
            return changed;
        }
        let ids: Vec<u64> = self
            .tabs
            .iter()
            .flat_map(|t| t.panes.iter())
            .filter(|p| {
                p.path().is_some_and(|path| {
                    let path = real(path);
                    changes.contains(&path)
                })
            })
            .map(|p| p.id)
            .collect();
        for id in ids {
            let Some(path) = self.pane(id).and_then(|p| p.path().map(Path::to_path_buf)) else {
                continue;
            };
            let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
            let same = self
                .pane(id)
                .and_then(|p| p.doc.as_ref())
                .is_some_and(|d| d.modified == modified && modified.is_some());
            if same {
                continue;
            }
            let version = self.id();
            if let Some(p) = self.pane_mut(id) {
                // Keeps the reader's place: rows keep their keys, so the virtual list holds the
                // row at the top of the view while the content under it changes.
                p.doc = Some(LoadedDoc::load(&path, version));
            }
            self.assets_dirty = true;
            changed = true;
        }
        if changed {
            self.refresh_find();
            tx.set_title(self.window_title());
        }
        changed
    }

    // MARK: Opening

    pub fn add_tab(&mut self) -> usize {
        let id = self.id();
        let pane = self.id();
        let folder = self.tabs.get(self.active).and_then(|t| t.folder.clone());
        let scan = folder.as_ref().map(|f| files::scan_folder(f));
        self.tabs.push(Tab {
            id,
            panes: vec![Pane::new(pane)],
            focus: 0,
            folder,
            scan,
            expanded: HashSet::new(),
            split: 0.5,
        });
        self.active = self.tabs.len() - 1;
        self.active
    }

    /// Opens `path` (a document or a folder).
    pub fn open_path(&mut self, path: &Path, how: OpenIn) {
        let path = files::normalize(path);
        if path.is_dir() {
            self.set_folder(&path);
            return;
        }
        let target_tab = match how {
            OpenIn::NewTab => self.tab_for_new_document(true),
            OpenIn::Auto => self.tab_for_new_document(false),
            OpenIn::Here => self.active,
        };
        self.active = target_tab;
        let id = self.tab().pane().id;
        self.navigate(id, &path, None, true);
    }

    /// The tab a newly opened document goes in: an empty current tab, else (with "open in
    /// tabs") a new one, else the current one. An already open document is selected instead.
    fn tab_for_new_document(&mut self, force_new: bool) -> usize {
        if self.tab().pane().doc.is_none() {
            return self.active;
        }
        if force_new || self.prefs.open_in_tabs { self.add_tab() } else { self.active }
    }

    pub fn open_paths(&mut self, paths: &[PathBuf], tx: &mut Tx) {
        for p in paths {
            // A document already open in this window is brought forward.
            if let Some(ti) = self.tabs.iter().position(|t| {
                t.panes.iter().any(|pane| pane.path() == Some(files::normalize(p).as_path()))
            }) {
                self.active = ti;
                continue;
            }
            self.open_path(p, OpenIn::Auto);
        }
        tx.set_title(self.window_title());
    }

    pub fn set_folder(&mut self, folder: &Path) {
        let folder = files::normalize(folder);
        let scan = files::scan_folder(&folder);
        let first = first_document(&scan.tree, &folder);
        let tab = self.tab_mut();
        tab.folder = Some(folder.clone());
        tab.scan = Some(scan);
        tab.expanded.clear();
        self.prefs = prefs::update(|p| {
            p.note_recent(&folder);
            p.sidebar_open = true;
            p.sidebar_mode = SidebarMode::Files;
        });
        if self.tab().pane().doc.is_none()
            && let Some(first) = first
        {
            let id = self.tab().pane().id;
            self.navigate(id, &first, None, true);
        }
        self.watch_dirty = true;
    }

    /// Shows `path` in pane `id`, keeping history.
    pub fn navigate(&mut self, id: u64, path: &Path, anchor: Option<String>, record: bool) {
        crate::perf::mark(format!("open {}", files::display_name(path)));
        let path = files::normalize(path);
        let version = self.id();
        let Some(pane) = self.pane_mut(id) else { return };
        if pane.path() == Some(path.as_path()) {
            if let Some(a) = anchor {
                pane.pending_anchor = Some(a);
            }
            return;
        }
        if record && let Some(d) = &pane.doc {
            pane.back.push(Location { path: d.path.clone(), row: pane.visible.start });
            pane.forward.clear();
        }
        let loaded = LoadedDoc::load(&path, version);
        let ok = loaded.error.is_none();
        pane.doc = Some(loaded);
        pane.visible = 0..0;
        pane.pending_anchor = anchor;
        if ok {
            self.prefs = prefs::update(|p| p.note_recent(&path));
        }
        self.assets_dirty = true;
        self.watch_dirty = true;
        self.refresh_find();
    }

    pub fn open_link(&mut self, pane: u64, href: &str, tx: &mut Tx) {
        let Some(path) = self.pane(pane).and_then(|p| p.path().map(Path::to_path_buf)) else {
            return;
        };
        if let Some(label) = href.strip_prefix("#fnref-") {
            let row = self
                .pane(pane)
                .and_then(|p| p.doc.as_ref())
                .and_then(|d| d.doc.blocks.iter().position(|b| block_refers(b, label)));
            if let Some(row) = row {
                self.scroll_to_row(pane, row, tx);
            }
            return;
        }
        if let Some(label) = href.strip_prefix("#fn-") {
            tx.scroll_into_view(format!("fn-{pane}-{label}"));
            return;
        }
        match files::resolve_link(href, &path) {
            LinkTarget::Anchor(anchor) => self.scroll_to_anchor(pane, &anchor, tx),
            LinkTarget::External(url) => {
                let _ = tx.open_url(&url);
            }
            LinkTarget::Document(target, anchor) => {
                if !target.exists() {
                    self.show_toast(format!("{} doesn't exist.", files::display_name(&target)));
                    return;
                }
                if tx.mods().cmd {
                    self.add_tab();
                    let id = self.tab().pane().id;
                    self.navigate(id, &target, anchor, true);
                } else {
                    self.navigate(pane, &target, anchor, true);
                }
                tx.scroll_to_instant(reader::scroll_key(self.tab().pane().id), 0.0, 0.0);
                tx.set_title(self.window_title());
            }
            LinkTarget::File(target) => {
                if target.exists() {
                    crate::platform::open_with_default_app(&target);
                } else {
                    self.show_toast(format!("{} doesn't exist.", files::display_name(&target)));
                }
            }
        }
    }

    pub fn scroll_to_anchor(&mut self, pane: u64, anchor: &str, tx: &mut Tx) {
        let Some(row) = self.pane(pane).and_then(|p| p.doc.as_ref()).and_then(|d| {
            d.doc
                .anchors
                .get(anchor)
                .copied()
                .or_else(|| d.doc.anchors.get(&doc::slugify(anchor)).copied())
        }) else {
            return;
        };
        self.scroll_to_row(pane, row, tx);
    }

    pub fn scroll_to_row(&mut self, pane: u64, row: usize, tx: &mut Tx) {
        let key = reader::row_key(pane, row);
        tx.scroll_into_view(key.clone());
        tx.scroll_key_to(key, 0.0);
    }

    pub fn rows_visible(&mut self, pane: u64, range: Range<usize>) -> bool {
        let Some(p) = self.pane_mut(pane) else { return false };
        let before = p.current_heading();
        p.visible = range;
        p.current_heading() != before
    }

    pub fn show_toast(&mut self, text: String) {
        self.toast = Some(Toast { text, until: self.clock + 3.0 });
    }

    pub fn window_title(&self) -> String {
        match self.tab().pane().doc.as_ref() {
            Some(d) => d.title(),
            None => "Mdow".into(),
        }
    }

    // MARK: Tabs

    pub fn select_tab(&mut self, i: usize, tx: &mut Tx) {
        if i < self.tabs.len() {
            crate::perf::mark(format!("switch to tab {}", i + 1));
            self.active = i;
            self.find = None;
            tx.set_title(self.window_title());
        }
    }

    pub fn close_tab(&mut self, i: usize, tx: &mut Tx) {
        if i >= self.tabs.len() {
            return;
        }
        if self.tabs.len() == 1 {
            if self.tabs[0].pane().doc.is_none() {
                tx.close_window();
                return;
            }
            // Closing the last tab leaves an empty one, as a browser window would.
            self.tabs.clear();
            self.add_tab();
        } else {
            self.tabs.remove(i);
            if self.active > i || self.active >= self.tabs.len() {
                self.active = self.active.saturating_sub(1);
            }
        }
        self.find = None;
        self.watch_dirty = true;
        tx.set_title(self.window_title());
    }

    // MARK: Find

    /// How documents are set right now (the column width doesn't matter to text).
    pub fn read_style(&self) -> reader::blocks::ReadStyle {
        reader::blocks::ReadStyle {
            size: self.prefs.font_size(),
            body_family: self.prefs.body_font.family(),
            code_family: self.prefs.code_font.family(),
            line: self.prefs.line_spacing.multiple(),
            column: 600.0,
        }
    }

    /// Lays pane `id`'s web view over it, below the find bar while that's open.
    pub fn place_web(&self, id: u64) {
        let Some(([x, y, w, h], dark)) = self.web_rects.get(&id).copied() else { return };
        let Some(path) =
            self.pane(id).and_then(|p| p.doc.as_ref()).filter(|d| d.html).map(|d| d.path.clone())
        else {
            return;
        };
        let top = if self.find.as_ref().is_some_and(|f| f.pane == id) { 56.0 } else { 0.0 };
        let rect = [x, y + top, w, (h - top).max(1.0)];
        crate::webview::place(
            &self.key,
            id,
            &path,
            rect,
            dark,
            self.prefs.darken_html,
            self.prefs.zoom,
        );
    }

    /// Shows the web views of the tab in front's HTML panes, hides the rest, and drops those of
    /// panes that are gone.
    fn sync_web_views(&mut self) {
        let front: Vec<u64> = self
            .tab()
            .panes
            .iter()
            .filter(|p| p.doc.as_ref().is_some_and(|d| d.html && d.error.is_none()))
            .map(|p| p.id)
            .collect();
        let alive: Vec<u64> = self
            .tabs
            .iter()
            .flat_map(|t| t.panes.iter())
            .filter(|p| p.doc.as_ref().is_some_and(|d| d.html))
            .map(|p| p.id)
            .collect();
        crate::webview::retain_panes(&self.key, &alive);
        for id in &front {
            self.place_web(*id);
        }
        crate::webview::sync(&self.key, &front, self.quick_open.is_some() || self.sheet.is_some());
    }

    /// The document in front as a page for WebKit to print, or None for an HTML page (WebKit
    /// prints the page itself).
    fn print_page(&self) -> Option<PathBuf> {
        let d = self.tab().pane().doc.as_ref()?;
        if d.html {
            return None;
        }
        crate::export::write_temp(&d.doc, &d.path, self.prefs.body_font.family())
    }

    /// ⌘A: select every text of the pane in front's document.
    pub fn select_all(&mut self, tx: &mut Tx) -> bool {
        let pane = self.tab().pane();
        if pane.doc.as_ref().is_none_or(|d| d.error.is_some()) {
            return false;
        }
        let (id, visible) = (pane.id, pane.visible.clone());
        self.select_all = Some(id);
        self.select_all_rows = visible;
        tx.select_all_text(reader::rows_key(id));
        true
    }

    /// ⌘C after ⌘A: the whole document, including rows that aren't on screen.
    pub fn copy_all(&mut self, tx: &mut Tx) -> bool {
        let pane = self.tab().pane();
        let (Some(id), Some(doc)) = (self.select_all, pane.doc.as_ref()) else { return false };
        if id != pane.id {
            return false;
        }
        let p = self.palette();
        tx.copy(reader::plain_text(doc, &self.read_style(), &p));
        true
    }

    pub fn refresh_find(&mut self) {
        let Some(find) = self.find.as_mut() else { return };
        let query = find.query.text.clone();
        let pane_id = self.tabs[self.active.min(self.tabs.len() - 1)].pane().id;
        find.pane = pane_id;
        let st = reader::blocks::ReadStyle {
            size: self.prefs.font_size(),
            body_family: self.prefs.body_font.family(),
            code_family: self.prefs.code_font.family(),
            line: self.prefs.line_spacing.multiple(),
            column: 600.0,
        };
        let p = self.palette();
        let matches = self
            .tabs
            .iter()
            .flat_map(|t| t.panes.iter())
            .find(|p| p.id == pane_id)
            .and_then(|p| p.doc.as_ref())
            .map(|d| reader::find(d, &query, &st, &p))
            .unwrap_or_default();
        let find = self.find.as_mut().unwrap();
        find.current = if matches.is_empty() {
            None
        } else {
            Some(find.current.unwrap_or(0).min(matches.len() - 1))
        };
        find.matches = matches;
        self.find_revision += 1;
    }

    /// After typing: the first match at or below the reader's place.
    /// The pane in front shows an HTML page.
    pub fn front_is_html(&self) -> bool {
        self.tab().pane().doc.as_ref().is_some_and(|d| d.html && d.error.is_none())
    }

    fn find_in_page(&mut self, backwards: bool) {
        let pane = self.tab().pane().id;
        let Some(f) = self.find.as_mut() else { return };
        f.pane = pane;
        let query = f.query.text.clone();
        if query.is_empty() {
            f.web_found = None;
            return;
        }
        crate::webview::find(&self.key, pane, &query, backwards, self.waker.clone());
    }

    pub fn find_changed(&mut self, tx: &mut Tx) {
        if self.front_is_html() {
            self.find_in_page(false);
            return;
        }
        let top = self.tab().pane().visible.start;
        if let Some(f) = self.find.as_mut() {
            f.current = None;
        }
        self.refresh_find();
        if let Some(f) = self.find.as_mut()
            && !f.matches.is_empty()
        {
            f.current = Some(f.matches.iter().position(|m| m.row >= top).unwrap_or(0));
        }
        self.reveal_match(tx);
    }

    pub fn find_step(&mut self, forward: bool, tx: &mut Tx) {
        if self.front_is_html() {
            self.find_in_page(!forward);
            return;
        }
        let Some(f) = self.find.as_mut() else { return };
        let n = f.matches.len();
        if n == 0 {
            return;
        }
        f.current = Some(match f.current {
            None => 0,
            Some(c) if forward => (c + 1) % n,
            Some(c) => (c + n - 1) % n,
        });
        self.find_revision += 1;
        self.reveal_match(tx);
    }

    fn reveal_match(&mut self, tx: &mut Tx) {
        let Some(f) = self.find.as_ref() else { return };
        let Some(m) = f.current.and_then(|c| f.matches.get(c)) else { return };
        let key = reader::row_key(f.pane, m.row);
        tx.scroll_into_view(key);
        tx.scroll_center("find-current");
    }

    // MARK: Headings

    fn heading_step(&mut self, forward: bool, tx: &mut Tx) {
        let pane = self.tab().pane();
        let id = pane.id;
        let Some(doc) = pane.doc.as_ref() else { return };
        let top = pane.visible.start;
        let heads = &doc.doc.headings;
        let target = if forward {
            heads.iter().find(|h| h.row > top).map(|h| h.row)
        } else {
            heads.iter().rev().find(|h| h.row < top).map(|h| h.row)
        };
        if let Some(row) = target {
            self.scroll_to_row(id, row, tx);
        }
    }

    // MARK: History

    fn history(&mut self, forward: bool, tx: &mut Tx) {
        let version = self.id();
        let pane = self.tab_mut().pane_mut();
        let Some(loc) = (if forward { pane.forward.pop() } else { pane.back.pop() }) else {
            return;
        };
        if let Some(d) = &pane.doc {
            let here = Location { path: d.path.clone(), row: pane.visible.start };
            if forward { pane.back.push(here) } else { pane.forward.push(here) }
        }
        pane.doc = Some(LoadedDoc::load(&loc.path, version));
        pane.visible = 0..0;
        // The new rows aren't laid out yet; `poll` scrolls once they are.
        pane.pending_row = Some(loc.row);
        self.assets_dirty = true;
        self.watch_dirty = true;
        self.refresh_find();
        tx.set_title(self.window_title());
    }

    pub fn can_go(&self, forward: bool) -> bool {
        let p = self.tab().pane();
        if forward { !p.forward.is_empty() } else { !p.back.is_empty() }
    }

    // MARK: Commands

    pub fn run(&mut self, cmd: Cmd, tx: &mut Tx) {
        if self.panel.is_some() {
            match cmd {
                Cmd::CloseTab | Cmd::CloseWindow => tx.close_window(),
                Cmd::Settings | Cmd::Shortcuts => crate::open_panel(if cmd == Cmd::Settings {
                    Sheet::Settings
                } else {
                    Sheet::Shortcuts
                }),
                // Everything else is for the document window behind it.
                other => {
                    let target = windows::document_target();
                    windows::send(&target, other);
                }
            }
            return;
        }
        match cmd {
            Cmd::Open(paths) => self.open_paths(&paths, tx),
            Cmd::OpenDialog => {
                let options =
                    rune::FileDialogOptions { title: "Open".into(), ..Default::default() }
                        .filter("Documents", &["md", "markdown", "mdx", "html", "htm"]);
                tx.open_file(options, |s: &mut App, result, tx| {
                    if let Ok(Some(path)) = result {
                        s.open_paths(&[path], tx);
                    }
                });
            }
            Cmd::OpenFolderDialog => {
                let options =
                    rune::FileDialogOptions { title: "Open Folder".into(), ..Default::default() };
                tx.open_folder(options, |s: &mut App, result, tx| {
                    if let Ok(Some(path)) = result {
                        s.set_folder(&path);
                        tx.set_title(s.window_title());
                    }
                });
            }
            Cmd::OpenRecent(i) => {
                if let Some(p) = self.prefs.recents.get(i).cloned() {
                    self.open_paths(&[p], tx);
                }
            }
            Cmd::ClearRecents => {
                self.prefs = prefs::update(|p| p.recents.clear());
            }
            Cmd::NewWindow => crate::open_new_window(Vec::new()),
            Cmd::NewTab => {
                self.add_tab();
                self.quick_open = None;
                tx.set_title(self.window_title());
            }
            Cmd::CloseTab => {
                if self.sheet.is_some() {
                    self.sheet = None;
                } else {
                    let i = self.active;
                    self.close_tab(i, tx);
                }
            }
            Cmd::CloseWindow => tx.close_window(),
            Cmd::Quit => {
                // Quitting skips the window's close handler, so do what it does.
                self.save_session();
                self.companion.shutdown();
                tx.quit();
            }
            Cmd::Reload => {
                if let Some(folder) = self.tab().folder.clone() {
                    self.tab_mut().scan = Some(files::scan_folder(&folder));
                }
                let version = self.id();
                let pane = self.tab_mut().pane_mut();
                if let Some(path) = pane.path().map(Path::to_path_buf) {
                    pane.doc = Some(LoadedDoc::load(&path, version));
                }
                let id = self.tab().pane().id;
                if self.front_is_html() {
                    crate::webview::reload(&self.key, id);
                }
                self.assets_dirty = true;
                self.refresh_find();
            }
            Cmd::ShowInFinder => {
                if let Some(p) = self.tab().pane().path() {
                    crate::platform::reveal_in_files(p);
                }
            }
            Cmd::OpenInEditor => {
                if let Some(p) = self.tab().pane().path() {
                    crate::platform::open_in_editor(p);
                }
            }
            Cmd::CopyPath => {
                if let Some(p) = self.tab().pane().path() {
                    tx.copy(p.display().to_string());
                    self.show_toast("Copied the path".into());
                }
            }
            Cmd::Find => {
                if self.find.is_none() {
                    let pane = self.tab().pane().id;
                    self.find = Some(FindState {
                        query: TextEdit::new(""),
                        pane,
                        matches: Vec::new(),
                        current: None,
                        web_found: None,
                    });
                }
                if let Some(f) = self.find.as_mut() {
                    f.query.focused = true;
                    let n = f.query.text.len();
                    f.query.set_selection(0, n);
                }
            }
            Cmd::FindNext => {
                if self.find.is_none() {
                    self.run(Cmd::Find, tx);
                } else {
                    self.find_step(true, tx);
                }
            }
            Cmd::FindPrevious => self.find_step(false, tx),
            Cmd::UseSelectionForFind => {
                let selected =
                    tx.text_selection().map(|s| s.text().to_string()).unwrap_or_default();
                if !selected.trim().is_empty() {
                    self.run(Cmd::Find, tx);
                    if let Some(f) = self.find.as_mut() {
                        f.query.replace_text(selected.trim());
                    }
                    self.find_changed(tx);
                }
            }
            Cmd::ToggleSidebar => {
                self.prefs = prefs::update(|p| p.sidebar_open = !p.sidebar_open);
            }
            Cmd::Sidebar(mode) => {
                self.prefs = prefs::update(|p| {
                    if p.sidebar_open && p.sidebar_mode == mode {
                        p.sidebar_open = false;
                    } else {
                        p.sidebar_open = true;
                        p.sidebar_mode = mode;
                    }
                });
            }
            Cmd::ToggleCompanion => {
                let doc = self.tab().pane().path().map(Path::to_path_buf);
                let folder = self.tab().folder.clone();
                self.companion.toggle(doc.as_deref(), folder.as_deref());
            }
            Cmd::ToggleSplit => {
                let id = self.id();
                let tab = self.tab_mut();
                if tab.panes.len() > 1 {
                    tab.panes.truncate(1);
                    tab.focus = 0;
                } else {
                    let mut pane = Pane::new(id);
                    if let Some(path) = tab.pane().path().map(Path::to_path_buf) {
                        pane.doc = Some(LoadedDoc::load(&path, id));
                    }
                    tab.panes.push(pane);
                    tab.focus = 1;
                }
                self.assets_dirty = true;
                self.watch_dirty = true;
            }
            Cmd::ActualSize => {
                self.prefs = prefs::update(|p| p.zoom = 1.0);
            }
            Cmd::ZoomIn => {
                self.prefs = prefs::update(|p| p.zoom_in());
            }
            Cmd::ZoomOut => {
                self.prefs = prefs::update(|p| p.zoom_out());
            }
            Cmd::ReadingWidth(w) => {
                self.prefs = prefs::update(|p| p.reading_width = w);
            }
            Cmd::ToggleAppearance => {
                let dark = self.dark();
                self.prefs = prefs::update(|p| {
                    p.appearance = if dark { Appearance::Light } else { Appearance::Dark }
                });
                crate::platform::set_appearance(self.prefs.appearance);
            }
            Cmd::FullScreen => rune_mac::toggle_full_screen(),
            Cmd::Back => self.history(false, tx),
            Cmd::Forward => self.history(true, tx),
            Cmd::PreviousHeading => self.heading_step(false, tx),
            Cmd::NextHeading => self.heading_step(true, tx),
            Cmd::QuickOpen => {
                if self.quick_open.is_some() {
                    self.quick_open = None;
                } else {
                    let mut query = TextEdit::new("");
                    query.focused = true;
                    self.quick_open = Some(QuickOpen { query, selected: 0 });
                }
            }
            Cmd::NextTab => {
                let i = (self.active + 1) % self.tabs.len();
                self.select_tab(i, tx);
            }
            Cmd::PreviousTab => {
                let i = (self.active + self.tabs.len() - 1) % self.tabs.len();
                self.select_tab(i, tx);
            }
            Cmd::SelectTab(i) => {
                // ⌘9 is the last tab, as in browsers.
                let i = if i >= 8 { self.tabs.len() - 1 } else { i };
                self.select_tab(i, tx);
            }
            Cmd::Print => {
                if self.tab().pane().doc.as_ref().is_some_and(|d| d.error.is_none()) {
                    let title = self.window_title();
                    let page = self.print_page();
                    crate::webview::print(&self.key, self.tab().pane().id, page, None, &title);
                }
            }
            Cmd::ExportPdf => {
                let Some(d) = self.tab().pane().doc.as_ref().filter(|d| d.error.is_none()) else {
                    return;
                };
                let stem = d
                    .path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or("Document".into());
                let options = rune::FileDialogOptions {
                    title: "Export as PDF".into(),
                    directory: d.path.parent().map(Path::to_path_buf),
                    file_name: Some(format!("{stem}.pdf")),
                    ..Default::default()
                }
                .filter("PDF", &["pdf"]);
                tx.save_file(options, |s: &mut App, result, _tx| {
                    if let Ok(Some(path)) = result {
                        let title = s.window_title();
                        let page = s.print_page();
                        crate::webview::print(&s.key, s.tab().pane().id, page, Some(path), &title);
                    }
                });
            }
            Cmd::PageSetup => crate::webview::page_setup(),
            Cmd::Share => {
                if let Some(p) = self.tab().pane().path() {
                    crate::webview::share(&self.key, p, self.more_at);
                }
            }
            Cmd::StartSpeaking => {
                // The selection, else the whole document, read by the system voice.
                let selected = tx
                    .text_selection()
                    .map(|s| s.text().to_string())
                    .filter(|t| !t.trim().is_empty());
                let text = selected.or_else(|| {
                    let p = self.palette();
                    self.tab()
                        .pane()
                        .doc
                        .as_ref()
                        .map(|d| reader::plain_text(d, &self.read_style(), &p))
                });
                if let Some(text) = text {
                    self.stop_speaking();
                    self.speech = crate::platform::speak(&text);
                }
            }
            Cmd::StopSpeaking => self.stop_speaking(),
            Cmd::JumpToSelection => {
                // The row the selection starts in: its key is the first path segment like
                // `p{pane}r{row}`.
                let row = tx.text_selection().and_then(|sel| {
                    sel.texts().iter().find(|t| t.is_selected()).and_then(|t| {
                        t.path()
                            .split('/')
                            .find(|seg| seg.starts_with('p') && seg.contains('r'))
                            .map(str::to_string)
                    })
                });
                if let Some(key) = row {
                    tx.scroll_center(key);
                }
            }
            Cmd::OpenUrl(pane, url, new_tab) => self.open_url_from_page(pane, &url, new_tab, tx),
            Cmd::Settings | Cmd::Shortcuts => {
                let kind = if cmd == Cmd::Settings { Sheet::Settings } else { Sheet::Shortcuts };
                if rune::is_headless() {
                    self.sheet = if self.sheet == Some(kind) { None } else { Some(kind) };
                } else {
                    crate::open_panel(kind);
                }
            }
        }
    }

    /// The document's context menu, at `press`: copying and looking up the selection, the
    /// code block under the pointer (`code`), and speech.
    pub fn reader_menu(&mut self, code: Option<String>, press: rune::PointerPress, tx: &mut Tx) {
        let selected =
            tx.text_selection().map(|s| s.text().to_string()).filter(|t| !t.trim().is_empty());
        let mut menu = rune::PopupMenu::new();
        if code.is_some() {
            menu = menu.item(rune::PopupItem::new("code", "Copy Code Block").symbol("doc.on.doc"));
        }
        if let Some(text) = &selected {
            let short: String = text.trim().chars().take(24).collect();
            let shown =
                if text.trim().chars().count() > 24 { format!("{short}…") } else { short };
            menu = menu.item(rune::PopupItem::new("copy", "Copy"));
            // Dictionary is the Mac's.
            if crate::platform::MAC {
                menu = menu.item(rune::PopupItem::new("lookup", format!("Look Up “{shown}”")));
            }
            menu = menu.item(rune::PopupItem::new("google", "Search with Google"));
        }
        menu = menu.item(rune::PopupItem::new("all", "Select All"));
        // So is the system voice.
        if crate::platform::MAC {
            menu = menu.separator().item(if self.speech.is_some() {
                rune::PopupItem::new("stop", "Stop Speaking")
            } else {
                rune::PopupItem::new("speak", "Start Speaking")
            });
        }
        tx.popup_menu(menu, press, move |s: &mut App, picked: Option<&str>, tx: &mut Tx| {
            let encode = |t: &str| -> String {
                t.trim()
                    .bytes()
                    .map(|b| match b {
                        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => {
                            (b as char).to_string()
                        }
                        b' ' => "+".to_string(),
                        _ => format!("%{b:02X}"),
                    })
                    .collect()
            };
            match picked {
                Some("code") => {
                    if let Some(c) = &code {
                        tx.copy(c.clone());
                    }
                }
                Some("copy") => {
                    if let Some(t) = &selected {
                        tx.copy(t.clone());
                    }
                }
                Some("lookup") => {
                    if let Some(t) = &selected {
                        let _ = tx.open_url(&format!("dict://{}", encode(t).replace('+', "%20")));
                    }
                }
                Some("google") => {
                    if let Some(t) = &selected {
                        let _ =
                            tx.open_url(&format!("https://www.google.com/search?q={}", encode(t)));
                    }
                }
                Some("all") => {
                    s.select_all(tx);
                }
                Some("speak") => s.run(Cmd::StartSpeaking, tx),
                Some("stop") => s.run(Cmd::StopSpeaking, tx),
                _ => {}
            }
        });
    }

    /// A file's context menu in the sidebar; `recent` adds the Recents items.
    pub fn file_menu(
        &mut self,
        path: PathBuf,
        recent: bool,
        press: rune::PointerPress,
        tx: &mut Tx,
    ) {
        let mut menu = rune::PopupMenu::new();
        if !path.is_dir() {
            menu = menu
                .item(rune::PopupItem::new("open", "Open"))
                .item(rune::PopupItem::new("tab", "Open in New Tab"))
                .separator();
        } else if recent {
            menu = menu.item(rune::PopupItem::new("open", "Open")).separator();
        }
        menu = menu
            .item(rune::PopupItem::new("finder", crate::platform::label("Show in Finder")))
            .item(rune::PopupItem::new("path", "Copy Path"));
        if recent {
            menu = menu
                .separator()
                .item(rune::PopupItem::new("remove", "Remove from Recents"))
                .item(rune::PopupItem::new("clear", "Clear Recents"));
        }
        tx.popup_menu(menu, press, move |s: &mut App, picked: Option<&str>, tx: &mut Tx| {
            match picked {
                Some("open") => s.open_paths(std::slice::from_ref(&path), tx),
                Some("tab") => {
                    s.open_path(&path, OpenIn::NewTab);
                    tx.set_title(s.window_title());
                }
                Some("finder") => crate::platform::reveal_in_files(&path),
                Some("path") => tx.copy(path.display().to_string()),
                Some("remove") => {
                    let p = path.clone();
                    s.prefs = prefs::update(|pr| pr.recents.retain(|r| *r != p));
                }
                Some("clear") => s.prefs = prefs::update(|pr| pr.recents.clear()),
                _ => {}
            }
        });
    }

    fn stop_speaking(&mut self) {
        if let Some(mut child) = self.speech.take() {
            let _ = child.kill();
        }
    }

    /// A link clicked in an HTML page: other documents open in Mdow, the rest in their apps.
    fn open_url_from_page(&mut self, pane: u64, url: &str, new_tab: bool, tx: &mut Tx) {
        let Some(path) = url.strip_prefix("file://") else {
            let _ = tx.open_url(url);
            return;
        };
        let (path, anchor) = match path.split_once('#') {
            Some((p, a)) => (p, Some(doc::percent_decode(a))),
            None => (path, None),
        };
        let path = PathBuf::from(doc::percent_decode(path));
        if !files::is_document(&path) {
            if path.exists() {
                crate::platform::open_with_default_app(&path);
            }
            return;
        }
        if new_tab {
            self.add_tab();
            let id = self.tab().pane().id;
            self.navigate(id, &path, anchor, true);
        } else {
            self.navigate(pane, &path, anchor, true);
        }
        tx.set_title(self.window_title());
    }

    /// The tabs to restore at the next launch.
    pub fn session(&self) -> Vec<PathBuf> {
        self.tabs.iter().filter_map(|t| t.pane().path().map(Path::to_path_buf)).collect()
    }

    pub fn save_session(&mut self) {
        if self.key != windows::MAIN || rune::is_headless() {
            return;
        }
        let session = self.session();
        self.saved_session = session.clone();
        self.prefs = prefs::update(|p| p.session = vec![session]);
    }

    /// Back to one empty tab, as a closed window comes back.
    pub fn reset(&mut self) {
        self.tabs.clear();
        self.add_tab();
        self.find = None;
        self.quick_open = None;
        self.sheet = None;
        self.companion.shutdown();
        self.watch_dirty = true;
        // The tabs just saved stay the session: the empty window isn't one to restore.
        self.saved_session = self.session();
    }

    /// Splits the current tab with `path` in the second pane (the `split` scene).
    pub fn open_split_of(&mut self, path: &Path) {
        let id = self.id();
        let tab = self.tab_mut();
        let mut pane = Pane::new(id);
        pane.doc = Some(LoadedDoc::load(path, id));
        tab.panes.truncate(1);
        tab.panes.push(pane);
        tab.focus = 1;
        self.assets_dirty = true;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenIn {
    /// Follow the "open in tabs" setting.
    Auto,
    NewTab,
    Here,
}

fn first_document(tree: &[files::TreeNode], folder: &Path) -> Option<PathBuf> {
    // A README at the top of the folder, else the first document.
    for name in ["README.md", "readme.md", "Readme.md", "index.md"] {
        let p = folder.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    fn first(nodes: &[files::TreeNode]) -> Option<PathBuf> {
        for n in nodes {
            match &n.children {
                None => return Some(n.path.clone()),
                Some(c) => {
                    if let Some(p) = first(c) {
                        return Some(p);
                    }
                }
            }
        }
        None
    }
    first(tree)
}

fn collect_assets(blocks: &[Block], out: &mut Vec<Block>) {
    for b in blocks {
        match b {
            Block::Mermaid(_) | Block::Math(_) | Block::Images(_) => out.push(b.clone()),
            Block::Quote(c) | Block::Alert(_, c) => collect_assets(c, out),
            Block::List(l) => {
                for item in &l.items {
                    collect_assets(&item.blocks, out);
                }
            }
            Block::Footnotes(notes) => {
                for n in notes {
                    collect_assets(&n.blocks, out);
                }
            }
            _ => {}
        }
    }
}

/// Inline formulas anywhere in `blocks`, for typesetting ahead of drawing.
fn collect_formulas(blocks: &[Block], out: &mut Vec<String>) {
    fn inlines(list: &[doc::Inline], out: &mut Vec<String>) {
        for i in list {
            match i {
                doc::Inline::Math(tex) => out.push(tex.clone()),
                doc::Inline::Strong(c)
                | doc::Inline::Emph(c)
                | doc::Inline::Strike(c)
                | doc::Inline::Underline(c)
                | doc::Inline::Mark(c)
                | doc::Inline::Sup(c)
                | doc::Inline::Sub(c)
                | doc::Inline::Link { children: c, .. } => inlines(c, out),
                _ => {}
            }
        }
    }
    for b in blocks {
        match b {
            Block::Paragraph(i) | Block::Heading { inlines: i, .. } => inlines(i, out),
            Block::Quote(c) | Block::Alert(_, c) => collect_formulas(c, out),
            Block::List(l) => {
                for item in &l.items {
                    collect_formulas(&item.blocks, out);
                }
            }
            Block::Table(t) => {
                for row in std::iter::once(&t.head).chain(t.rows.iter()) {
                    for cell in row {
                        inlines(cell, out);
                    }
                }
            }
            Block::Footnotes(notes) => {
                for n in notes {
                    collect_formulas(&n.blocks, out);
                }
            }
            _ => {}
        }
    }
}

/// Whether `b` holds a reference to footnote `label`.
fn block_refers(b: &Block, label: &str) -> bool {
    fn inlines(list: &[doc::Inline], label: &str) -> bool {
        list.iter().any(|i| match i {
            doc::Inline::FootnoteRef(l) => l == label,
            doc::Inline::Strong(c)
            | doc::Inline::Emph(c)
            | doc::Inline::Strike(c)
            | doc::Inline::Underline(c)
            | doc::Inline::Mark(c)
            | doc::Inline::Sup(c)
            | doc::Inline::Sub(c)
            | doc::Inline::Link { children: c, .. } => inlines(c, label),
            _ => false,
        })
    }
    match b {
        Block::Paragraph(i) | Block::Heading { inlines: i, .. } => inlines(i, label),
        Block::Quote(c) | Block::Alert(_, c) => c.iter().any(|b| block_refers(b, label)),
        Block::List(l) => l.items.iter().any(|it| it.blocks.iter().any(|b| block_refers(b, label))),
        Block::Table(t) => std::iter::once(&t.head)
            .chain(t.rows.iter())
            .any(|r| r.iter().any(|c| inlines(c, label))),
        _ => false,
    }
}
