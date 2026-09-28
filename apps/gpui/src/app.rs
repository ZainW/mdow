use crate::{
    actions::{
        CheckForUpdates, CloseTab, Dismiss, FindNext, FindPrevious, NextTab, OpenFile, OpenFolder,
        PreviousTab, SelectLastTab, SelectTab1, SelectTab2, SelectTab3, SelectTab4, SelectTab5,
        SelectTab6, SelectTab7, SelectTab8, SidebarFolder, SidebarOutline, SidebarRecents,
        ToggleFind, TogglePalette, ToggleSettings, ToggleShortcuts, ToggleSidebar, ToggleSplitView,
        ToggleWideMode, ZoomIn, ZoomOut, ZoomReset,
    },
    actions::{ClearRecents, Minimize, OpenRecent, ToggleFullScreen, Zoom},
    anchor::ScrollAnchor,
    companion::{
        ToggleCompanion,
        ui::{CompanionHost, DocumentContext},
    },
    document::{
        ASYNC_PARSE_MIN_BYTES, DocumentError, LoadedSource, PREVIEW_MIN_BYTES, ParsedDocument,
        is_supported_document, is_supported_markdown, load_source, parse_document,
        slice_document_head,
    },
    overlay::{
        CommandId, FindEvent, FindHit, FindMatches, FindOverlay, OpenOverlay, OverlayHost,
        OverlayKind, PaletteAction, PaletteEvent, PaletteOverlay, SettingsEvent, SettingsPanel,
        ShortcutsCard, ShortcutsEvent,
    },
    persist::{SessionRole, StateStore, StoredPrefs},
    prefs::{PrefEdit, Prefs, SidebarMode, ThemeMode},
    session::{Recents, SavedWindowBounds, Session},
    sparkle::{self, UpdateUi},
    split::{PaneId, SplitState},
    syntax::{PreparedDocument, prepare_document},
    tabs::{TabLoad, TabSet},
    theme::{Metrics, ShellLayout, Theme},
    ui::{
        cheat_sheet::{self, CheatSheetHold, render_cheat_sheet},
        chrome::{
            SidebarProps, TabFocus, render_breadcrumb, render_deleted_banner, render_empty_toolbar,
            render_error_banner, render_reload_error_banner, render_sidebar, render_tab_bar,
            render_update_banner,
        },
        field::{self, Field, FieldEvent},
        primitives::{ContextMenu, ContextMenuEntry, ContextMenuEvent, context_menu_layer},
        reader::{
            LinkFocusKey, LinkRoute, LinkSurfaceKey, ReaderPane, classify_link,
            clear_expired_code_copy_feedback,
        },
        split_view::{SplitPane, render_split},
        welcome::{DropSummary, drop_overlay, error_state, welcome},
        zoom_hud::{self, ZoomHud, ZoomHudHandlers, render_zoom_hud},
    },
    watcher::{FileWatcher, WatchMessage},
    workspace::{WorkspaceEntryKind, WorkspaceError, WorkspaceTree, scan_workspace},
};
use gpui::{
    App, ClipboardItem, Context, DragMoveEvent, Entity, ExternalPaths, FocusHandle, Focusable,
    IntoElement, PathPromptOptions, Pixels, Point, Render, Subscription, Task, Timer, Window, div,
    prelude::*, px,
};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc::Receiver},
    time::{Duration, Instant},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserFacingError {
    pub title: String,
    pub body: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppOpenError {
    Document(UserFacingError),
    Workspace(UserFacingError),
}

impl AppOpenError {
    pub fn view(&self) -> &UserFacingError {
        match self {
            Self::Document(view) | Self::Workspace(view) => view,
        }
    }

    pub fn into_view(self) -> UserFacingError {
        match self {
            Self::Document(view) | Self::Workspace(view) => view,
        }
    }
}

impl From<DocumentError> for AppOpenError {
    fn from(error: DocumentError) -> Self {
        Self::Document(UserFacingError {
            title: error.title().into(),
            body: error.body().into(),
            path: error.path().to_owned(),
        })
    }
}

impl From<WorkspaceError> for AppOpenError {
    fn from(error: WorkspaceError) -> Self {
        Self::Workspace(UserFacingError {
            title: error.title().into(),
            body: error.body().into(),
            path: error.path().to_owned(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentOpened {
    ActivatedExisting,
    LoadedFromDisk,
}

/// How [`AppModel::begin_open`] went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenStart {
    ActivatedExisting,
    LoadedFromDisk,
    /// A placeholder tab is open; run the load off the UI thread.
    Pending(PendingLoad),
}

/// A large document to read and parse off the UI thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingLoad {
    pub path: PathBuf,
    pub generation: u64,
    /// Show the opening of the document before the full parse finishes.
    pub preview: bool,
    /// A live reload of an open tab rather than a first open.
    pub reload: bool,
}

/// `Some(size)` when `path` is large enough to load off the UI thread; validates the path the
/// way [`load_source`] would so bad opens still fail synchronously.
fn deferred_load_size(path: &Path) -> Result<Option<u64>, DocumentError> {
    if !is_supported_document(path) {
        return Err(DocumentError::Unsupported {
            path: path.to_owned(),
        });
    }
    let metadata = std::fs::metadata(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            DocumentError::Missing {
                path: path.to_owned(),
            }
        } else {
            DocumentError::Read {
                path: path.to_owned(),
                message: error.to_string(),
            }
        }
    })?;
    Ok((metadata.len() >= ASYNC_PARSE_MIN_BYTES).then_some(metadata.len()))
}

/// What the background half of a [`PendingLoad`] reports back, in order.
enum LoadEvent {
    Preview(PreparedDocument),
    Full(PreparedDocument),
    Failed(DocumentError),
}

/// Background half of a [`PendingLoad`]: read the file and, for a preview, parse its opening.
pub fn read_for_load(
    load: &PendingLoad,
) -> Result<(LoadedSource, Option<PreparedDocument>), DocumentError> {
    let loaded = load_source(&load.path)?;
    let preview = load
        .preview
        .then(|| slice_document_head(&loaded.source))
        .flatten()
        .map(|head| {
            PreparedDocument::preview(parse_document(
                loaded.canonical_path.clone(),
                head.to_owned(),
            ))
        });
    Ok((loaded, preview))
}

/// Background half of a [`PendingLoad`]: parse the whole document.
pub fn parse_for_load(loaded: LoadedSource) -> PreparedDocument {
    prepare_document(parse_document(loaded.canonical_path, loaded.source))
}

#[derive(Debug, Default)]
pub struct AppModel {
    pub tabs: TabSet,
    pub workspace: Option<WorkspaceTree>,
    pub workspace_error: Option<UserFacingError>,
    pub recents: Recents,
    /// Open tabs whose file disappeared from disk; their last content stays readable.
    pub deleted: HashSet<PathBuf>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BatchOpenResult {
    pub document_error: Option<UserFacingError>,
    pub workspace_error: Option<UserFacingError>,
    document_attempted: bool,
    document_opened: bool,
}

impl BatchOpenResult {
    pub fn document_attempted(&self) -> bool {
        self.document_attempted
    }

    pub fn document_opened(&self) -> bool {
        self.document_opened
    }
}

impl AppModel {
    pub fn open_document(&mut self, path: &Path) -> Result<(), AppOpenError> {
        let loaded = load_source(path)?;
        self.deleted.remove(&loaded.canonical_path);
        let parsed = parse_document(loaded.canonical_path, loaded.source);
        self.tabs.open_prepared(prepare_document(parsed));
        if let Some(tab) = self.tabs.active() {
            self.recents.note(tab.path());
        }
        Ok(())
    }

    pub fn open_or_activate(&mut self, path: &Path) -> Result<DocumentOpened, AppOpenError> {
        if self.tabs.activate(path) {
            return Ok(DocumentOpened::ActivatedExisting);
        }
        self.open_document(path)?;
        Ok(DocumentOpened::LoadedFromDisk)
    }

    pub fn reload_path(&mut self, path: &Path) -> Result<(), AppOpenError> {
        let tab_path = canonical_file_identity(path);
        let loaded = match load_source(path) {
            Ok(loaded) => loaded,
            Err(error) => {
                if matches!(error, DocumentError::Missing { .. }) {
                    self.deleted.insert(tab_path.clone());
                } else {
                    self.deleted.remove(&tab_path);
                }
                let error = AppOpenError::from(error);
                self.tabs
                    .set_reload_error(&tab_path, error.view().body.clone());
                return Err(error);
            }
        };
        self.deleted.remove(&tab_path);
        let parsed = parse_document(loaded.canonical_path, loaded.source);
        self.tabs.replace_prepared(prepare_document(parsed));
        Ok(())
    }

    /// Opens `path` without blocking on a large file: small files load synchronously (no
    /// loading flash), larger ones get a placeholder tab at once and a [`PendingLoad`] for the
    /// caller to run off the UI thread. `allow_preview` lets huge Markdown files show their
    /// opening first (fresh opens only; a restored reading position needs the whole document).
    pub fn begin_open(
        &mut self,
        path: &Path,
        allow_preview: bool,
    ) -> Result<OpenStart, AppOpenError> {
        if self.tabs.activate(path) {
            return Ok(OpenStart::ActivatedExisting);
        }
        let Some(size) = deferred_load_size(path)? else {
            self.open_document(path)?;
            return Ok(OpenStart::LoadedFromDisk);
        };
        let canonical = path.canonicalize().map_err(|error| DocumentError::Read {
            path: path.to_owned(),
            message: error.to_string(),
        })?;
        self.deleted.remove(&canonical);
        let generation = self.tabs.open_loading(&canonical, Instant::now());
        self.recents.note(&canonical);
        Ok(OpenStart::Pending(PendingLoad {
            preview: allow_preview
                && is_supported_markdown(&canonical)
                && size >= PREVIEW_MIN_BYTES as u64,
            path: canonical,
            generation,
            reload: false,
        }))
    }

    /// Reloads a changed tab: small files synchronously, large ones as a [`PendingLoad`] while
    /// the tab keeps showing its current version. Read failures apply at once either way.
    pub fn begin_reload(&mut self, path: &Path) -> Result<Option<PendingLoad>, AppOpenError> {
        match deferred_load_size(path) {
            Ok(Some(_)) => {
                let tab_path = canonical_file_identity(path);
                let Some(generation) = self.tabs.begin_reload(&tab_path) else {
                    return Ok(None);
                };
                Ok(Some(PendingLoad {
                    path: tab_path,
                    generation,
                    preview: false,
                    reload: true,
                }))
            }
            Ok(None) | Err(_) => self.reload_path(path).map(|()| None),
        }
    }

    /// Applies a background read failure: an open fails its placeholder tab, a reload keeps
    /// the last good copy with an error banner. Stale results change nothing.
    pub fn fail_load(&mut self, load: &PendingLoad, error: DocumentError) -> Option<AppOpenError> {
        if !self.tabs.is_current(&load.path, load.generation) {
            return None;
        }
        if load.reload {
            if matches!(error, DocumentError::Missing { .. }) {
                self.deleted.insert(load.path.clone());
            } else {
                self.deleted.remove(&load.path);
            }
            let error = AppOpenError::from(error);
            self.tabs
                .set_reload_error(&load.path, error.view().body.clone());
            Some(error)
        } else {
            self.tabs.close(&load.path);
            Some(AppOpenError::from(error))
        }
    }

    /// Installs a background parse result; returns whether it was still current.
    pub fn finish_load(&mut self, load: &PendingLoad, document: PreparedDocument) -> bool {
        let applied = self.tabs.apply_loaded(document, load.generation);
        if applied && load.reload {
            self.deleted.remove(&load.path);
        }
        applied
    }

    /// Installs a folder scan that ran off the UI thread.
    pub fn apply_workspace_scan(
        &mut self,
        scanned: Result<WorkspaceTree, WorkspaceError>,
    ) -> Result<(), AppOpenError> {
        match scanned {
            Ok(workspace) => {
                self.workspace = Some(workspace);
                self.workspace_error = None;
                Ok(())
            }
            Err(error) => {
                let error = AppOpenError::from(error);
                self.workspace_error = Some(error.view().clone());
                Err(error)
            }
        }
    }

    pub fn open_workspace(&mut self, path: &Path) -> Result<(), AppOpenError> {
        match scan_workspace(path) {
            Ok(workspace) => {
                self.workspace = Some(workspace);
                self.workspace_error = None;
                Ok(())
            }
            Err(error) => {
                let error = AppOpenError::from(error);
                self.workspace_error = Some(error.view().clone());
                Err(error)
            }
        }
    }

    /// Applies a background rescan of `root`; returns whether the visible tree changed.
    pub fn apply_workspace_rescan(
        &mut self,
        root: &Path,
        scanned: Result<WorkspaceTree, WorkspaceError>,
    ) -> bool {
        let Some(current) = self
            .workspace
            .as_ref()
            .filter(|tree| tree.root.path == root)
        else {
            return false;
        };
        let Ok(mut scanned) = scanned else {
            // A vanished or unreadable folder keeps its last listing until the reader acts.
            return false;
        };
        if current.same_entries(&scanned) {
            return false;
        }
        scanned.restore_expansion(&current.expanded_directories());
        self.workspace = Some(scanned);
        true
    }

    pub fn open_path(&mut self, path: &Path) -> Result<(), AppOpenError> {
        if path.is_dir() {
            self.open_workspace(path)
        } else {
            self.open_or_activate(path).map(|_| ())
        }
    }

    pub fn open_paths<I, P>(&mut self, paths: I) -> BatchOpenResult
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut result = BatchOpenResult::default();
        let mut workspace_attempted = false;
        for path in paths {
            let path = path.as_ref();
            if path.is_dir() {
                workspace_attempted = true;
                if let Err(AppOpenError::Workspace(error)) = self.open_workspace(path)
                    && result.workspace_error.is_none()
                {
                    result.workspace_error = Some(error);
                }
            } else {
                result.document_attempted = true;
                match self.open_or_activate(path) {
                    Ok(_) => result.document_opened = true,
                    Err(AppOpenError::Document(error)) if result.document_error.is_none() => {
                        result.document_error = Some(error);
                    }
                    Err(AppOpenError::Document(_)) => {}
                    Err(AppOpenError::Workspace(_)) => unreachable!(),
                }
            }
        }
        if workspace_attempted {
            self.workspace_error = result.workspace_error.clone();
        }
        result
    }

    pub fn close_tab(&mut self, path: &Path) -> bool {
        let closed = self.tabs.close(path);
        if let Some(tab) = closed.as_ref() {
            self.deleted.remove(tab.path());
        }
        closed.is_some()
    }

    pub fn is_deleted(&self, path: &Path) -> bool {
        self.deleted.contains(path)
    }

    pub fn dismiss_active_reload_error(&mut self) -> bool {
        let Some(tab) = self.tabs.active().filter(|tab| tab.reload_error.is_some()) else {
            return false;
        };
        self.tabs.replace_prepared((*tab.document).clone())
    }
}

pub const DEFAULT_WINDOW_TITLE: &str = "Mdow Native";

/// Mirrors the Electron shell: the active document's file name, or the app name when empty.
pub fn window_title_for(active_path: Option<&Path>) -> String {
    active_path
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| DEFAULT_WINDOW_TITLE.into())
}

fn canonical_file_identity(path: &Path) -> PathBuf {
    crate::session::file_identity(path)
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DropState {
    active: bool,
    summary: DropSummary,
}

impl DropState {
    pub fn is_active(self) -> bool {
        self.active
    }

    pub fn summary(self) -> DropSummary {
        self.summary
    }

    /// Starts a drag, remembering what it carries; returns whether the drag just began.
    pub fn enter_with(&mut self, summary: DropSummary) -> bool {
        self.summary = summary;
        self.enter()
    }

    pub fn enter(&mut self) -> bool {
        self.set_active(true)
    }

    pub fn leave(&mut self) -> bool {
        self.set_active(false)
    }

    pub fn dropped(&mut self) -> bool {
        self.leave()
    }

    fn set_active(&mut self, active: bool) -> bool {
        let changed = self.active != active;
        self.active = active;
        changed
    }
}

/// What a context-menu entry does once confirmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextAction {
    Open(PathBuf),
    CloseTab(PathBuf),
    CloseOtherTabs(PathBuf),
    CloseTabsToRight(PathBuf),
    CloseAllTabs,
    OpenInPane(PathBuf, PaneId),
    CopyPath(PathBuf),
    Reveal(PathBuf),
    RemoveRecent(PathBuf),
    /// Reader context menu: copy the selection of the document's reader.
    CopySelection(PathBuf),
    SelectAll(PathBuf),
}

/// Menu entries paired with the action each confirms (`None` for separators).
pub type ContextMenuSpec = Vec<(ContextMenuEntry, Option<ContextAction>)>;

fn menu_item(
    label: &'static str,
    action: ContextAction,
) -> (ContextMenuEntry, Option<ContextAction>) {
    (ContextMenuEntry::item(label), Some(action))
}

fn menu_item_if(
    enabled: bool,
    label: &'static str,
    action: ContextAction,
) -> (ContextMenuEntry, Option<ContextAction>) {
    if enabled {
        menu_item(label, action)
    } else {
        (ContextMenuEntry::disabled(label), None)
    }
}

fn menu_separator() -> (ContextMenuEntry, Option<ContextAction>) {
    (ContextMenuEntry::Separator, None)
}

pub fn tab_context_menu(path: &Path, tab_paths: &[PathBuf]) -> ContextMenuSpec {
    let index = tab_paths.iter().position(|tab| tab == path);
    let has_others = tab_paths.len() > 1;
    let has_right = index.is_some_and(|index| index + 1 < tab_paths.len());
    vec![
        menu_item("Close", ContextAction::CloseTab(path.to_owned())),
        menu_item_if(
            has_others,
            "Close Others",
            ContextAction::CloseOtherTabs(path.to_owned()),
        ),
        menu_item_if(
            has_right,
            "Close to the Right",
            ContextAction::CloseTabsToRight(path.to_owned()),
        ),
        menu_item("Close All", ContextAction::CloseAllTabs),
        menu_separator(),
        menu_item(
            "Open in Left Pane",
            ContextAction::OpenInPane(path.to_owned(), PaneId::Primary),
        ),
        menu_item(
            "Open in Right Pane",
            ContextAction::OpenInPane(path.to_owned(), PaneId::Secondary),
        ),
        menu_separator(),
        menu_item("Copy Path", ContextAction::CopyPath(path.to_owned())),
        menu_item("Reveal in Finder", ContextAction::Reveal(path.to_owned())),
    ]
}

pub fn tree_context_menu(path: &Path, directory: bool) -> ContextMenuSpec {
    let mut menu = Vec::new();
    if !directory {
        menu.push(menu_item("Open", ContextAction::Open(path.to_owned())));
        menu.push(menu_separator());
    }
    menu.push(menu_item(
        "Copy Path",
        ContextAction::CopyPath(path.to_owned()),
    ));
    menu.push(menu_item(
        "Reveal in Finder",
        ContextAction::Reveal(path.to_owned()),
    ));
    menu
}

pub fn recent_context_menu(path: &Path) -> ContextMenuSpec {
    vec![
        menu_item("Open", ContextAction::Open(path.to_owned())),
        menu_separator(),
        menu_item("Copy Path", ContextAction::CopyPath(path.to_owned())),
        menu_item("Reveal in Finder", ContextAction::Reveal(path.to_owned())),
        menu_separator(),
        menu_item(
            "Remove from Recents",
            ContextAction::RemoveRecent(path.to_owned()),
        ),
    ]
}

struct OpenContextMenu {
    view: Entity<ContextMenu>,
    actions: Vec<Option<ContextAction>>,
    position: Point<Pixels>,
    _events: Subscription,
}

pub struct MdowApp {
    pub model: AppModel,
    pub sidebar_open: bool,
    pub wide_mode: bool,
    /// Side-by-side panes; the focused pane always shows the active tab.
    pub split: SplitState,
    /// True while the split divider is being dragged, so it stays highlighted.
    divider_dragging: bool,
    prefs: StoredPrefs,
    overlays: OverlayHost,
    last_window_bounds: Option<SavedWindowBounds>,
    pub drop_state: DropState,
    pub open_error: Option<UserFacingError>,
    copied_code: Option<(usize, Instant)>,
    hovered_link: Option<LinkFocusKey>,
    focused_link: Option<LinkFocusKey>,
    reader_panes: HashMap<PathBuf, Entity<ReaderPane>>,
    reader_link_focus_handles: HashMap<(PathBuf, LinkFocusKey), FocusHandle>,
    /// `None` when the platform watcher could not start; documents then open without live reload.
    file_watcher: Option<FileWatcher>,
    _watch_poll_task: Option<Task<()>>,
    workspace_refresh: Option<Task<()>>,
    /// Off-thread document reads and parses, one per tab; replacing one cancels the old.
    load_tasks: HashMap<PathBuf, Task<()>>,
    /// Saved reading positions waiting for their document to finish loading.
    restore_anchors: HashMap<PathBuf, ScrollAnchor>,
    /// The latest reading position of each tab the reader has shown, for the session.
    anchors: HashMap<PathBuf, ScrollAnchor>,
    session_save: Option<Task<()>>,
    folder_filter: Entity<Field>,
    _folder_filter_events: Subscription,
    /// Folders the reader collapsed while a filter is active; reset when the query changes.
    filter_collapsed: HashSet<PathBuf>,
    context_menu: Option<OpenContextMenu>,
    outline_scroll: gpui::UniformListScrollHandle,
    last_outline_active: Option<usize>,
    tab_focus: HashMap<PathBuf, TabFocus>,
    menu_recents: Option<Vec<PathBuf>>,
    update: UpdateUi,
    update_dismissed: bool,
    _update_poll_task: Task<()>,
    zoom_hud: ZoomHud,
    cheat_sheet: CheatSheetHold,
    _activation_subscription: Subscription,
    theme: Theme,
    window_title: Option<String>,
    /// The AI companion panel (see `companion::ui`).
    pub(crate) companion: CompanionHost,
    focus_handle: FocusHandle,
    _appearance_subscription: Subscription,
}

pub(crate) struct ReaderPaintState {
    pub copied_code: Option<(usize, Instant)>,
    pub hovered_link: Option<LinkFocusKey>,
    pub focused_link: Option<LinkFocusKey>,
    pub find_hits: Option<Arc<[FindHit]>>,
    pub find_active: Option<FindHit>,
}

/// Space pages like a browser: Shift+Space goes back up.
fn reader_scroll_key(key: &str, shift: bool) -> &str {
    match (key, shift) {
        ("space", false) => "pagedown",
        ("space", true) => "pageup",
        _ => key,
    }
}

fn reader_key_modifiers_are_allowed(
    key: &str,
    control: bool,
    alt: bool,
    platform: bool,
    function: bool,
) -> bool {
    !control
        && !alt
        && !platform
        && (!function || matches!(key, "home" | "end" | "pageup" | "pagedown"))
}

impl MdowApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::boot(
            Prefs::default(),
            StateStore::in_memory(),
            SessionRole::Owner,
            window,
            cx,
        )
    }

    pub fn boot(
        prefs: Prefs,
        store: StateStore,
        role: SessionRole,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::boot_with_watcher(prefs, store, role, FileWatcher::new(), window, cx)
    }

    pub(crate) fn boot_with_watcher(
        prefs: Prefs,
        store: StateStore,
        role: SessionRole,
        file_watcher: anyhow::Result<FileWatcher>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        let folder_filter = cx.new(|cx| {
            let mut field = Field::search("Filter files", window, cx);
            field.set_tab_stop();
            field
        });
        let folder_filter_events = cx.subscribe_in(&folder_filter, window, Self::on_filter_event);
        focus_handle.focus(window);
        let activation_subscription = cx.observe_window_activation(window, |this, window, cx| {
            if !window.is_window_active() && this.cheat_sheet.reset() {
                cx.notify();
            }
        });
        let appearance_subscription = cx.observe_window_appearance(window, |this, window, cx| {
            this.theme = this.resolve_theme(window);
            cx.notify();
        });
        let file_watcher = file_watcher
            .inspect_err(|error| eprintln!("Mdow: live reload is unavailable: {error:#}"))
            .ok();
        let watch_poll_task = file_watcher
            .as_ref()
            .map(|watcher| Self::spawn_watch_poll(watcher.messages(), cx));

        let wide_mode = prefs.reader_width.is_full();
        let update_poll_task = cx.spawn(async move |this, cx| {
            Timer::after(Duration::from_secs(sparkle::LAUNCH_CHECK_DELAY_SECS)).await;
            let _ = this.update(cx, |this, _| {
                sparkle::set_automatic_checks(this.prefs.get().auto_update);
                sparkle::start();
            });
            loop {
                Timer::after(Duration::from_millis(200)).await;
                if this
                    .update(cx, |this, cx| {
                        this.poll_sparkle(cx);
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            model: AppModel::default(),
            sidebar_open: true,
            wide_mode,
            split: SplitState::default(),
            divider_dragging: false,
            prefs: StoredPrefs::restore(prefs, store, role),
            overlays: OverlayHost::default(),
            last_window_bounds: None,
            drop_state: DropState::default(),
            open_error: None,
            copied_code: None,
            hovered_link: None,
            focused_link: None,
            reader_panes: HashMap::new(),
            reader_link_focus_handles: HashMap::new(),
            file_watcher,
            _watch_poll_task: watch_poll_task,
            workspace_refresh: None,
            load_tasks: HashMap::new(),
            restore_anchors: HashMap::new(),
            anchors: HashMap::new(),
            session_save: None,
            folder_filter,
            _folder_filter_events: folder_filter_events,
            filter_collapsed: HashSet::new(),
            context_menu: None,
            outline_scroll: gpui::UniformListScrollHandle::new(),
            last_outline_active: None,
            tab_focus: HashMap::new(),
            menu_recents: None,
            update: UpdateUi::default(),
            update_dismissed: false,
            _update_poll_task: update_poll_task,
            zoom_hud: ZoomHud::default(),
            cheat_sheet: CheatSheetHold::default(),
            _activation_subscription: activation_subscription,
            theme: Theme::for_appearance(window.appearance()),
            window_title: None,
            companion: CompanionHost::default(),
            focus_handle,
            _appearance_subscription: appearance_subscription,
        }
    }

    /// The palette for the theme preference and window appearance, sized for the interface scale.
    fn resolve_theme(&self, window: &Window) -> Theme {
        let prefs = self.prefs.get();
        Theme::resolve(prefs.theme_mode, window.appearance()).scaled(prefs.interface_scale)
    }

    fn spawn_watch_poll(
        poll_messages: Arc<Mutex<Receiver<WatchMessage>>>,
        cx: &mut Context<Self>,
    ) -> Task<()> {
        cx.spawn(async move |this, cx| {
            loop {
                Timer::after(Duration::from_millis(100)).await;
                let messages = {
                    let Ok(receiver) = poll_messages.lock() else {
                        break;
                    };
                    receiver.try_iter().collect::<Vec<_>>()
                };
                if messages.is_empty() {
                    continue;
                }
                if this
                    .update(cx, |this, cx| {
                        this.handle_watch_messages(messages, cx);
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
    }

    fn handle_watch_messages(&mut self, messages: Vec<WatchMessage>, cx: &mut Context<Self>) {
        let mut changed = false;
        for message in messages {
            match message {
                WatchMessage::Reload(path) => {
                    if self.model.tabs.get(&path).is_some() {
                        crate::perf::mark("reload_start");
                        if let Ok(Some(load)) = self.model.begin_reload(&path) {
                            self.spawn_load(load, cx);
                        } else {
                            crate::perf::mark("reload_applied");
                        }
                        changed = true;
                    }
                }
                WatchMessage::FolderChanged(root) => self.refresh_workspace(&root, cx),
            }
        }
        if changed {
            cx.notify();
        }
    }

    /// Rescans the open folder off the main thread and swaps in the result, keeping the
    /// reader's expanded folders (the filter lives outside the tree, so it survives too).
    pub(crate) fn refresh_workspace(&mut self, root: &Path, cx: &mut Context<Self>) {
        if self
            .model
            .workspace
            .as_ref()
            .is_none_or(|tree| tree.root.path != root)
        {
            return;
        }
        let root = root.to_owned();
        self.workspace_refresh = Some(cx.spawn(async move |this, cx| {
            let scan_root = root.clone();
            let scanned = cx
                .background_spawn(async move { scan_workspace(&scan_root) })
                .await;
            this.update(cx, |this, cx| {
                if this.model.apply_workspace_rescan(&root, scanned) {
                    cx.notify();
                }
            })
            .ok();
        }));
    }

    /// Reads and parses a large document off the UI thread: a preview first when asked for,
    /// then the whole document. Results for a tab that was closed, reopened or reloaded again
    /// meanwhile are dropped.
    pub(crate) fn spawn_load(&mut self, load: PendingLoad, cx: &mut Context<Self>) {
        let path = load.path.clone();
        // The read and parse start on a background thread right away rather than from a
        // foreground task, which would wait for the window's first frames to finish.
        let (sender, receiver) = async_channel::bounded(2);
        let job = load.clone();
        let background = cx.background_spawn(async move {
            match read_for_load(&job) {
                Err(error) => {
                    sender.send(LoadEvent::Failed(error)).await.ok();
                }
                Ok((loaded, preview)) => {
                    if let Some(preview) = preview
                        && sender.send(LoadEvent::Preview(preview)).await.is_err()
                    {
                        return;
                    }
                    sender
                        .send(LoadEvent::Full(parse_for_load(loaded)))
                        .await
                        .ok();
                }
            }
        });
        let task = cx.spawn(async move |this, cx| {
            let _background = background;
            while let Ok(event) = receiver.recv().await {
                let keep_going = this
                    .update(cx, |this, cx| match event {
                        LoadEvent::Preview(preview) => this.finish_load(&load, preview, cx),
                        LoadEvent::Full(document) => {
                            this.finish_load(&load, document, cx);
                            this.load_tasks.remove(&load.path);
                            false
                        }
                        LoadEvent::Failed(error) => {
                            if let Some(error) = this.model.fail_load(&load, error)
                                && !load.reload
                            {
                                this.open_error = Some(error.into_view());
                                this.active_document_changed(cx);
                            }
                            this.load_tasks.remove(&load.path);
                            cx.notify();
                            false
                        }
                    })
                    .unwrap_or(false);
                if !keep_going {
                    break;
                }
            }
        });
        self.load_tasks.insert(path, task);
    }

    fn finish_load(
        &mut self,
        load: &PendingLoad,
        document: PreparedDocument,
        cx: &mut Context<Self>,
    ) -> bool {
        let partial = document.is_partial();
        if !self.model.finish_load(load, document) {
            return false;
        }
        if !partial {
            crate::perf::mark(if load.reload {
                "reload_applied"
            } else {
                "full_ready"
            });
        }
        if self
            .model
            .tabs
            .active()
            .is_some_and(|tab| tab.path() == load.path)
        {
            let document = self.model.tabs.active().map(|tab| tab.document.clone());
            self.overlays.retarget_find(document, cx);
        }
        cx.notify();
        true
    }

    /// Scans a folder off the UI thread, then shows it (the tree only caps its size, so a huge
    /// folder could otherwise stall the window while it opens).
    fn open_workspace_async(&mut self, path: &Path, cx: &mut Context<Self>) {
        let root = path.to_owned();
        self.workspace_refresh = Some(cx.spawn(async move |this, cx| {
            let scan_root = root.clone();
            let scanned = cx
                .background_spawn(async move { scan_workspace(&scan_root) })
                .await;
            this.update(cx, |this, cx| {
                this.model.apply_workspace_scan(scanned).ok();
                this.sync_folder_watch();
                this.persist_session();
                cx.notify();
            })
            .ok();
        }));
    }

    pub fn open_path(&mut self, path: &Path, cx: &mut Context<Self>) {
        crate::perf::mark("open_start");
        if path.is_dir() {
            self.open_workspace_async(path, cx);
            cx.notify();
            return;
        }
        match self.model.begin_open(path, true) {
            Ok(OpenStart::ActivatedExisting) => {
                self.open_error = None;
                self.active_document_changed(cx);
            }
            Ok(OpenStart::LoadedFromDisk) => {
                crate::perf::mark("full_ready");
                let watch_error = self
                    .model
                    .tabs
                    .active()
                    .map(|tab| tab.path().to_owned())
                    .and_then(|path| self.watch_document(&path).err());
                self.open_error = watch_error;
                self.active_document_changed(cx);
            }
            Ok(OpenStart::Pending(load)) => {
                self.open_error = self.watch_document(&load.path).err();
                self.spawn_load(load, cx);
                self.active_document_changed(cx);
            }
            Err(AppOpenError::Document(error)) => self.open_error = Some(error),
            Err(AppOpenError::Workspace(_)) => {}
        }
        cx.notify();
    }

    pub fn open_paths(&mut self, paths: impl IntoIterator<Item = PathBuf>, cx: &mut Context<Self>) {
        let mut document_attempted = false;
        let mut document_opened = false;
        let mut document_error = None;
        let mut folder = None;
        for path in paths {
            if path.is_dir() {
                folder = Some(path);
                continue;
            }
            document_attempted = true;
            match self.model.begin_open(&path, true) {
                Ok(OpenStart::Pending(load)) => {
                    document_opened = true;
                    self.spawn_load(load, cx);
                }
                Ok(_) => document_opened = true,
                Err(AppOpenError::Document(error)) => {
                    document_error.get_or_insert(error);
                }
                Err(AppOpenError::Workspace(_)) => {}
            }
        }
        if let Some(folder) = folder {
            self.open_workspace_async(&folder, cx);
        }
        let watch_error = document_opened
            .then(|| self.watch_all_documents())
            .flatten();
        if document_attempted {
            self.open_error = document_error.or(watch_error);
        }
        if document_opened {
            self.active_document_changed(cx);
        }
        self.drop_state.dropped();
        cx.notify();
    }

    /// Keeps the recursive folder watch pointed at whichever folder the sidebar shows.
    fn sync_folder_watch(&mut self) {
        let root = self
            .model
            .workspace
            .as_ref()
            .map(|tree| tree.root.path.clone());
        let Some(watcher) = self.file_watcher.as_mut() else {
            return;
        };
        match root {
            Some(root) => {
                if let Err(error) = watcher.watch_folder(&root) {
                    eprintln!("Mdow: folder watching is unavailable: {error:#}");
                }
            }
            None => watcher.unwatch_folder(),
        }
    }

    fn watch_all_documents(&mut self) -> Option<UserFacingError> {
        let paths = self
            .model
            .tabs
            .paths()
            .map(Path::to_owned)
            .collect::<Vec<_>>();
        let mut first_error = None;
        for path in paths {
            if let Err(error) = self.watch_document(&path)
                && first_error.is_none()
            {
                first_error = Some(error);
            }
        }
        first_error
    }

    fn watch_document(&mut self, path: &Path) -> Result<(), UserFacingError> {
        let Some(file_watcher) = self.file_watcher.as_mut() else {
            return Ok(());
        };
        file_watcher.watch(path).map_err(|error| UserFacingError {
            title: "Couldn't watch this file".into(),
            body: error.to_string(),
            path: path.to_owned(),
        })
    }

    fn open_workspace_path(&mut self, path: &Path, cx: &mut Context<Self>) {
        self.open_workspace_async(path, cx);
        self.apply_pref(PrefEdit::Sidebar(SidebarMode::Folder), cx);
        cx.notify();
    }

    fn drag_moved(&mut self, summary: DropSummary, window: &mut Window, cx: &mut Context<Self>) {
        if !self.drop_state.enter_with(summary) {
            return;
        }
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            loop {
                Timer::after(Duration::from_millis(16)).await;
                let drag_is_active = cx.update(|_, cx| cx.has_active_drag()).unwrap_or(false);
                if !drag_is_active {
                    this.update(cx, |this, cx| {
                        if this.drop_state.leave() {
                            cx.notify();
                        }
                    })
                    .ok();
                    break;
                }
            }
        })
        .detach();
    }

    pub fn open_file_prompt(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Open".into()),
        });
        cx.spawn(async move |this, cx| match receiver.await {
            Ok(Ok(Some(paths))) => {
                this.update(cx, |this, cx| this.open_paths(paths, cx)).ok();
            }
            Ok(Ok(None)) => {}
            Ok(Err(_)) | Err(_) => {
                this.update(cx, |this, cx| {
                    this.open_error = Some(UserFacingError {
                        title: "Couldn't open file picker".into(),
                        body: "The system file picker could not be opened. Try again.".into(),
                        path: PathBuf::new(),
                    });
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    pub fn open_folder_prompt(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Open Folder".into()),
        });
        cx.spawn(async move |this, cx| match receiver.await {
            Ok(Ok(Some(paths))) => {
                this.update(cx, |this, cx| {
                    if let Some(path) = paths.first() {
                        this.open_workspace_path(path, cx);
                    }
                })
                .ok();
            }
            Ok(Ok(None)) => {}
            Ok(Err(_)) | Err(_) => {
                this.update(cx, |this, cx| {
                    this.model.workspace_error = Some(UserFacingError {
                        title: "Couldn't open folder picker".into(),
                        body: "The system folder picker could not be opened. Try again.".into(),
                        path: PathBuf::new(),
                    });
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    fn open_file(&mut self, _: &OpenFile, _: &mut Window, cx: &mut Context<Self>) {
        self.open_file_prompt(cx);
    }

    fn open_folder(&mut self, _: &OpenFolder, _: &mut Window, cx: &mut Context<Self>) {
        self.open_folder_prompt(cx);
    }

    fn toggle_sidebar(&mut self, _: &ToggleSidebar, _: &mut Window, cx: &mut Context<Self>) {
        self.click_toggle_sidebar(cx);
    }

    fn toggle_wide_mode(&mut self, _: &ToggleWideMode, _: &mut Window, cx: &mut Context<Self>) {
        self.click_toggle_wide_mode(cx);
    }

    fn on_toggle_split_view(
        &mut self,
        _: &ToggleSplitView,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_split_view(cx);
    }

    /// Shows `path` in the focused reader after a split change, keeping find, the outline and
    /// the session pointed at it.
    fn show_split_target(&mut self, target: Option<PathBuf>, cx: &mut Context<Self>) {
        if let Some(target) = target {
            self.model.tabs.activate(&target);
        }
        self.active_document_changed(cx);
        cx.notify();
    }

    pub(crate) fn toggle_split_view(&mut self, cx: &mut Context<Self>) {
        if self.model.tabs.is_empty() {
            return;
        }
        let tabs = self.tab_paths();
        let active = self.model.tabs.active().map(|tab| tab.path().to_owned());
        let target = self.split.toggle(&tabs, active.as_deref());
        self.show_split_target(target, cx);
    }

    /// Clicking into a pane makes it the one find, the outline, zoom feedback and keyboard
    /// scrolling act on.
    pub(crate) fn focus_pane(&mut self, pane: PaneId, cx: &mut Context<Self>) {
        if !self.split.is_enabled() || self.split.active_pane() == pane {
            return;
        }
        let target = self.split.set_active_pane(pane);
        self.show_split_target(target, cx);
    }

    pub(crate) fn open_in_pane(&mut self, path: &Path, pane: PaneId, cx: &mut Context<Self>) {
        let tabs = self.tab_paths();
        let active = self.model.tabs.active().map(|tab| tab.path().to_owned());
        let target = self
            .split
            .set_pane_tab(pane, path, &tabs, active.as_deref());
        if target.is_some() {
            self.show_split_target(target, cx);
        }
    }

    pub(crate) fn drag_split_divider(&mut self, x: f32, width: f32, cx: &mut Context<Self>) {
        self.divider_dragging = true;
        if self.split.drag_divider_to(x, width) {
            cx.notify();
        }
    }

    pub(crate) fn end_split_divider_drag(&mut self, cx: &mut Context<Self>) {
        self.divider_dragging = false;
        self.persist_session();
        cx.notify();
    }

    pub(crate) fn reset_split_divider(&mut self, cx: &mut Context<Self>) {
        if self.split.reset_ratio() {
            self.persist_session();
            cx.notify();
        }
    }

    pub(crate) fn click_toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_open = !self.sidebar_open;
        cx.notify();
    }

    pub(crate) fn click_toggle_wide_mode(&mut self, cx: &mut Context<Self>) {
        self.apply_pref(PrefEdit::ToggleFull, cx);
    }

    pub(crate) fn click_toggle_overlay(
        &mut self,
        kind: OverlayKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_overlay(kind, window, cx);
    }

    #[cfg(test)]
    pub(crate) fn overlay_kind(&self) -> Option<OverlayKind> {
        self.overlays.kind()
    }

    #[cfg(test)]
    pub(crate) fn prefs_snapshot(&self) -> Prefs {
        *self.prefs.get()
    }

    pub(crate) fn reveal_path(&self, path: &Path) {
        let _ = std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn();
    }

    pub fn restore_session(&mut self, session: Session, cx: &mut Context<Self>) {
        self.model.recents = session.recents.clone();
        if let Some(folder) = session.last_folder.as_ref() {
            self.open_workspace_async(folder, cx);
        }
        if let Some(tabs) = session.tabs.as_ref() {
            for path in tabs.iter() {
                let anchor = session
                    .anchors
                    .get(path)
                    .copied()
                    .filter(|anchor| !anchor.is_top());
                // A saved reading position needs the whole document, so only fresh tabs preview.
                match self.model.begin_open(path, anchor.is_none()) {
                    Ok(OpenStart::Pending(load)) => {
                        if let Some(anchor) = anchor {
                            self.restore_anchors.insert(load.path.clone(), anchor);
                        }
                        self.spawn_load(load, cx);
                    }
                    Ok(_) => {
                        if let (Some(anchor), Some(tab)) = (anchor, self.model.tabs.active()) {
                            self.restore_anchors.insert(tab.path().to_owned(), anchor);
                        }
                    }
                    Err(_) => {}
                }
            }
            self.model.tabs.activate(tabs.active());
            let _ = self.watch_all_documents();
            let open = self.tab_paths();
            if let Some(target) = session
                .split
                .as_ref()
                .and_then(|saved| self.split.restore(saved, &open))
            {
                self.model.tabs.activate(&target);
            } else if let Some(active) = self.model.tabs.active().map(|tab| tab.path().to_owned()) {
                self.split.activated(&active, &open);
            }
        }
        self.last_window_bounds = session.window;
        self.clear_reader_transient_state();
        cx.notify();
    }

    // --- Companion hooks (the panel itself lives in `companion::ui`) ---
    pub(crate) fn companion_prefs(&self) -> crate::prefs::CompanionPrefs {
        self.prefs.get().companion
    }

    pub(crate) fn companion_state_path(&self) -> PathBuf {
        self.prefs.state_path().to_owned()
    }

    pub(crate) fn companion_pref(&mut self, edit: PrefEdit, cx: &mut Context<Self>) {
        self.apply_pref(edit, cx);
    }
    // --- end Companion hooks ---

    fn apply_pref(&mut self, edit: PrefEdit, cx: &mut Context<Self>) {
        let session = self.session_snapshot();
        let auto_update = self.prefs.get().auto_update;
        if !self.prefs.apply(edit, &session) {
            return;
        }
        if self.prefs.get().auto_update != auto_update {
            sparkle::set_automatic_checks(self.prefs.get().auto_update);
        }
        self.wide_mode = self.prefs.get().reader_width.is_full();
        self.overlays
            .refresh_settings(self.prefs.get(), self.update.clone(), cx);
        cx.notify();
    }

    fn poll_sparkle(&mut self, cx: &mut Context<Self>) {
        let next = sparkle::current_ui();
        if self.update == next {
            return;
        }
        if next.resets_dismissed() {
            self.update_dismissed = false;
        }
        self.update = next;
        self.overlays
            .refresh_settings(self.prefs.get(), self.update.clone(), cx);
        cx.notify();
    }

    fn check_for_updates(&mut self, cx: &mut Context<Self>) {
        self.update_dismissed = false;
        self.update = self
            .update
            .apply(sparkle::UpdateEvent::Checking { manual: true });
        sparkle::check(true);
        self.overlays
            .refresh_settings(self.prefs.get(), self.update.clone(), cx);
        cx.notify();
    }

    pub(crate) fn download_update(&mut self, cx: &mut Context<Self>) {
        if !self.update.can_download() {
            return;
        }
        sparkle::download();
        cx.notify();
    }

    pub(crate) fn install_update(&mut self, cx: &mut Context<Self>) {
        if !self.update.can_install() {
            return;
        }
        sparkle::install();
        cx.notify();
    }

    pub(crate) fn dismiss_update_banner(&mut self, cx: &mut Context<Self>) {
        self.update_dismissed = true;
        sparkle::dismiss_choice();
        cx.notify();
    }

    fn on_check_for_updates(
        &mut self,
        _: &CheckForUpdates,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.check_for_updates(cx);
    }

    fn toggle_overlay(&mut self, kind: OverlayKind, window: &mut Window, cx: &mut Context<Self>) {
        if self.overlays.kind() == Some(kind) {
            self.overlays.close(Some(window));
            cx.notify();
            return;
        }
        let overlay = match kind {
            OverlayKind::Find => {
                let document = self.model.tabs.active().map(|tab| tab.document.clone());
                let theme_mode = self.prefs.get().theme_mode;
                let view = cx.new(|cx| FindOverlay::new(document, theme_mode, window, cx));
                let events = cx.subscribe_in(&view, window, |this, _, event, _, cx| {
                    this.on_find_event(event, cx);
                });
                OpenOverlay::find(view, events)
            }
            OverlayKind::Palette => {
                let workspace_files = self
                    .model
                    .workspace
                    .as_ref()
                    .map(WorkspaceTree::files)
                    .unwrap_or_default();
                let view = cx.new(|cx| {
                    PaletteOverlay::new(
                        self.model.recents.clone(),
                        workspace_files,
                        self.prefs.get().theme_mode,
                        window,
                        cx,
                    )
                });
                let events = cx.subscribe_in(&view, window, |this, _, event, window, cx| {
                    this.on_palette_event(event, window, cx);
                });
                OpenOverlay::palette(view, events)
            }
            OverlayKind::Settings => {
                let view = cx.new(|cx| {
                    SettingsPanel::new(*self.prefs.get(), self.update.clone(), window, cx)
                });
                let events = cx.subscribe_in(&view, window, |this, _, event, window, cx| {
                    if matches!(event, SettingsEvent::ChooseCompanionExecutable) {
                        this.choose_companion_executable(window, cx);
                    } else {
                        this.on_settings_event(event, cx);
                    }
                });
                OpenOverlay::settings(view, events)
            }
            OverlayKind::Shortcuts => {
                let theme_mode = self.prefs.get().theme_mode;
                let view = cx.new(|cx| ShortcutsCard::new(theme_mode, window, cx));
                let events = cx.subscribe_in(&view, window, |this, _, event, _, cx| {
                    this.on_shortcuts_event(event, cx);
                });
                OpenOverlay::shortcuts(view, events)
            }
        };
        self.overlays.open(overlay, self.focus_handle.clone());
        cx.notify();
    }

    fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.close_context_menu(window, cx) {
            return;
        }
        if self.overlays.close(Some(window)) {
            cx.notify();
            return;
        }
        if let Some(pane) = self.active_reader_pane()
            && pane.update(cx, |pane, cx| pane.clear_selection(cx))
        {
            return;
        }
        if self.model.dismiss_active_reload_error() {
            cx.notify();
        }
    }

    fn run_command(&mut self, id: CommandId, window: &mut Window, cx: &mut Context<Self>) {
        match id {
            CommandId::OpenFile => self.open_file_prompt(cx),
            CommandId::OpenFolder => self.open_folder_prompt(cx),
            CommandId::CloseTab => self.close_active_tab(&CloseTab, window, cx),
            CommandId::NextTab => self.cycle_tab(1, cx),
            CommandId::PreviousTab => self.cycle_tab(-1, cx),
            CommandId::ToggleSidebar => self.toggle_sidebar(&ToggleSidebar, window, cx),
            CommandId::SidebarRecents => {
                self.apply_pref(PrefEdit::Sidebar(SidebarMode::Recents), cx)
            }
            CommandId::SidebarFolder => self.apply_pref(PrefEdit::Sidebar(SidebarMode::Folder), cx),
            CommandId::SidebarOutline => {
                self.apply_pref(PrefEdit::Sidebar(SidebarMode::Outline), cx)
            }
            CommandId::ToggleWideMode => self.apply_pref(PrefEdit::ToggleFull, cx),
            CommandId::ToggleSplitView => self.toggle_split_view(cx),
            CommandId::LineWidth(width) => self.apply_pref(PrefEdit::LineWidth(width), cx),
            CommandId::ThemeSystem => self.apply_pref(PrefEdit::Theme(ThemeMode::System), cx),
            CommandId::ThemeLight => self.apply_pref(PrefEdit::Theme(ThemeMode::Light), cx),
            CommandId::ThemeDark => self.apply_pref(PrefEdit::Theme(ThemeMode::Dark), cx),
            CommandId::ZoomIn => self.zoom_with_feedback(PrefEdit::ZoomIn, cx),
            CommandId::ZoomOut => self.zoom_with_feedback(PrefEdit::ZoomOut, cx),
            CommandId::ZoomReset => self.zoom_with_feedback(PrefEdit::ZoomReset, cx),
            CommandId::FindInDocument => self.toggle_overlay(OverlayKind::Find, window, cx),
            CommandId::OpenSettings => self.toggle_overlay(OverlayKind::Settings, window, cx),
            CommandId::OpenShortcuts => self.toggle_overlay(OverlayKind::Shortcuts, window, cx),
            CommandId::CheckForUpdates => self.check_for_updates(cx),
            CommandId::ToggleCompanion => self.toggle_companion(window, cx),
        }
    }

    fn active_document_changed(&mut self, cx: &mut Context<Self>) {
        if let Some(active) = self.model.tabs.active().map(|tab| tab.path().to_owned()) {
            let tabs = self.tab_paths();
            self.split.activated(&active, &tabs);
        }
        self.clear_reader_transient_state();
        let document = self.model.tabs.active().map(|tab| tab.document.clone());
        self.overlays.retarget_find(document, cx);
        self.prefs.save_session(&self.session_snapshot());
    }

    fn session_snapshot(&self) -> Session {
        Session::from_parts(
            self.model.tabs.paths().map(Path::to_owned),
            self.model.tabs.active().map(|tab| tab.path().to_owned()),
            self.model
                .workspace
                .as_ref()
                .map(|tree| tree.root.path.clone()),
            self.model.recents.clone(),
            self.last_window_bounds,
        )
        .with_anchors(
            self.model
                .tabs
                .paths()
                .filter_map(|path| {
                    let anchor = self
                        .anchors
                        .get(path)
                        .or_else(|| self.restore_anchors.get(path))?;
                    Some((path.to_owned(), *anchor))
                })
                .collect(),
        )
        .with_split(self.split.session())
    }

    fn scroll_reader_to_block(&mut self, block: usize, cx: &mut Context<Self>) {
        let Some(path) = self.model.tabs.active().map(|tab| tab.path().to_owned()) else {
            return;
        };
        let Some(pane) = self.reader_panes.get(&path).cloned() else {
            return;
        };
        pane.update(cx, |pane, cx| {
            pane.scroll_to_block(block);
            cx.notify();
        });
    }

    fn on_find_event(&mut self, event: &FindEvent, cx: &mut Context<Self>) {
        match event {
            FindEvent::ActiveHit(hit) => {
                if let Some(pane) = self.active_reader_pane() {
                    pane.update(cx, |pane, cx| {
                        pane.reveal_find_hit(*hit);
                        cx.notify();
                    });
                }
            }
            FindEvent::Dismissed => {
                self.overlays.close(None);
                cx.notify();
            }
        }
    }

    fn on_palette_event(
        &mut self,
        event: &PaletteEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            PaletteEvent::Invoked(action) => {
                self.overlays.close(Some(window));
                match action {
                    PaletteAction::Run(id) => self.run_command(*id, window, cx),
                    PaletteAction::Open(path) => self.open_path(path, cx),
                }
            }
            PaletteEvent::Dismissed => {
                self.overlays.close(Some(window));
                cx.notify();
            }
        }
    }

    fn on_settings_event(&mut self, event: &SettingsEvent, cx: &mut Context<Self>) {
        match event {
            SettingsEvent::Edited(edit) => self.apply_pref(*edit, cx),
            SettingsEvent::CheckForUpdates => self.check_for_updates(cx),
            SettingsEvent::DownloadUpdate => self.download_update(cx),
            SettingsEvent::ViewReleases => {
                let _ = open::that(sparkle::RELEASES_URL);
            }
            SettingsEvent::InstallUpdate => self.install_update(cx),
            SettingsEvent::ChooseCompanionExecutable => {}
            SettingsEvent::Dismissed => {
                self.overlays.close(None);
                cx.notify();
            }
        }
    }

    fn on_shortcuts_event(&mut self, event: &ShortcutsEvent, cx: &mut Context<Self>) {
        if matches!(event, ShortcutsEvent::Dismissed) {
            self.overlays.close(None);
            cx.notify();
        }
    }

    fn on_toggle_find(&mut self, _: &ToggleFind, window: &mut Window, cx: &mut Context<Self>) {
        self.toggle_overlay(OverlayKind::Find, window, cx);
    }

    fn on_toggle_palette(
        &mut self,
        _: &TogglePalette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_overlay(OverlayKind::Palette, window, cx);
    }

    fn on_toggle_settings(
        &mut self,
        _: &ToggleSettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_overlay(OverlayKind::Settings, window, cx);
    }

    fn on_toggle_shortcuts(
        &mut self,
        _: &ToggleShortcuts,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_overlay(OverlayKind::Shortcuts, window, cx);
    }

    fn on_dismiss(&mut self, _: &Dismiss, window: &mut Window, cx: &mut Context<Self>) {
        self.dismiss(window, cx);
    }

    fn on_find_next(&mut self, _: &FindNext, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(find) = self.overlays.find().cloned() {
            find.update(cx, |find, cx| find.advance(false, cx));
        }
    }

    fn on_find_previous(&mut self, _: &FindPrevious, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(find) = self.overlays.find().cloned() {
            find.update(cx, |find, cx| find.advance(true, cx));
        }
    }

    fn on_zoom_in(&mut self, _: &ZoomIn, _: &mut Window, cx: &mut Context<Self>) {
        self.zoom_with_feedback(PrefEdit::ZoomIn, cx);
    }

    fn on_zoom_out(&mut self, _: &ZoomOut, _: &mut Window, cx: &mut Context<Self>) {
        self.zoom_with_feedback(PrefEdit::ZoomOut, cx);
    }

    fn on_zoom_reset(&mut self, _: &ZoomReset, _: &mut Window, cx: &mut Context<Self>) {
        self.zoom_with_feedback(PrefEdit::ZoomReset, cx);
    }

    /// Zoom from a shortcut, command or the pill itself, and flash the zoom pill. Settings shows
    /// its own Text size stepper, so the pill stays hidden behind it.
    pub(crate) fn zoom_with_feedback(&mut self, edit: PrefEdit, cx: &mut Context<Self>) {
        self.apply_pref(edit, cx);
        if self.overlays.kind() == Some(OverlayKind::Settings) {
            return;
        }
        let generation = self.zoom_hud.show();
        self.expire_zoom_hud_after(generation, zoom_hud::VISIBLE_FOR, cx);
        cx.notify();
    }

    fn expire_zoom_hud_after(&self, generation: u64, delay: Duration, cx: &mut Context<Self>) {
        // The executor's timer (unlike smol's `Timer`) follows the test clock.
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let fade = this
                .update(cx, |this, cx| {
                    let reduce_motion = zoom_hud::prefers_reduced_motion();
                    let fade = this.zoom_hud.expire(generation, reduce_motion);
                    cx.notify();
                    fade
                })
                .ok()
                .flatten();
            if let Some(fade) = fade {
                cx.background_executor().timer(zoom_hud::FADE_OUT).await;
                this.update(cx, |this, cx| {
                    if this.zoom_hud.finish_fade(fade) {
                        cx.notify();
                    }
                })
                .ok();
            }
        })
        .detach();
    }

    fn hover_zoom_hud(&mut self, hovered: bool, cx: &mut Context<Self>) {
        if let Some(generation) = self.zoom_hud.hover(hovered) {
            self.expire_zoom_hud_after(generation, zoom_hud::HOVER_GRACE, cx);
        }
        cx.notify();
    }

    /// Palette, settings, shortcuts, find or a context menu own the screen: no cheat sheet.
    fn cheat_sheet_blocked(&self) -> bool {
        self.overlays.kind().is_some() || self.context_menu.is_some()
    }

    fn cheat_sheet_modifiers_changed(
        &mut self,
        modifiers: gpui::Modifiers,
        cx: &mut Context<Self>,
    ) {
        let was_visible = self.cheat_sheet.is_visible();
        let blocked = self.cheat_sheet_blocked();
        if let Some(generation) = self.cheat_sheet.modifiers_changed(modifiers, blocked) {
            // The executor's timer (unlike smol's `Timer`) follows the test clock.
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(cheat_sheet::HOLD_DELAY)
                    .await;
                this.update(cx, |this, cx| {
                    let blocked = this.cheat_sheet_blocked();
                    if this.cheat_sheet.timer_fired(generation, blocked) {
                        cx.notify();
                    }
                })
                .ok();
            })
            .detach();
        }
        if was_visible != self.cheat_sheet.is_visible() {
            cx.notify();
        }
    }

    #[cfg(test)]
    pub(crate) fn cheat_sheet_visible(&self) -> bool {
        self.cheat_sheet.is_visible()
    }

    #[cfg(test)]
    pub(crate) fn zoom_hud_phase(&self) -> zoom_hud::HudPhase {
        self.zoom_hud.phase()
    }

    pub(crate) fn set_sidebar_mode(&mut self, mode: SidebarMode, cx: &mut Context<Self>) {
        self.apply_pref(PrefEdit::Sidebar(mode), cx);
    }

    fn on_sidebar_recents(&mut self, _: &SidebarRecents, _: &mut Window, cx: &mut Context<Self>) {
        self.set_sidebar_mode(SidebarMode::Recents, cx);
    }

    fn on_sidebar_folder(&mut self, _: &SidebarFolder, _: &mut Window, cx: &mut Context<Self>) {
        self.apply_pref(PrefEdit::Sidebar(SidebarMode::Folder), cx);
    }

    fn on_sidebar_outline(&mut self, _: &SidebarOutline, _: &mut Window, cx: &mut Context<Self>) {
        self.apply_pref(PrefEdit::Sidebar(SidebarMode::Outline), cx);
    }

    fn clear_reader_transient_state(&mut self) {
        self.copied_code = None;
        self.hovered_link = None;
        self.focused_link = None;
    }

    /// Transient reader feedback (find hits, hovered/focused link, copied code) belongs to the
    /// focused document; the other split pane paints none of it.
    pub(crate) fn reader_paint_state(&self, document: &Path, cx: &App) -> ReaderPaintState {
        let focused = self
            .model
            .tabs
            .active()
            .is_some_and(|tab| tab.path() == document);
        if !focused {
            return ReaderPaintState {
                copied_code: None,
                hovered_link: None,
                focused_link: None,
                find_hits: None,
                find_active: None,
            };
        }
        let matches = self.overlays.find().map(|find| find.read(cx).matches());
        ReaderPaintState {
            copied_code: self.copied_code,
            hovered_link: self.hovered_link,
            focused_link: self.focused_link,
            find_hits: matches.map(FindMatches::shared_hits),
            find_active: matches.and_then(FindMatches::active),
        }
    }

    fn active_reader_pane(&self) -> Option<Entity<ReaderPane>> {
        let path = self.model.tabs.active()?.path();
        self.reader_panes.get(path).cloned()
    }

    /// A click in the reader takes focus from a text field (such as the find bar) so Copy and
    /// Select All go to the reader, as in a browser.
    pub(crate) fn focus_reader(&self, window: &mut Window) {
        if !self.focus_handle.is_focused(window) {
            self.focus_handle.focus(window);
        }
    }

    pub(crate) fn reader_has_selection(&self, path: &Path, cx: &App) -> bool {
        self.reader_panes
            .get(path)
            .is_some_and(|pane| pane.read(cx).has_selection())
    }

    pub(crate) fn open_reader_context_menu(
        &mut self,
        path: &Path,
        has_selection: bool,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let spec = vec![
            menu_item_if(
                has_selection,
                "Copy",
                ContextAction::CopySelection(path.to_owned()),
            ),
            menu_item("Select All", ContextAction::SelectAll(path.to_owned())),
        ];
        self.open_context_menu(spec, position, window, cx);
    }

    /// Edit > Copy (and Cmd+C) when no text field has focus: copy the reader's selection.
    fn on_reader_copy(&mut self, _: &field::Copy, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(pane) = self.active_reader_pane() {
            pane.update(cx, |pane, cx| pane.copy_selection(cx));
        }
    }

    /// Edit > Select All (and Cmd+A) when no text field has focus: select the whole document.
    fn on_reader_select_all(
        &mut self,
        _: &field::SelectAll,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if matches!(self.overlays.kind(), None | Some(OverlayKind::Find))
            && let Some(pane) = self.active_reader_pane()
        {
            pane.update(cx, |pane, cx| pane.select_all(cx));
        }
    }

    /// Outline row to highlight for the reader's current scroll position.
    pub(crate) fn active_outline_heading(&self, cx: &App) -> Option<usize> {
        let path = self.model.tabs.active()?.path();
        self.reader_panes.get(path)?.read(cx).active_heading()
    }

    #[cfg(test)]
    pub(crate) fn reader_list_state(&self, path: &Path, cx: &App) -> Option<gpui::ListState> {
        self.reader_panes
            .get(path)
            .map(|pane| pane.read(cx).list_state())
    }

    fn ensure_reader_pane(
        &mut self,
        document: Arc<crate::syntax::PreparedDocument>,
        load: TabLoad,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<ReaderPane> {
        let pane = self.ensure_reader_pane_for(document.clone(), load, window, cx);
        let path = &document.path;
        if load.is_ready()
            && !document.blocks.is_empty()
            && let Some(anchor) = self.restore_anchors.remove(path)
        {
            pane.update(cx, |pane, cx| {
                pane.restore_anchor(anchor);
                cx.notify();
            });
        }
        if let Some(anchor) = pane.read(cx).scroll_anchor()
            && self.anchors.insert(path.clone(), anchor) != Some(anchor)
        {
            self.schedule_session_save(cx);
        }
        pane
    }

    /// Saves the session (with reading positions) once scrolling has settled for a moment.
    fn schedule_session_save(&mut self, cx: &mut Context<Self>) {
        if self.session_save.is_some() {
            return;
        }
        self.session_save = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(750))
                .await;
            this.update(cx, |this, _| {
                this.session_save = None;
                this.persist_session();
            })
            .ok();
        }));
    }

    fn ensure_reader_pane_for(
        &mut self,
        document: Arc<crate::syntax::PreparedDocument>,
        load: TabLoad,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<ReaderPane> {
        let path = document.path.clone();
        let style = self.prefs.get().reader_style();
        let theme = self.theme;
        let live = self
            .model
            .tabs
            .paths()
            .map(Path::to_owned)
            .collect::<HashSet<_>>();
        self.reader_panes
            .retain(|open_path, _| live.contains(open_path));
        self.reader_link_focus_handles
            .retain(|(open_path, _), _| live.contains(open_path));
        if let Some(pane) = self.reader_panes.get(&path).cloned() {
            if !pane.read(cx).hosts_document(&document) {
                self.retain_reader_link_focus_handles(&document, window);
            }
            pane.update(cx, |pane, cx| pane.sync(document, load, style, theme, cx));
            pane
        } else {
            self.retain_reader_link_focus_handles(&document, window);
            let app = cx.weak_entity();
            let pane = cx.new(|cx| {
                let mut pane = ReaderPane::new(app, document.clone(), style, theme);
                pane.sync(document, load, style, theme, cx);
                pane
            });
            self.reader_panes.insert(path, pane.clone());
            cx.on_next_frame(window, |_, _, cx| cx.notify());
            pane
        }
    }

    fn sync_focused_link(&mut self, window: &Window, cx: &mut Context<Self>) {
        let focused_link =
            self.reader_link_focus_handles
                .iter()
                .find_map(|((path, key), handle)| {
                    (self
                        .model
                        .tabs
                        .active()
                        .is_some_and(|tab| tab.path() == path)
                        && handle.is_focused(window))
                    .then_some(*key)
                });
        if self.focused_link != focused_link {
            self.focused_link = focused_link;
            cx.notify();
        }
    }

    fn retain_reader_link_focus_handles(
        &mut self,
        document: &PreparedDocument,
        window: &mut Window,
    ) {
        // Handles are created lazily for rendered blocks; a document nobody has rendered yet
        // (a fresh load) has none to prune. The link keys come precomputed off the UI thread.
        if self.focused_link.is_none()
            && !self
                .reader_link_focus_handles
                .keys()
                .any(|(path, _)| path == &document.path)
        {
            return;
        }
        let active_keys = &document.layout().link_keys;
        let mut removed_focused_handle = false;
        self.reader_link_focus_handles
            .retain(|(path, key), handle| {
                let keep = path != &document.path || active_keys.contains(key);
                if !keep && handle.is_focused(window) {
                    removed_focused_handle = true;
                }
                keep
            });
        let focused_handle_key =
            self.reader_link_focus_handles
                .iter()
                .find_map(|((path, key), handle)| {
                    (path == &document.path && handle.is_focused(window)).then_some(*key)
                });
        let focus_state_mismatch = focused_handle_key != self.focused_link;
        if removed_focused_handle || (focus_state_mismatch && focused_handle_key.is_some()) {
            self.focus_handle.focus(window);
        }
        if removed_focused_handle || focus_state_mismatch {
            self.focused_link = None;
        }
    }

    pub(crate) fn ensure_block_link_focus_handles(
        &mut self,
        document: &ParsedDocument,
        block_index: usize,
        cx: &mut Context<Self>,
    ) -> HashMap<LinkFocusKey, FocusHandle> {
        use crate::ui::reader::block_link_focus_targets;

        block_link_focus_targets(document, block_index)
            .into_iter()
            .map(|target| {
                let map_key = (document.path.clone(), target.key);
                let handle = self
                    .reader_link_focus_handles
                    .entry(map_key)
                    .or_insert_with(|| cx.focus_handle().tab_index(0).tab_stop(true))
                    .clone();
                (target.key, handle)
            })
            .collect()
    }

    pub fn close_active_tab(&mut self, _: &CloseTab, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = self.model.tabs.active().map(|tab| tab.path().to_owned()) {
            self.close_tab(&path, cx);
        }
    }

    pub(crate) fn toggle_directory(&mut self, path: &Path, cx: &mut Context<Self>) {
        if self
            .model
            .workspace
            .as_mut()
            .is_some_and(|workspace| workspace.toggle_directory(path))
        {
            cx.notify();
        }
    }

    pub(crate) fn activate_tab(&mut self, path: &Path, cx: &mut Context<Self>) {
        if self.model.tabs.activate(path) {
            self.open_error = None;
            self.active_document_changed(cx);
            cx.notify();
        }
    }

    fn select_tab_index(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(path) = self.model.tabs.path_at(index).map(Path::to_owned) {
            self.activate_tab(&path, cx);
        }
    }

    fn cycle_tab(&mut self, step: isize, cx: &mut Context<Self>) {
        if let Some(path) = self.model.tabs.cycled_path(step).map(Path::to_owned) {
            self.activate_tab(&path, cx);
        }
    }

    fn on_next_tab(&mut self, _: &NextTab, _: &mut Window, cx: &mut Context<Self>) {
        self.cycle_tab(1, cx);
    }

    fn on_previous_tab(&mut self, _: &PreviousTab, _: &mut Window, cx: &mut Context<Self>) {
        self.cycle_tab(-1, cx);
    }

    fn on_select_last_tab(&mut self, _: &SelectLastTab, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(last) = self.model.tabs.len().checked_sub(1) {
            self.select_tab_index(last, cx);
        }
    }

    /// Reader scrolling must not steal keys from text fields, modal overlays, or a focused
    /// control that Space activates.
    fn reader_may_take_key(&self, key: &str, window: &Window, cx: &App) -> bool {
        if self.context_menu.is_some() {
            return false;
        }
        if window
            .context_stack()
            .iter()
            .any(|context| context.contains("Field"))
        {
            return false;
        }
        match key {
            "space" => window
                .focused(cx)
                .is_none_or(|focused| focused == self.focus_handle),
            "up" | "down" => matches!(self.overlays.kind(), None | Some(OverlayKind::Find)),
            _ => true,
        }
    }

    fn scroll_active_reader(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        let Some(path) = self.model.tabs.active().map(|tab| tab.path().to_owned()) else {
            return false;
        };
        let Some(pane) = self.reader_panes.get(&path).cloned() else {
            return false;
        };
        pane.update(cx, |pane, cx| {
            let scrolled = pane.scroll_by_key(key);
            if scrolled {
                cx.notify();
            }
            scrolled
        })
    }

    pub(crate) fn close_tab(&mut self, path: &Path, cx: &mut Context<Self>) {
        let Some(path) = self.model.tabs.get(path).map(|tab| tab.path().to_owned()) else {
            return;
        };
        let path = path.as_path();
        if self.model.close_tab(path) {
            let tabs = self.tab_paths();
            let active = self.model.tabs.active().map(|tab| tab.path().to_owned());
            if let Some(target) = self.split.closed(path, &tabs, active.as_deref()) {
                self.model.tabs.activate(&target);
            }
            self.tab_focus.remove(path);
            self.reader_panes.remove(path);
            self.load_tasks.remove(path);
            self.anchors.remove(path);
            self.restore_anchors.remove(path);
            self.reader_link_focus_handles
                .retain(|(document_path, _), _| document_path != path);
            self.active_document_changed(cx);
            cx.notify();
        }
    }

    pub(crate) fn jump_to_heading(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(tab) = self.model.tabs.active() else {
            return;
        };
        let Some(heading_path) = tab.document.heading_path(index) else {
            return;
        };
        let Some(pane) = self.reader_panes.get(tab.path()).cloned() else {
            return;
        };
        pane.update(cx, |pane, cx| pane.jump_to_heading(heading_path, cx));
    }

    pub(crate) fn dismiss_reload_error(&mut self, cx: &mut Context<Self>) {
        if self.model.dismiss_active_reload_error() {
            cx.notify();
        }
    }

    pub(crate) fn activate_link(
        &mut self,
        document_path: &Path,
        target: &str,
        cx: &mut Context<Self>,
    ) {
        match classify_link(document_path, target) {
            LinkRoute::Markdown(path) => self.open_path(&path, cx),
            LinkRoute::Anchor(fragment) => {
                let heading = self
                    .model
                    .tabs
                    .active()
                    .and_then(|tab| tab.document.anchor_heading(&fragment));
                if let Some(heading) = heading {
                    self.jump_to_heading(heading, cx);
                    return;
                }
                let block = self
                    .model
                    .tabs
                    .active()
                    .and_then(|tab| tab.document.anchor_block(&fragment));
                if let Some(block) = block {
                    self.scroll_reader_to_block(block, cx);
                }
            }
            LinkRoute::Web(url) => {
                let _ = open::that(url);
            }
            LinkRoute::Local(path) => {
                let _ = open::that(path);
            }
            LinkRoute::Inert => {}
        }
    }

    pub(crate) fn set_hovered_link(
        &mut self,
        hovered_link: Option<LinkFocusKey>,
        cx: &mut Context<Self>,
    ) {
        if self.hovered_link != hovered_link {
            self.hovered_link = hovered_link;
            cx.notify();
        }
    }

    pub(crate) fn clear_hovered_link_for_surface(
        &mut self,
        surface: LinkSurfaceKey,
        cx: &mut Context<Self>,
    ) {
        if self.hovered_link.is_some_and(|key| key.surface == surface) {
            self.hovered_link = None;
            cx.notify();
        }
    }

    pub(crate) fn copy_code(&mut self, block_index: usize, code: String, cx: &mut Context<Self>) {
        let copied_at = Instant::now();
        cx.write_to_clipboard(ClipboardItem::new_string(code));
        self.copied_code = Some((block_index, copied_at));
        cx.notify();
        cx.spawn(async move |this, cx| {
            Timer::after(Duration::from_secs(2)).await;
            this.update(cx, |this, cx| {
                if clear_expired_code_copy_feedback(
                    &mut this.copied_code,
                    block_index,
                    Instant::now(),
                ) {
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }
}

impl MdowApp {
    pub(crate) fn folder_filter_query(&self, cx: &App) -> String {
        self.folder_filter.read(cx).text().to_owned()
    }

    #[cfg(test)]
    pub(crate) fn set_folder_filter(&mut self, query: &str, cx: &mut Context<Self>) {
        self.folder_filter
            .update(cx, |field, cx| field.set_text(query.to_owned(), cx));
    }

    fn on_filter_event(
        &mut self,
        field: &Entity<Field>,
        event: &FieldEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            FieldEvent::Edited => {
                self.filter_collapsed.clear();
                cx.notify();
            }
            FieldEvent::Cancelled => {
                if field.read(cx).text().is_empty() {
                    self.focus_handle.focus(window);
                } else {
                    field.update(cx, |field, cx| field.set_text("", cx));
                }
                cx.notify();
            }
            FieldEvent::Submitted { .. } => {
                let query = field.read(cx).text().to_owned();
                let first = self.model.workspace.as_ref().and_then(|tree| {
                    tree.filtered_rows(&query, &self.filter_collapsed)
                        .rows
                        .into_iter()
                        .find(|row| row.row.kind == WorkspaceEntryKind::File)
                        .map(|row| row.row.path)
                });
                if let Some(path) = first {
                    self.open_path(&path, cx);
                }
            }
        }
    }

    /// Folder rows toggle the tree's own expansion, or the filter-only collapse set while
    /// a filter is active (filtered views always start with every match's ancestors open).
    pub(crate) fn toggle_tree_directory(
        &mut self,
        path: &Path,
        filtering: bool,
        cx: &mut Context<Self>,
    ) {
        if filtering {
            if !self.filter_collapsed.remove(path) {
                self.filter_collapsed.insert(path.to_owned());
            }
            cx.notify();
        } else {
            self.toggle_directory(path, cx);
        }
    }

    fn persist_session(&mut self) {
        self.prefs.save_session(&self.session_snapshot());
    }

    pub(crate) fn clear_recents(&mut self, cx: &mut Context<Self>) {
        if self.model.recents.clear() {
            self.persist_session();
            cx.notify();
        }
    }

    pub(crate) fn remove_recent(&mut self, path: &Path, cx: &mut Context<Self>) {
        if self.model.recents.remove(path) {
            self.persist_session();
            cx.notify();
        }
    }

    fn close_tabs(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        for path in paths {
            self.close_tab(&path, cx);
        }
    }

    fn tab_paths(&self) -> Vec<PathBuf> {
        self.model.tabs.paths().map(Path::to_owned).collect()
    }

    pub(crate) fn run_context_action(&mut self, action: ContextAction, cx: &mut Context<Self>) {
        match action {
            ContextAction::Open(path) => self.open_path(&path, cx),
            ContextAction::CloseTab(path) => self.close_tab(&path, cx),
            ContextAction::CloseOtherTabs(path) => {
                let others = self
                    .tab_paths()
                    .into_iter()
                    .filter(|tab| tab != &path)
                    .collect();
                self.activate_tab(&path, cx);
                self.close_tabs(others, cx);
            }
            ContextAction::CloseTabsToRight(path) => {
                let tabs = self.tab_paths();
                let right = tabs
                    .iter()
                    .position(|tab| tab == &path)
                    .map(|index| tabs[index + 1..].to_vec())
                    .unwrap_or_default();
                self.close_tabs(right, cx);
            }
            ContextAction::CloseAllTabs => {
                let tabs = self.tab_paths();
                self.close_tabs(tabs, cx);
            }
            ContextAction::OpenInPane(path, pane) => self.open_in_pane(&path, pane, cx),
            ContextAction::CopyPath(path) => {
                cx.write_to_clipboard(ClipboardItem::new_string(
                    path.to_string_lossy().into_owned(),
                ));
            }
            ContextAction::Reveal(path) => self.reveal_path(&path),
            ContextAction::RemoveRecent(path) => self.remove_recent(&path, cx),
            ContextAction::CopySelection(path) => {
                if let Some(pane) = self.reader_panes.get(&path).cloned() {
                    pane.update(cx, |pane, cx| pane.copy_selection(cx));
                }
            }
            ContextAction::SelectAll(path) => {
                if let Some(pane) = self.reader_panes.get(&path).cloned() {
                    pane.update(cx, |pane, cx| pane.select_all(cx));
                }
            }
        }
    }

    pub(crate) fn open_context_menu(
        &mut self,
        spec: ContextMenuSpec,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (entries, actions): (Vec<_>, Vec<_>) = spec.into_iter().unzip();
        let theme = self.theme;
        let view = cx.new(|cx| ContextMenu::new(entries, theme, window, cx));
        let events = cx.subscribe_in(&view, window, |this, menu, event, window, cx| {
            // A late event from a menu that was already replaced must not close its successor.
            if this
                .context_menu
                .as_ref()
                .is_some_and(|open| open.view == *menu)
            {
                this.on_context_menu_event(*event, window, cx);
            }
        });
        self.context_menu = Some(OpenContextMenu {
            view,
            actions,
            position,
            _events: events,
        });
        cx.notify();
    }

    fn close_context_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.context_menu.take().is_none() {
            return false;
        }
        self.focus_handle.focus(window);
        cx.notify();
        true
    }

    fn on_context_menu_event(
        &mut self,
        event: ContextMenuEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let action = match event {
            ContextMenuEvent::Confirmed(index) => self
                .context_menu
                .as_ref()
                .and_then(|menu| menu.actions.get(index).cloned().flatten()),
            ContextMenuEvent::Dismissed => None,
        };
        self.close_context_menu(window, cx);
        if let Some(action) = action {
            self.run_context_action(action, cx);
        }
    }

    pub(crate) fn open_tab_context_menu(
        &mut self,
        path: &Path,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let spec = tab_context_menu(path, &self.tab_paths());
        self.open_context_menu(spec, position, window, cx);
    }

    pub(crate) fn open_tree_context_menu(
        &mut self,
        path: &Path,
        directory: bool,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_context_menu(tree_context_menu(path, directory), position, window, cx);
    }

    pub(crate) fn open_recent_context_menu(
        &mut self,
        path: &Path,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_context_menu(recent_context_menu(path), position, window, cx);
    }

    #[cfg(test)]
    pub(crate) fn context_menu_view(&self) -> Option<Entity<ContextMenu>> {
        self.context_menu.as_ref().map(|menu| menu.view.clone())
    }

    /// Focus handles for each tab and its close control, kept stable per path.
    fn sync_tab_focus(&mut self, cx: &mut Context<Self>) -> Vec<TabFocus> {
        let paths = self.tab_paths();
        self.tab_focus.retain(|path, _| paths.contains(path));
        paths
            .into_iter()
            .map(|path| {
                let focus = self.tab_focus.entry(path).or_insert_with(|| TabFocus {
                    tab: cx.focus_handle().tab_index(0).tab_stop(true),
                    close: cx.focus_handle().tab_index(0).tab_stop(true),
                });
                TabFocus {
                    tab: focus.tab.clone(),
                    close: focus.close.clone(),
                }
            })
            .collect()
    }

    /// Keeps the File > Open Recent submenu in step with this window's recents.
    fn sync_recent_menu(&mut self, cx: &mut Context<Self>) {
        let recents = self
            .model
            .recents
            .iter()
            .map(Path::to_owned)
            .collect::<Vec<_>>();
        if self.menu_recents.as_ref() == Some(&recents) {
            return;
        }
        cx.set_menus(crate::menus::app_menus(&recents));
        self.menu_recents = Some(recents);
    }

    fn on_open_recent(&mut self, action: &OpenRecent, _: &mut Window, cx: &mut Context<Self>) {
        self.open_path(&action.path, cx);
    }

    fn on_clear_recents(&mut self, _: &ClearRecents, _: &mut Window, cx: &mut Context<Self>) {
        self.clear_recents(cx);
    }
}

impl MdowApp {
    /// Banners plus the reader for one open document. Window-level open errors only show in
    /// the focused pane.
    fn render_document_surface(
        &mut self,
        path: &Path,
        focused: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = self.theme;
        let tab = self
            .model
            .tabs
            .get(path)
            .expect("split panes only render open documents");
        let (document, load, path, reload_error) = (
            tab.document.clone(),
            tab.load,
            tab.path().to_owned(),
            tab.reload_error.clone(),
        );
        let mut surface = div()
            .flex()
            .flex_col()
            .flex_grow()
            .min_w_0()
            .min_h_0()
            .bg(theme.background);
        if focused && let Some(error) = self.open_error.as_ref() {
            surface = surface.child(render_error_banner(theme, error));
        }
        if self.model.is_deleted(&path) {
            surface = surface.child(render_deleted_banner(theme, &path, cx));
        } else if let Some(body) = reload_error {
            surface = surface.child(render_reload_error_banner(
                theme,
                &UserFacingError {
                    title: "Couldn't reload this file".into(),
                    body,
                    path: path.clone(),
                },
                cx,
            ));
        }
        let pane = self.ensure_reader_pane(document, load, window, cx);
        surface.child(pane).into_any_element()
    }
}

impl Focusable for MdowApp {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MdowApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::perf::mark_once_after("window_render", "open_start");
        let bounds = window.bounds();
        self.last_window_bounds = Some(SavedWindowBounds {
            x: f32::from(bounds.origin.x),
            y: f32::from(bounds.origin.y),
            width: f32::from(bounds.size.width),
            height: f32::from(bounds.size.height),
        });
        self.theme = self.resolve_theme(window);
        let theme = self.theme;
        let scale = self.prefs.get().interface_scale;
        if crate::theme::active_ui_scale(cx) != scale
            || !cx.has_global::<crate::theme::ActiveUiScale>()
        {
            cx.set_global(crate::theme::ActiveUiScale(scale));
        }
        self.folder_filter
            .update(cx, |field, _| field.apply_theme(theme));
        self.sync_recent_menu(cx);
        let window_width = f32::from(window.viewport_size().width);
        // The companion panel takes its width from the right edge before the shell lays out.
        self.companion.set_prefs(self.prefs.get().companion);
        let companion_width = self.companion.reserved_width(window_width, cx);
        let layout = ShellLayout::for_width_scaled(
            window_width - companion_width,
            self.sidebar_open,
            self.wide_mode,
            theme.ui,
        );
        let zoom_hud = self
            .zoom_hud
            .is_visible()
            .then(|| {
                render_zoom_hud(
                    &self.zoom_hud,
                    self.prefs.get().zoom.percent(),
                    zoom_hud::prefers_reduced_motion(),
                    self.theme,
                    ZoomHudHandlers {
                        zoom_out: Box::new(cx.listener(|this, _, _, cx| {
                            this.zoom_with_feedback(PrefEdit::ZoomOut, cx)
                        })),
                        zoom_in: Box::new(cx.listener(|this, _, _, cx| {
                            this.zoom_with_feedback(PrefEdit::ZoomIn, cx)
                        })),
                        reset: Box::new(cx.listener(|this, _, _, cx| {
                            this.zoom_with_feedback(PrefEdit::ZoomReset, cx)
                        })),
                        hover: Box::new(cx.listener(|this, hovered: &bool, _, cx| {
                            this.hover_zoom_hud(*hovered, cx)
                        })),
                    },
                )
            })
            .flatten();
        let active_path = self.model.tabs.active().map(|tab| tab.path().to_owned());
        let title = window_title_for(active_path.as_deref());
        if self.window_title.as_deref() != Some(title.as_str()) {
            window.set_window_title(&title);
            self.window_title = Some(title);
        }

        let active_heading = self.active_outline_heading(cx);
        if active_heading != self.last_outline_active {
            if let Some(index) = active_heading {
                self.outline_scroll
                    .scroll_to_item(index, gpui::ScrollStrategy::Center);
            }
            self.last_outline_active = active_heading;
        }

        let sidebar = (layout.sidebar.width > 0.0).then(|| {
            let active_document = self.model.tabs.active().map(|tab| tab.document.clone());
            let filter_query = self.folder_filter_query(cx);
            render_sidebar(
                theme,
                SidebarProps {
                    mode: self.prefs.get().sidebar_mode,
                    recents: &self.model.recents,
                    workspace: self.model.workspace.as_ref(),
                    workspace_error: self.model.workspace_error.as_ref(),
                    document_title: active_document
                        .as_deref()
                        .map(|document| document.title.as_str()),
                    outline: active_document.clone(),
                    active_heading,
                    active_path: active_path.as_deref(),
                    filter: &self.folder_filter,
                    filter_query: &filter_query,
                    filter_collapsed: &self.filter_collapsed,
                    outline_scroll: &self.outline_scroll,
                    width: layout.sidebar.width,
                },
                cx,
            )
        });

        let mut main = div()
            .debug_selector(|| "main-column".into())
            .relative()
            .flex()
            .flex_col()
            .min_w_0()
            .min_h_0()
            .flex_grow();
        if self.model.tabs.is_empty() {
            let companion_button = self.companion.toggle_button(theme, cx);
            main = main
                .child(render_empty_toolbar(theme, &layout, companion_button, cx))
                .child(match self.open_error.as_ref() {
                    Some(error) => error_state(theme, error, cx),
                    None => welcome(theme, &self.model.recents, cx),
                });
        } else {
            let tab_focus = self.sync_tab_focus(cx);
            let tab_bar = render_tab_bar(theme, self, &layout, &tab_focus, window, cx);
            main = main.child(tab_bar);
            if self.split.is_enabled() {
                let active_pane = self.split.active_pane();
                let panes = [PaneId::Primary, PaneId::Secondary].map(|pane| {
                    let path = self.split.pane_path(pane).map(Path::to_owned);
                    let title = path.as_deref().and_then(|path| {
                        self.model
                            .tabs
                            .get(path)
                            .map(crate::ui::chrome::breadcrumb_display)
                    });
                    let content =
                        path.filter(|path| self.model.tabs.get(path).is_some())
                            .map(|path| {
                                self.render_document_surface(&path, pane == active_pane, window, cx)
                            });
                    SplitPane {
                        pane,
                        title: title.map(|display| display.primary),
                        active: pane == active_pane,
                        content,
                    }
                });
                let [primary, secondary] = panes;
                main = main.child(render_split(
                    theme,
                    &self.split,
                    layout.main.width,
                    self.divider_dragging,
                    primary,
                    secondary,
                    cx,
                ));
            } else {
                let tab = self
                    .model
                    .tabs
                    .active()
                    .expect("a non-empty tab set always has an active document");
                let breadcrumb = render_breadcrumb(theme, tab, cx);
                let path = tab.path().to_owned();
                let surface = self.render_document_surface(&path, true, window, cx);
                main = main.child(breadcrumb).child(surface);
            }
        }
        main = main.children(zoom_hud);

        let drop_overlay_layer = self
            .drop_state
            .is_active()
            .then(|| drop_overlay(theme, self.drop_state.summary()));
        if self.cheat_sheet_blocked() {
            self.cheat_sheet.reset();
        }
        let cheat_sheet_layer = self.cheat_sheet.is_visible().then(|| {
            render_cheat_sheet(&self.cheat_sheet, zoom_hud::prefers_reduced_motion(), theme)
        });
        let context_menu_layer = self
            .context_menu
            .as_ref()
            .map(|menu| context_menu_layer(menu.view.clone(), menu.position));
        let companion_context = DocumentContext {
            active_path: active_path.clone(),
            workspace_root: self
                .model
                .workspace
                .as_ref()
                .map(|tree| tree.root.path.clone()),
        };
        let companion_panel = self.companion.render(
            self.prefs.get().companion,
            theme,
            companion_context,
            window_width,
            &self.focus_handle,
            window,
            cx,
        );

        div()
            .id("mdow-root")
            .track_focus(&self.focus_handle)
            .on_modifiers_changed(cx.listener(
                |this, event: &gpui::ModifiersChangedEvent, _, cx| {
                    this.cheat_sheet_modifiers_changed(event.modifiers, cx);
                },
            ))
            .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if this.cheat_sheet.key_down() {
                    cx.notify();
                }
                let modifiers = event.keystroke.modifiers;
                let key = event.keystroke.key.as_str();
                if reader_key_modifiers_are_allowed(
                    key,
                    modifiers.control,
                    modifiers.alt,
                    modifiers.platform,
                    modifiers.function,
                ) && this.reader_may_take_key(key, window, cx)
                    && this.scroll_active_reader(reader_scroll_key(key, modifiers.shift), cx)
                {
                    cx.stop_propagation();
                    return;
                }
                if event.keystroke.key == "tab"
                    && !modifiers.control
                    && !modifiers.alt
                    && !modifiers.platform
                    && !modifiers.function
                {
                    if modifiers.shift {
                        window.focus_prev();
                    } else {
                        window.focus_next();
                    }
                    this.sync_focused_link(window, cx);
                    window.prevent_default();
                    cx.stop_propagation();
                }
            }))
            .on_action(cx.listener(Self::open_file))
            .on_action(cx.listener(Self::open_folder))
            .on_action(cx.listener(Self::toggle_sidebar))
            .on_action(cx.listener(Self::close_active_tab))
            .on_action(cx.listener(Self::toggle_wide_mode))
            .on_action(cx.listener(Self::on_toggle_split_view))
            .on_action(cx.listener(Self::on_toggle_find))
            .on_action(cx.listener(Self::on_toggle_palette))
            .on_action(cx.listener(Self::on_toggle_settings))
            .on_action(cx.listener(Self::on_toggle_shortcuts))
            .on_action(cx.listener(Self::on_check_for_updates))
            .on_action(cx.listener(Self::on_dismiss))
            .on_action(cx.listener(Self::on_find_next))
            .on_action(cx.listener(Self::on_find_previous))
            .on_action(cx.listener(Self::on_reader_copy))
            .on_action(cx.listener(Self::on_reader_select_all))
            .on_action(cx.listener(Self::on_zoom_in))
            .on_action(cx.listener(Self::on_zoom_out))
            .on_action(cx.listener(Self::on_zoom_reset))
            .on_action(cx.listener(Self::on_sidebar_recents))
            .on_action(cx.listener(Self::on_sidebar_folder))
            .on_action(cx.listener(Self::on_sidebar_outline))
            .on_action(cx.listener(Self::on_next_tab))
            .on_action(cx.listener(Self::on_previous_tab))
            .on_action(cx.listener(|this, _: &SelectTab1, _, cx| this.select_tab_index(0, cx)))
            .on_action(cx.listener(|this, _: &SelectTab2, _, cx| this.select_tab_index(1, cx)))
            .on_action(cx.listener(|this, _: &SelectTab3, _, cx| this.select_tab_index(2, cx)))
            .on_action(cx.listener(|this, _: &SelectTab4, _, cx| this.select_tab_index(3, cx)))
            .on_action(cx.listener(|this, _: &SelectTab5, _, cx| this.select_tab_index(4, cx)))
            .on_action(cx.listener(|this, _: &SelectTab6, _, cx| this.select_tab_index(5, cx)))
            .on_action(cx.listener(|this, _: &SelectTab7, _, cx| this.select_tab_index(6, cx)))
            .on_action(cx.listener(|this, _: &SelectTab8, _, cx| this.select_tab_index(7, cx)))
            .on_action(cx.listener(Self::on_select_last_tab))
            .on_action(cx.listener(Self::on_open_recent))
            .on_action(cx.listener(Self::on_clear_recents))
            .on_action(cx.listener(|this, _: &ToggleCompanion, window, cx| {
                this.toggle_companion(window, cx)
            }))
            .on_action(|_: &Minimize, window, _| window.minimize_window())
            .on_action(|_: &Zoom, window, _| window.zoom_window())
            .on_action(|_: &ToggleFullScreen, window, _| window.toggle_fullscreen())
            .on_drag_move::<ExternalPaths>(cx.listener(
                |this, event: &DragMoveEvent<ExternalPaths>, window, cx| {
                    let summary = DropSummary::from_paths(event.drag(cx).paths());
                    this.drag_moved(summary, window, cx);
                },
            ))
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| {
                this.open_paths(paths.paths().to_vec(), cx);
            }))
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .bg(theme.background)
            .font_family(Metrics::FONT_SANS)
            .text_size(px(theme.ui.control_font))
            .text_color(theme.foreground)
            .child(
                div()
                    .flex()
                    .flex_grow()
                    .min_h_0()
                    .children(sidebar)
                    .child(main)
                    .children(companion_panel),
            )
            .children(
                (!self.update_dismissed)
                    .then(|| render_update_banner(theme, &self.update, cx))
                    .flatten(),
            )
            .children(self.overlays.render_layer(theme))
            .children(drop_overlay_layer)
            .children(cheat_sheet_layer)
            .children(context_menu_layer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlay::command_catalog;
    use crate::session::Recents;
    use crate::theme::{TrafficLightClearance, TrafficLights};
    use crate::ui::reader::reader_key_target;
    use crate::ui::text_surface::{SurfaceId, TextPoint};
    use gpui::{
        FileDropEvent, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, MouseButton, MouseDownEvent,
        MouseUpEvent, Pixels, Point, ScrollDelta, ScrollWheelEvent, TestAppContext,
        TitlebarOptions, VisualTestContext, WindowOptions, point,
    };
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        path::{Path, PathBuf},
        sync::Arc,
    };

    fn markdown_workspace() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("guides")).unwrap();
        fs::write(root.path().join("README.md"), "# Home").unwrap();
        fs::write(root.path().join("guides/start.md"), "# Start").unwrap();
        root
    }

    fn watcher_workspace() -> tempfile::TempDir {
        // macOS FSEvents is flaky under /var/folders; Linux has no /private/tmp.
        let parent = if Path::new("/private/tmp").is_dir() {
            Path::new("/private/tmp")
        } else {
            Path::new("/tmp")
        };
        tempfile::Builder::new()
            .prefix("mdow-app-watch-")
            .tempdir_in(parent)
            .unwrap()
    }

    struct PermissionRestore {
        path: PathBuf,
        mode: u32,
    }

    impl PermissionRestore {
        fn deny(path: &Path) -> Self {
            let mut permissions = fs::metadata(path).unwrap().permissions();
            let mode = permissions.mode();
            permissions.set_mode(0o000);
            fs::set_permissions(path, permissions).unwrap();
            Self {
                path: path.to_owned(),
                mode,
            }
        }
    }

    impl Drop for PermissionRestore {
        fn drop(&mut self) {
            let mut permissions = fs::metadata(&self.path).unwrap().permissions();
            permissions.set_mode(self.mode);
            fs::set_permissions(&self.path, permissions).unwrap();
        }
    }

    fn click_debug(visual: &mut VisualTestContext, selector: &'static str) {
        visual.update(|window, cx| window.draw(cx).clear());
        let center = visual
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("{selector} should be painted"))
            .center();
        visual.simulate_mouse_move(center, None, Modifiers::none());
        visual.simulate_mouse_down(center, MouseButton::Left, Modifiers::none());
        visual.simulate_mouse_up(center, MouseButton::Left, Modifiers::none());
    }

    fn focus_next(visual: &mut VisualTestContext, count: usize) {
        visual.update(|window, cx| window.draw(cx).clear());
        for _ in 0..count {
            visual.update(|window, _| window.focus_next());
        }
        visual.update(|window, cx| window.draw(cx).clear());
    }

    fn activate_focused(visual: &mut VisualTestContext, key: &str) {
        visual.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse(key).unwrap(),
        });
    }

    fn block_link_key(block_index: usize, link_index: usize) -> LinkFocusKey {
        LinkFocusKey::new(LinkSurfaceKey::block(block_index), link_index)
    }

    #[test]
    fn opening_a_file_populates_a_tab_and_selects_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("guide.md");
        fs::write(&path, "# Guide").unwrap();
        let mut model = AppModel::default();

        model.open_path(&path).unwrap();

        assert_eq!(model.tabs.len(), 1);
        assert_eq!(model.tabs.active().unwrap().document.title, "Guide");
    }

    #[test]
    fn open_or_activate_reuses_the_existing_document() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("guide.md");
        fs::write(&path, "# Guide").unwrap();
        let mut model = AppModel::default();
        model.open_document(&path).unwrap();
        let before = model.tabs.active().unwrap().document.clone();

        let opened = model.open_or_activate(&path).unwrap();

        assert_eq!(opened, DocumentOpened::ActivatedExisting);
        assert!(Arc::ptr_eq(&before, &model.tabs.active().unwrap().document));
    }

    #[test]
    fn successful_reload_replaces_content_without_reordering_or_reactivating_tabs() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first.md");
        let second = dir.path().join("second.md");
        fs::write(&first, "# First").unwrap();
        fs::write(&second, "# Second").unwrap();
        let mut model = AppModel::default();
        model.open_document(&first).unwrap();
        model.open_document(&second).unwrap();
        let before = model.tabs.paths().map(Path::to_owned).collect::<Vec<_>>();
        let active_before = model.tabs.active().unwrap().path().to_owned();
        fs::write(&first, "# Changed").unwrap();

        model.reload_path(&first).unwrap();

        assert_eq!(
            model.tabs.paths().map(Path::to_owned).collect::<Vec<_>>(),
            before
        );
        assert_eq!(model.tabs.active().unwrap().path(), active_before);
        assert_eq!(model.tabs.get(&first).unwrap().document.title, "Changed");
        assert!(model.tabs.get(&first).unwrap().reload_error.is_none());
    }

    #[test]
    fn failed_reload_preserves_the_last_document_and_sets_a_readable_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("guide.md");
        fs::write(&path, "# Last good").unwrap();
        let mut model = AppModel::default();
        model.open_document(&path).unwrap();
        let before = model.tabs.active().unwrap().document.clone();
        fs::remove_file(&path).unwrap();

        let error = model.reload_path(&path).unwrap_err();

        let tab = model.tabs.active().unwrap();
        assert!(Arc::ptr_eq(&tab.document, &before));
        assert_eq!(tab.document.title, "Last good");
        assert_eq!(
            tab.reload_error.as_deref(),
            Some(error.view().body.as_str())
        );
        assert_eq!(error.view().title, "File not found");
        assert!(!error.view().body.contains("DocumentError"));
    }

    #[test]
    fn opening_a_folder_populates_the_tree_without_opening_a_tab() {
        let root = markdown_workspace();
        let mut model = AppModel::default();

        model.open_path(root.path()).unwrap();

        assert_eq!(
            model.workspace.as_ref().unwrap().root.path,
            root.path().canonicalize().unwrap()
        );
        assert!(model.tabs.is_empty());
    }

    #[test]
    fn a_failed_open_preserves_the_last_successful_workspace_and_tabs() {
        let root = markdown_workspace();
        let file = root.path().join("README.md");
        let invalid = root.path().join("broken.md");
        fs::write(&invalid, [0xff, 0xfe]).unwrap();
        let mut model = AppModel::default();
        model.open_path(root.path()).unwrap();
        model.open_path(&file).unwrap();

        let error = model.open_path(&invalid).unwrap_err();

        assert!(matches!(error, AppOpenError::Document(_)));
        assert_eq!(error.view().title, "This file is not UTF-8");
        assert_eq!(error.view().path, invalid);
        assert_eq!(
            model.workspace.as_ref().unwrap().root.path,
            root.path().canonicalize().unwrap()
        );
        assert_eq!(model.tabs.len(), 1);
        assert_eq!(model.tabs.active().unwrap().document.title, "Home");
    }

    #[test]
    fn opening_multiple_paths_reports_the_first_error_but_keeps_successes() {
        let dir = tempfile::tempdir().unwrap();
        let unsupported = dir.path().join("notes.txt");
        let first = dir.path().join("first.md");
        let second = dir.path().join("second.md");
        fs::write(&unsupported, "not markdown").unwrap();
        fs::write(&first, "# First").unwrap();
        fs::write(&second, "# Second").unwrap();
        let mut model = AppModel::default();

        let result = model.open_paths([unsupported.as_path(), first.as_path(), second.as_path()]);
        let error = result.document_error.as_ref().unwrap();

        assert_eq!(error.title, "Unsupported file type");
        assert_eq!(error.path, unsupported);
        assert!(result.workspace_error.is_none());
        assert_eq!(
            model.tabs.paths().collect::<Vec<_>>(),
            vec![
                first.canonicalize().unwrap(),
                second.canonicalize().unwrap()
            ]
        );
        assert_eq!(
            model.tabs.active().unwrap().path(),
            second.canonicalize().unwrap()
        );
        assert!(model.workspace.is_none());
    }

    #[test]
    fn opening_a_missing_path_exposes_readable_copy_without_debug_formatting() {
        let missing = Path::new("/tmp/mdow-task-5-missing.md");
        let mut model = AppModel::default();

        let error = model.open_path(missing).unwrap_err();

        assert!(matches!(error, AppOpenError::Document(_)));
        assert_eq!(error.view().title, "File not found");
        assert_eq!(
            error.view().body,
            "This file may have been moved or renamed."
        );
        assert_eq!(error.view().path, missing);
        assert!(!error.view().body.contains("DocumentError"));
    }

    #[test]
    fn workspace_failure_uses_sidebar_error_without_replacing_workspace_or_tabs() {
        let root = markdown_workspace();
        let file = root.path().join("README.md");
        let missing = root.path().join("missing-folder");
        let mut model = AppModel::default();
        model.open_path(root.path()).unwrap();
        model.open_path(&file).unwrap();
        let workspace_path = model.workspace.as_ref().unwrap().root.path.clone();

        let error = model.open_workspace(&missing).unwrap_err();

        assert!(matches!(error, AppOpenError::Workspace(_)));
        assert_eq!(error.view().path, missing);
        assert_eq!(model.workspace.as_ref().unwrap().root.path, workspace_path);
        assert_eq!(model.tabs.len(), 1);
        assert_eq!(model.workspace_error.as_ref(), Some(error.view()));
    }

    #[test]
    fn nested_unreadable_directory_is_skipped_without_a_sidebar_error() {
        let root = markdown_workspace();
        let file = root.path().join("README.md");
        let partial = tempfile::tempdir().unwrap();
        let denied = partial.path().join("denied");
        fs::create_dir(&denied).unwrap();
        fs::write(denied.join("hidden.md"), "# Hidden").unwrap();
        fs::write(partial.path().join("visible.md"), "# Visible").unwrap();
        let _restore = PermissionRestore::deny(&denied);
        let mut model = AppModel::default();
        model.open_workspace(root.path()).unwrap();
        model.open_document(&file).unwrap();

        model.open_workspace(partial.path()).unwrap();

        let workspace = model.workspace.as_ref().unwrap();
        assert_eq!(workspace.root.path, partial.path().canonicalize().unwrap());
        assert_eq!(
            workspace.files(),
            vec![partial.path().join("visible.md").canonicalize().unwrap()]
        );
        assert!(model.workspace_error.is_none());
        assert_eq!(model.tabs.len(), 1);
    }

    #[test]
    fn successful_workspace_open_clears_only_the_sidebar_error() {
        let first = markdown_workspace();
        let second = markdown_workspace();
        let missing = first.path().join("missing-folder");
        let mut model = AppModel::default();

        model.open_workspace(&missing).unwrap_err();
        assert!(model.workspace_error.is_some());

        model.open_workspace(second.path()).unwrap();

        assert!(model.workspace_error.is_none());
        assert_eq!(
            model.workspace.as_ref().unwrap().root.path,
            second.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn batch_keeps_first_workspace_failure_when_a_later_workspace_succeeds() {
        let bad = markdown_workspace();
        let good = markdown_workspace();
        let bad_path = bad.path().canonicalize().unwrap();
        fs::set_permissions(bad.path(), fs::Permissions::from_mode(0o000)).unwrap();
        let mut model = AppModel::default();

        let result = model.open_paths([bad.path(), good.path()]);
        fs::set_permissions(bad.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let error = result.workspace_error.as_ref().unwrap();

        assert_eq!(error.path, bad_path);
        assert_eq!(model.workspace_error.as_ref(), Some(error));
        assert!(result.document_error.is_none());
        assert_eq!(
            model.workspace.as_ref().unwrap().root.path,
            good.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn batch_keeps_the_first_of_two_workspace_failures() {
        let first = markdown_workspace();
        let second = markdown_workspace();
        let first_path = first.path().canonicalize().unwrap();
        fs::set_permissions(first.path(), fs::Permissions::from_mode(0o000)).unwrap();
        fs::set_permissions(second.path(), fs::Permissions::from_mode(0o000)).unwrap();
        let mut model = AppModel::default();

        let result = model.open_paths([first.path(), second.path()]);
        fs::set_permissions(first.path(), fs::Permissions::from_mode(0o700)).unwrap();
        fs::set_permissions(second.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let error = result.workspace_error.as_ref().unwrap();

        assert_eq!(error.path, first_path);
        assert_eq!(model.workspace_error.as_ref(), Some(error));
        assert!(result.document_error.is_none());
        assert!(model.workspace.is_none());
    }

    #[test]
    fn drop_state_tracks_enter_leave_and_drop_idempotently() {
        let mut state = DropState::default();

        assert!(!state.is_active());
        assert!(state.enter());
        assert!(state.is_active());
        assert!(!state.enter());
        assert!(state.leave());
        assert!(!state.is_active());
        assert!(!state.leave());
        assert!(state.enter());
        assert!(state.dropped());
        assert!(!state.is_active());
    }

    #[test]
    fn reader_key_targets_are_clamped_to_scroll_extent() {
        assert_eq!(reader_key_target("home", -240.0, 600.0, 1600.0), Some(0.0));
        assert_eq!(
            reader_key_target("end", -240.0, 600.0, 1600.0),
            Some(-1600.0)
        );
        assert_eq!(
            reader_key_target("pagedown", -240.0, 600.0, 1600.0),
            Some(-780.0)
        );
        assert_eq!(
            reader_key_target("pageup", -240.0, 600.0, 1600.0),
            Some(0.0)
        );
        assert_eq!(
            reader_key_target("down", -240.0, 600.0, 1600.0),
            Some(-280.0)
        );
        assert_eq!(reader_key_target("up", -20.0, 600.0, 1600.0), Some(0.0));
        assert_eq!(
            reader_key_target("down", -1590.0, 600.0, 1600.0),
            Some(-1600.0)
        );
        assert_eq!(reader_scroll_key("space", false), "pagedown");
        assert_eq!(reader_scroll_key("space", true), "pageup");
        assert_eq!(reader_scroll_key("down", true), "down");
    }

    fn long_reader_window(cx: &mut TestAppContext) -> gpui::WindowHandle<MdowApp> {
        document_window(
            cx,
            &(0..80)
                .map(|index| format!("Paragraph {index} keeps the native reader overflowing."))
                .collect::<Vec<_>>()
                .join("\n\n"),
        )
    }

    fn active_reader_offset(
        window: gpui::WindowHandle<MdowApp>,
        visual: &mut VisualTestContext,
    ) -> Pixels {
        window
            .update(visual, |app, _, cx| {
                app.reader_list_state(app.model.tabs.active().unwrap().path(), cx)
                    .unwrap()
                    .scroll_px_offset_for_scrollbar()
                    .y
            })
            .unwrap()
    }

    #[gpui::test]
    fn arrow_and_space_keys_scroll_the_reader(cx: &mut TestAppContext) {
        let window = long_reader_window(cx);
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.draw(cx).clear());
        visual.update(|window, cx| window.draw(cx).clear());

        visual.simulate_keystrokes("down");
        assert_eq!(active_reader_offset(window, &mut visual), px(-40.0));
        visual.simulate_keystrokes("up");
        assert_eq!(active_reader_offset(window, &mut visual), px(0.0));
        visual.simulate_keystrokes("space");
        let paged = active_reader_offset(window, &mut visual);
        assert!(paged < px(-40.0), "space should page down, got {paged:?}");
        visual.simulate_keystrokes("shift-space");
        assert_eq!(active_reader_offset(window, &mut visual), px(0.0));
    }

    #[gpui::test]
    fn reader_keys_leave_text_fields_and_focused_controls_alone(cx: &mut TestAppContext) {
        let window = long_reader_window(cx);
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.draw(cx).clear());
        visual.update(|window, cx| window.draw(cx).clear());

        visual.dispatch_action(ToggleFind);
        visual.update(|window, cx| window.draw(cx).clear());
        visual.simulate_keystrokes("space down");
        assert_eq!(active_reader_offset(window, &mut visual), px(0.0));
        window
            .update(&mut visual, |app, _, cx| {
                let find = app.overlays.find().unwrap().read(cx);
                assert_eq!(find.query_text(cx), " ");
            })
            .unwrap();

        visual.dispatch_action(Dismiss);
        // Focus the titlebar sidebar toggle: Space must activate it rather than page the reader.
        focus_next(&mut visual, 1);
        visual.simulate_keystrokes("space");
        activate_focused(&mut visual, "space");
        assert_eq!(active_reader_offset(window, &mut visual), px(0.0));
        window
            .update(&mut visual, |app, _, _| assert!(!app.sidebar_open))
            .unwrap();
    }

    #[test]
    fn reader_navigation_accepts_the_macos_function_modifier() {
        assert!(reader_key_modifiers_are_allowed(
            "end", false, false, false, true,
        ));
        assert!(!reader_key_modifiers_are_allowed(
            "tab", false, false, false, true,
        ));
        assert!(!reader_key_modifiers_are_allowed(
            "end", false, false, true, true,
        ));
    }

    #[gpui::test]
    fn open_paths_registers_live_reload_without_changing_tab_or_scroll_state(
        cx: &mut TestAppContext,
    ) {
        let dir = watcher_workspace();
        let first = dir.path().join("first.md");
        let second = dir.path().join("second.md");
        fs::write(
            &first,
            format!(
                "# First\n\n{}",
                "A paragraph for scrolling.\n\n".repeat(200)
            ),
        )
        .unwrap();
        fs::write(&second, "# Second").unwrap();
        let first = first.canonicalize().unwrap();
        let second = second.canonicalize().unwrap();
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| MdowApp::new(window, cx))
            })
            .unwrap()
        });
        window
            .update(cx, |app, _, cx| {
                app.open_paths([first.clone(), second.clone()], cx);
                app.activate_tab(&first, cx);
            })
            .unwrap();
        cx.run_until_parked();
        {
            let mut visual = VisualTestContext::from_window(*window, cx);
            visual.update(|window, cx| window.draw(cx).clear());
        }
        let scroll_handle = window
            .update(cx, |app, _, cx| app.reader_list_state(&first, cx).unwrap())
            .unwrap();
        let before = window
            .update(cx, |app, _, _| {
                app.model
                    .tabs
                    .paths()
                    .map(Path::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap();

        fs::write(
            &first,
            format!(
                "# Reloaded\n\n{}",
                "A paragraph for scrolling.\n\n".repeat(200)
            ),
        )
        .unwrap();
        std::thread::sleep(Duration::from_millis(350));
        cx.executor().advance_clock(Duration::from_millis(100));
        cx.run_until_parked();

        window
            .update(cx, |app, _, cx| {
                assert_eq!(
                    app.model
                        .tabs
                        .paths()
                        .map(Path::to_owned)
                        .collect::<Vec<_>>(),
                    before
                );
                assert_eq!(app.model.tabs.active().unwrap().path(), first);
                assert_eq!(
                    app.model.tabs.get(&first).unwrap().document.title,
                    "Reloaded"
                );
                app.reader_list_state(&first, cx)
                    .unwrap()
                    .set_offset_from_scrollbar(point(px(0.0), px(-64.0)));
            })
            .unwrap();
        assert_eq!(scroll_handle.scroll_px_offset_for_scrollbar().y, px(-64.0));
    }

    #[gpui::test]
    fn missing_file_watcher_degrades_to_opening_without_live_reload(cx: &mut TestAppContext) {
        let root = markdown_workspace();
        let path = root.path().join("README.md");
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    MdowApp::boot_with_watcher(
                        Prefs::default(),
                        StateStore::in_memory(),
                        SessionRole::Owner,
                        Err(anyhow::anyhow!("watcher unavailable")),
                        window,
                        cx,
                    )
                })
            })
            .unwrap()
        });

        window
            .update(cx, |app, _, cx| {
                app.open_path(&path, cx);
                app.open_paths([root.path().join("guides/start.md")], cx);
            })
            .unwrap();

        window
            .update(cx, |app, _, _| {
                assert!(app.file_watcher.is_none());
                assert_eq!(app.model.tabs.len(), 2);
                assert!(app.open_error.is_none());
            })
            .unwrap();
    }

    #[gpui::test]
    fn batch_open_watches_valid_files_after_a_stale_tab_watch_failure(cx: &mut TestAppContext) {
        let dir = watcher_workspace();
        let stale = dir.path().join("stale.md");
        let valid = dir.path().join("valid.md");
        fs::write(&stale, "# Stale").unwrap();
        fs::write(&valid, "# Valid").unwrap();
        let stale = stale.canonicalize().unwrap();
        let valid = valid.canonicalize().unwrap();
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| MdowApp::new(window, cx))
            })
            .unwrap()
        });
        window
            .update(cx, |app, _, cx| app.open_path(&stale, cx))
            .unwrap();
        fs::remove_file(&stale).unwrap();

        window
            .update(cx, |app, _, cx| app.open_paths([valid.clone()], cx))
            .unwrap();
        cx.run_until_parked();
        fs::write(&valid, "# Watched").unwrap();
        std::thread::sleep(Duration::from_millis(350));
        cx.executor().advance_clock(Duration::from_millis(100));
        cx.run_until_parked();

        window
            .update(cx, |app, _, _| {
                assert_eq!(
                    app.model.tabs.get(&valid).unwrap().document.title,
                    "Watched"
                );
            })
            .unwrap();
    }

    fn two_tab_window(
        cx: &mut TestAppContext,
    ) -> (
        gpui::WindowHandle<MdowApp>,
        PathBuf,
        PathBuf,
        tempfile::TempDir,
    ) {
        let root = markdown_workspace();
        let first = root.path().join("README.md");
        let second = root.path().join("guides/start.md");
        let mut model = AppModel::default();
        model.open_document(&first).unwrap();
        model.open_document(&second).unwrap();
        model.tabs.activate(&first.canonicalize().unwrap());
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model = model;
                    app.open_error = None;
                    app
                })
            })
            .unwrap()
        });
        (
            window,
            first.canonicalize().unwrap(),
            second.canonicalize().unwrap(),
            root,
        )
    }

    #[gpui::test]
    fn window_title_tracks_the_active_document_and_falls_back_when_empty(cx: &mut TestAppContext) {
        let (window, first, second, _root) = two_tab_window(cx);
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.draw(cx).clear());
        assert_eq!(visual.window_title().as_deref(), Some("README.md"));

        window
            .update(cx, |app, _, cx| app.activate_tab(&second, cx))
            .unwrap();
        visual.update(|window, cx| window.draw(cx).clear());
        assert_eq!(visual.window_title().as_deref(), Some("start.md"));

        window
            .update(cx, |app, _, cx| {
                app.close_tab(&second, cx);
                app.close_tab(&first, cx);
            })
            .unwrap();
        visual.update(|window, cx| window.draw(cx).clear());
        assert_eq!(visual.window_title().as_deref(), Some(DEFAULT_WINDOW_TITLE));
    }

    fn three_tab_window(cx: &mut TestAppContext) -> (gpui::WindowHandle<MdowApp>, Vec<PathBuf>) {
        let paths = ["a", "b", "c"]
            .map(|name| PathBuf::from(format!("/tmp/tab-nav-{name}.md")))
            .to_vec();
        let mut model = AppModel::default();
        for path in &paths {
            model
                .tabs
                .open(parse_document(path.clone(), "# Tab\n".into()));
        }
        model.tabs.activate(&paths[0]);
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model = model;
                    app
                })
            })
            .unwrap()
        });
        (window, paths)
    }

    #[gpui::test]
    fn tab_actions_select_by_number_and_cycle_with_wraparound(cx: &mut TestAppContext) {
        let (window, paths) = three_tab_window(cx);
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.draw(cx).clear());
        let active = |visual: &mut VisualTestContext| {
            window
                .update(visual, |app, _, _| {
                    app.model.tabs.active().unwrap().path().to_owned()
                })
                .unwrap()
        };

        visual.dispatch_action(SelectLastTab);
        assert_eq!(active(&mut visual), paths[2]);
        visual.dispatch_action(NextTab);
        assert_eq!(active(&mut visual), paths[0]);
        visual.dispatch_action(PreviousTab);
        assert_eq!(active(&mut visual), paths[2]);
        visual.dispatch_action(SelectTab2);
        assert_eq!(active(&mut visual), paths[1]);
        visual.dispatch_action(SelectTab8);
        assert_eq!(active(&mut visual), paths[1]);
    }

    #[gpui::test]
    fn tab_actions_are_ignored_without_tabs(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| MdowApp::new(window, cx))
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.draw(cx).clear());

        visual.dispatch_action(NextTab);
        visual.dispatch_action(SelectLastTab);

        window
            .update(cx, |app, _, _| assert!(app.model.tabs.is_empty()))
            .unwrap();
    }

    #[gpui::test]
    fn middle_clicking_a_tab_closes_it(cx: &mut TestAppContext) {
        let (window, first, second, _root) = two_tab_window(cx);
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.draw(cx).clear());
        let center = visual
            .debug_bounds("document-tab-1")
            .expect("second tab should be painted")
            .center();

        visual.simulate_mouse_down(center, MouseButton::Middle, Modifiers::none());
        visual.simulate_mouse_up(center, MouseButton::Middle, Modifiers::none());

        window
            .update(cx, |app, _, _| {
                assert_eq!(app.model.tabs.len(), 1);
                assert!(app.model.tabs.get(&second).is_none());
                assert_eq!(app.model.tabs.active().unwrap().path(), first);
            })
            .unwrap();
    }

    fn recents_only_window(
        cx: &mut TestAppContext,
    ) -> (gpui::WindowHandle<MdowApp>, PathBuf, tempfile::TempDir) {
        let root = markdown_workspace();
        let recent = root.path().join("README.md").canonicalize().unwrap();
        let recents = Recents::from_paths(vec![recent.clone()]);
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model.recents = recents;
                    app
                })
            })
            .unwrap()
        });
        (window, recent, root)
    }

    #[gpui::test]
    fn sidebar_recent_row_is_reachable_and_activatable_by_keyboard(cx: &mut TestAppContext) {
        let (window, recent, _root) = recents_only_window(cx);
        let mut visual = VisualTestContext::from_window(*window, cx);

        // Sidebar toggle, Recents, Folder, Outline, Clear, then the first recent row.
        focus_next(&mut visual, 6);
        activate_focused(&mut visual, "enter");

        window
            .update(cx, |app, _, _| {
                assert_eq!(app.model.tabs.active().unwrap().path(), recent);
            })
            .unwrap();
    }

    #[gpui::test]
    fn welcome_recent_row_is_reachable_and_activatable_by_keyboard(cx: &mut TestAppContext) {
        let (window, recent, _root) = recents_only_window(cx);
        let mut visual = VisualTestContext::from_window(*window, cx);

        // Sidebar toggle, sidebar modes, Clear, sidebar recent, Settings, palette, Open File,
        // Open Folder, then the first welcome recent.
        focus_next(&mut visual, 11);
        activate_focused(&mut visual, "space");

        window
            .update(cx, |app, _, _| {
                assert_eq!(app.model.tabs.active().unwrap().path(), recent);
            })
            .unwrap();
    }

    #[gpui::test]
    fn tab_close_target_is_reachable_and_activatable_by_keyboard(cx: &mut TestAppContext) {
        let (window, _first, second, _root) = two_tab_window(cx);
        let mut visual = VisualTestContext::from_window(*window, cx);

        // Sidebar toggle, Recents, Folder, Outline, Clear, both recent rows, Settings, both tabs,
        // find, palette, split view, wide-mode, then the first tab's nested close target.
        focus_next(&mut visual, 15);
        activate_focused(&mut visual, "space");

        window
            .update(cx, |app, _, _| {
                assert_eq!(app.model.tabs.len(), 1);
                assert_eq!(app.model.tabs.active().unwrap().path(), second);
            })
            .unwrap();
    }

    #[gpui::test]
    fn inactive_tab_is_reachable_and_activatable_by_keyboard(cx: &mut TestAppContext) {
        let (window, _first, second, _root) = two_tab_window(cx);
        let mut visual = VisualTestContext::from_window(*window, cx);

        // Sidebar toggle, Recents, Folder, Outline, Clear, both recent rows, Settings, first tab,
        // then second tab.
        focus_next(&mut visual, 10);
        activate_focused(&mut visual, "enter");

        window
            .update(cx, |app, _, _| {
                assert_eq!(app.model.tabs.len(), 2);
                assert_eq!(app.model.tabs.active().unwrap().path(), second);
            })
            .unwrap();
    }

    #[gpui::test]
    fn shell_folds_the_titlebar_into_the_sidebar_header_and_tab_row(cx: &mut TestAppContext) {
        let document = parse_document(
            PathBuf::from("/tmp/measured-tab.md"),
            "# Measured tab\n".into(),
        );
        let window = cx.update(|cx| {
            cx.open_window(
                WindowOptions {
                    titlebar: Some(TitlebarOptions {
                        title: Some("Mdow Native".into()),
                        appears_transparent: true,
                        traffic_light_position: Some(TrafficLights::position()),
                    }),
                    ..Default::default()
                },
                |window, cx| {
                    cx.new(|cx| {
                        let mut app = MdowApp::new(window, cx);
                        app.model.tabs.open(document);
                        app.open_error = None;
                        app
                    })
                },
            )
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.draw(cx).clear());

        // Sidebar open: the sidebar runs to the top and its header holds the toggle.
        assert!(visual.debug_bounds("titlebar").is_none());
        let sidebar = visual.debug_bounds("sidebar").expect("sidebar");
        let header = visual
            .debug_bounds("sidebar-header")
            .expect("sidebar header");
        let toggle = visual
            .debug_bounds("toggle-sidebar")
            .expect("sidebar toggle");
        let tab_bar = visual.debug_bounds("tab-bar").expect("tab bar");
        let tabs = visual.debug_bounds("tabs-scroll").expect("tab list");
        let tab = visual
            .debug_bounds("document-tab-0")
            .expect("first document tab");
        let breadcrumb = visual.debug_bounds("breadcrumb").expect("breadcrumb");
        let reader = visual.debug_bounds("reader-scroll").expect("reader");
        assert_eq!(sidebar.top(), px(0.0));
        assert_eq!(header.size.height, px(TrafficLights::titlebar_height()));
        assert!(visual.debug_bounds("titlebar-sidebar-toggle").is_none());
        assert_eq!(
            toggle.right(),
            sidebar.right() - px(1.0) - px(Metrics::SIDEBAR_HEADER_END_INSET)
        );
        assert!(toggle.left() > px(TrafficLightClearance::reserved().width()));
        assert_eq!(tab_bar.top(), px(0.0));
        assert_eq!(tab_bar.size.height, px(TrafficLights::titlebar_height()));
        assert_eq!(tab_bar.left(), sidebar.right());
        assert_eq!(tabs.origin.x, tab_bar.origin.x);
        assert_eq!(tab.origin.x, tabs.origin.x + px(Metrics::TAB_LIST_INSET));
        assert_eq!(tab.size.height, px(28.0));
        assert_eq!(tab.top() - tab_bar.top(), px(6.0));
        assert_eq!(breadcrumb.top(), px(40.0));
        assert_eq!(reader.top(), px(68.0));
        assert!(visual.debug_bounds("toggle-settings").is_none());
        assert!(visual.debug_bounds("toggle-find").is_some());
        assert!(visual.debug_bounds("toggle-palette").is_some());

        // Sidebar hidden: the tab row reserves the traffic lights, then the toggle.
        window
            .update(cx, |app, _, cx| app.click_toggle_sidebar(cx))
            .unwrap();
        visual.update(|window, cx| window.draw(cx).clear());
        let toggle = visual
            .debug_bounds("titlebar-sidebar-toggle")
            .expect("tab-row sidebar toggle");
        let tab_bar = visual.debug_bounds("tab-bar").expect("tab bar");
        let tabs = visual.debug_bounds("tabs-scroll").expect("tab list");
        assert_eq!(tab_bar.left(), px(0.0));
        assert_eq!(
            toggle.origin.x,
            px(TrafficLightClearance::reserved().width())
        );
        assert_eq!(
            tabs.origin.x,
            px(TrafficLightClearance::reserved().width() + Metrics::TITLEBAR_BUTTON)
        );
        let reader = visual.debug_bounds("reader-scroll").expect("reader");
        assert_eq!(reader.top(), px(68.0));
    }

    #[gpui::test]
    fn interface_scale_resizes_the_shell_chrome(cx: &mut TestAppContext) {
        use crate::prefs::InterfaceScale;

        // (scale, sidebar, titlebar row, tab, breadcrumb, icon button)
        let cases = [
            (InterfaceScale::Compact, 244.0, 40.0, 28.0, 28.0, 28.0),
            (InterfaceScale::Comfortable, 264.0, 40.0, 32.0, 32.0, 32.0),
            (InterfaceScale::Large, 280.0, 44.0, 36.0, 36.0, 36.0),
        ];
        for (scale, sidebar_width, row, tab_height, breadcrumb_height, button) in cases {
            let document =
                parse_document(PathBuf::from("/tmp/scaled-tab.md"), "# Scaled tab\n".into());
            let window = cx.update(|cx| {
                cx.open_window(Default::default(), |window, cx| {
                    cx.new(|cx| {
                        let mut app = MdowApp::new(window, cx);
                        app.model.tabs.open(document);
                        app.apply_pref(PrefEdit::InterfaceScale(scale), cx);
                        app
                    })
                })
                .unwrap()
            });
            let mut visual = VisualTestContext::from_window(*window, cx);
            visual.update(|window, cx| window.draw(cx).clear());

            let sidebar = visual.debug_bounds("sidebar").expect("sidebar");
            let header = visual.debug_bounds("sidebar-header").expect("header");
            let toggle = visual.debug_bounds("toggle-sidebar").expect("toggle");
            let modes = visual.debug_bounds("sidebar-modes").expect("modes");
            let tab_bar = visual.debug_bounds("tab-bar").expect("tab bar");
            let tab = visual.debug_bounds("document-tab-0").expect("tab");
            let find = visual.debug_bounds("toggle-find").expect("find");
            let breadcrumb = visual.debug_bounds("breadcrumb").expect("breadcrumb");
            let reader = visual.debug_bounds("reader-scroll").expect("reader");

            assert_eq!(sidebar.size.width, px(sidebar_width), "{scale:?}");
            assert_eq!(header.size.height, px(row), "{scale:?}");
            assert_eq!(tab_bar.size.height, px(row), "{scale:?}");
            assert_eq!(tab.size.height, px(tab_height), "{scale:?}");
            assert_eq!(modes.size.height, px(button), "{scale:?}");
            assert_eq!(toggle.size.height, px(button), "{scale:?}");
            assert_eq!(find.size.height, px(button), "{scale:?}");
            // Titlebar controls sit in the vertical centre of the scaled row.
            for control in [tab, toggle, find] {
                assert_eq!(
                    control.center().y,
                    tab_bar.center().y,
                    "{scale:?} control off-centre"
                );
            }
            assert_eq!(breadcrumb.top(), px(row), "{scale:?}");
            assert_eq!(breadcrumb.size.height, px(breadcrumb_height), "{scale:?}");
            assert_eq!(reader.top(), px(row + breadcrumb_height), "{scale:?}");
        }
    }

    #[gpui::test]
    fn disclosure_click_and_keyboard_activation_toggle_once_each(cx: &mut TestAppContext) {
        let root = markdown_workspace();
        let mut model = AppModel::default();
        model.open_workspace(root.path()).unwrap();
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model = model;
                    app.open_error = None;
                    app.set_sidebar_mode(SidebarMode::Folder, cx);
                    app
                })
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);

        // Recents, Folder, Outline, open-folder, then later the nested first-row disclosure.
        focus_next(&mut visual, 13);
        activate_focused(&mut visual, "space");
        window
            .update(cx, |app, _, _| {
                assert!(app.model.workspace.as_ref().unwrap().visible_rows()[0].expanded);
            })
            .unwrap();

        click_debug(&mut visual, "workspace-disclosure-0");

        window
            .update(cx, |app, _, _| {
                assert!(!app.model.workspace.as_ref().unwrap().visible_rows()[0].expanded);
            })
            .unwrap();
    }

    #[gpui::test]
    fn external_drag_enter_and_exit_update_the_rendered_drop_state(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| MdowApp::new(window, cx))
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.draw(cx).clear());

        visual.simulate_event(FileDropEvent::Entered {
            position: point(px(12.0), px(12.0)),
            paths: ExternalPaths::default(),
        });

        window
            .update(cx, |app, _, _| assert!(app.drop_state.is_active()))
            .unwrap();
        visual.run_until_parked();

        visual.simulate_event(FileDropEvent::Exited);
        std::thread::sleep(Duration::from_millis(20));
        visual.run_until_parked();

        window
            .update(cx, |app, _, _| assert!(!app.drop_state.is_active()))
            .unwrap();
    }

    #[gpui::test]
    fn folder_failure_preserves_main_error_workspace_and_tabs(cx: &mut TestAppContext) {
        let root = markdown_workspace();
        let missing = root.path().join("missing-folder");
        let file = root.path().join("README.md");
        let main_error = UserFacingError {
            title: "File error".into(),
            body: "Keep this in the document surface.".into(),
            path: file.clone(),
        };
        let mut model = AppModel::default();
        model.open_workspace(root.path()).unwrap();
        model.open_document(&file).unwrap();
        let workspace_path = model.workspace.as_ref().unwrap().root.path.clone();
        let expected_main_error = main_error.clone();
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model = model;
                    app.open_error = Some(main_error);
                    app
                })
            })
            .unwrap()
        });

        window
            .update(cx, |app, _, cx| app.open_workspace_path(&missing, cx))
            .unwrap();
        cx.run_until_parked();

        window
            .update(cx, |app, _, _| {
                assert_eq!(app.open_error.as_ref(), Some(&expected_main_error));
                assert_eq!(app.model.workspace_error.as_ref().unwrap().path, missing);
                assert_eq!(
                    app.model.workspace.as_ref().unwrap().root.path,
                    workspace_path
                );
                assert_eq!(app.model.tabs.len(), 1);
            })
            .unwrap();
    }

    #[gpui::test]
    fn reader_column_is_centered_in_a_wide_viewport(cx: &mut TestAppContext) {
        let document = parse_document(
            PathBuf::from("/tmp/reader-centered.md"),
            "# Centered\n\nA short paragraph.".into(),
        );
        let window = cx.update(|cx| {
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(gpui::WindowBounds::Windowed(gpui::Bounds::new(
                        point(px(0.0), px(0.0)),
                        gpui::size(px(1600.0), px(900.0)),
                    ))),
                    ..Default::default()
                },
                |window, cx| {
                    cx.new(|cx| {
                        let mut app = MdowApp::new(window, cx);
                        app.model.tabs.open(document);
                        app.open_error = None;
                        app
                    })
                },
            )
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.draw(cx).clear());

        let viewport = visual
            .debug_bounds("reader-scroll")
            .expect("reader viewport");
        let column = visual.debug_bounds("reader-column").expect("reader column");
        let left_gap = column.origin.x - viewport.origin.x;
        let right_gap =
            (viewport.origin.x + viewport.size.width) - (column.origin.x + column.size.width);

        assert!(
            column.size.width < viewport.size.width - px(200.0),
            "column {:?} should be narrower than viewport {:?}",
            column.size.width,
            viewport.size.width,
        );
        assert!(
            (f32::from(left_gap) - f32::from(right_gap)).abs() <= 16.0,
            "column should be centered: left gap {left_gap:?}, right gap {right_gap:?}",
        );
    }

    #[gpui::test]
    fn active_document_renders_one_scroll_surface_with_wrapping_inline_text(
        cx: &mut TestAppContext,
    ) {
        let document = parse_document(
            PathBuf::from("/tmp/reader-contract.md"),
            format!(
                "# Reader\n\nThis paragraph has *emphasis*, **strong text**, `inline code`, and [a local link](next.md). {}",
                "A deliberately long sentence keeps flowing through the same inline text surface. "
                    .repeat(48),
            ),
        );
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model.tabs.open(document);
                    app.open_error = None;
                    app
                })
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.draw(cx).clear());

        let bounds = visual
            .debug_bounds("reader-scroll")
            .expect("reader viewport");
        let column_bounds = visual.debug_bounds("reader-column").expect("reader column");
        assert!(visual.debug_bounds("reader-block-0").is_some());
        let paragraph = visual
            .debug_bounds("reader-inline-1-0")
            .expect("paragraph inline surface should be painted");
        assert!(paragraph.size.height > px(40.0));
        let handle = window
            .update(cx, |app, _, cx| {
                app.reader_list_state(app.model.tabs.active().unwrap().path(), cx)
                    .unwrap()
            })
            .unwrap();

        assert!(
            handle.max_offset_for_scrollbar().height > px(0.0),
            "reader viewport height {:?}, column height {:?}, max offset {:?}",
            bounds.size.height,
            column_bounds.size.height,
            handle.max_offset_for_scrollbar().height,
        );
        visual.simulate_event(ScrollWheelEvent {
            position: bounds.center(),
            delta: ScrollDelta::Pixels(point(px(0.0), px(-180.0))),
            ..Default::default()
        });
        visual.update(|window, cx| window.draw(cx).clear());

        assert!(handle.scroll_px_offset_for_scrollbar().y < px(0.0));
    }

    #[gpui::test]
    fn long_reader_exposes_a_thumb_whose_literal_drag_changes_the_active_offset(
        cx: &mut TestAppContext,
    ) {
        let document = parse_document(
            PathBuf::from("/tmp/reader-scrollbar.md"),
            (0..80)
                .map(|index| format!("Paragraph {index} keeps the native reader overflowing."))
                .collect::<Vec<_>>()
                .join("\n\n"),
        );
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model.tabs.open(document);
                    app.open_error = None;
                    app
                })
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.draw(cx).clear());
        visual.update(|window, cx| window.draw(cx).clear());

        let track = visual
            .debug_bounds("reader-scrollbar-track")
            .expect("overflowing reader should expose a scrollbar track");
        let thumb = visual
            .debug_bounds("reader-scrollbar-thumb")
            .expect("overflowing reader should expose a draggable thumb");
        assert!(track.contains(&thumb.center()));
        assert!(thumb.size.height < track.size.height);
        let handle = window
            .update(cx, |app, _, cx| {
                app.reader_list_state(app.model.tabs.active().unwrap().path(), cx)
                    .unwrap()
            })
            .unwrap();
        let before = handle.scroll_px_offset_for_scrollbar().y;
        let drag_to = point(thumb.center().x, thumb.center().y + px(120.0));

        visual.simulate_mouse_move(thumb.center(), None, Modifiers::none());
        visual.simulate_mouse_down(thumb.center(), MouseButton::Left, Modifiers::none());
        visual.simulate_mouse_move(drag_to, MouseButton::Left, Modifiers::none());
        visual.update(|window, cx| window.draw(cx).clear());
        visual.simulate_mouse_up(drag_to, MouseButton::Left, Modifiers::none());

        assert!(handle.scroll_px_offset_for_scrollbar().y < before);
    }

    #[gpui::test]
    fn clicking_the_reader_scrollbar_track_moves_the_active_offset(cx: &mut TestAppContext) {
        let document = parse_document(
            PathBuf::from("/tmp/reader-scrollbar-track.md"),
            (0..80)
                .map(|index| format!("Paragraph {index} keeps the native reader overflowing."))
                .collect::<Vec<_>>()
                .join("\n\n"),
        );
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model.tabs.open(document);
                    app.open_error = None;
                    app
                })
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.draw(cx).clear());
        visual.update(|window, cx| window.draw(cx).clear());

        let track = visual
            .debug_bounds("reader-scrollbar-track")
            .expect("overflowing reader should expose a scrollbar track");
        let handle = window
            .update(cx, |app, _, cx| {
                app.reader_list_state(app.model.tabs.active().unwrap().path(), cx)
                    .unwrap()
            })
            .unwrap();
        let click = point(track.center().x, track.bottom() - px(8.0));

        visual.simulate_click(click, Modifiers::none());
        visual.update(|window, cx| window.draw(cx).clear());

        assert!(handle.scroll_px_offset_for_scrollbar().y < px(0.0));
    }

    #[gpui::test]
    fn multi_block_list_item_renders_one_marker_and_indented_children_in_source_order(
        cx: &mut TestAppContext,
    ) {
        let document = prepare_document(parse_document(
            PathBuf::from("/tmp/multi-block-list.md"),
            "- before\n\n  ```rust\n  let n = 1;\n  ```\n\n  after\n".into(),
        ));
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model.tabs.open_prepared(document);
                    app.open_error = None;
                    app
                })
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.draw(cx).clear());

        let marker = visual
            .debug_bounds("reader-list-marker-0")
            .expect("one outer list marker");
        let leading = visual
            .debug_bounds("reader-list-child-0-0")
            .expect("leading paragraph child");
        let code = visual
            .debug_bounds("reader-list-child-0-1")
            .expect("nested fenced-code child");
        let trailing = visual
            .debug_bounds("reader-list-child-0-2")
            .expect("trailing paragraph child");

        assert!(marker.right() < leading.left());
        assert_eq!(leading.left(), code.left());
        assert_eq!(code.left(), trailing.left());
        assert!(leading.top() < code.top());
        assert!(code.top() < trailing.top());
        assert!(visual.debug_bounds("reader-code-0-1").is_some());
        assert!(visual.debug_bounds("reader-list-marker-0-1").is_none());
    }

    #[gpui::test]
    fn reader_bounds_match_markdown_css(cx: &mut TestAppContext) {
        let document = prepare_document(parse_document(
            PathBuf::from("/tmp/showcase.md"),
            include_str!("../tests/fixtures/showcase.md").into(),
        ));
        let wide_code_index = document
            .blocks
            .iter()
            .position(|block| {
                matches!(
                    block,
                    crate::document::DocumentBlock::CodeBlock { code, .. }
                        if code.contains("deliberately_long_code_line")
                )
            })
            .expect("wide code block in showcase");
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model.tabs.open_prepared(document);
                    app.open_error = None;
                    app
                })
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.draw(cx).clear());

        let scroll = visual
            .debug_bounds("reader-scroll")
            .expect("reader viewport");
        let first_block = visual
            .debug_bounds("reader-block-0")
            .expect("first reader block");
        let tab_bar = visual.debug_bounds("tab-bar").expect("tab bar");
        let tab = visual.debug_bounds("document-tab-0").expect("active tab");
        assert_eq!(first_block.top() - scroll.top(), px(32.0));
        assert_eq!(
            tab.top() - tab_bar.top(),
            px(6.0),
            "tab bar {tab_bar:?}, tab {tab:?}",
        );

        window
            .update(cx, |app, _, cx| {
                app.scroll_reader_to_block(wide_code_index, cx);
            })
            .unwrap();
        visual.update(|window, cx| window.draw(cx).clear());

        let column = visual.debug_bounds("reader-column").expect("reader column");
        let wide_code_selector: &'static str =
            Box::leak(format!("reader-code-{wide_code_index}").into_boxed_str());
        let code = visual
            .debug_bounds(wide_code_selector)
            .expect("wide code surface");
        assert!(code.left() >= column.left());
        assert!(code.right() <= column.right());
    }

    #[gpui::test]
    fn code_copy_control_writes_exact_source_and_renders_feedback(cx: &mut TestAppContext) {
        let code = "fn main() {\n    println!(\"Hello\");\n}\n";
        let document = parse_document(
            PathBuf::from("/tmp/code-copy.md"),
            format!("```rust\n{code}```\n"),
        );
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model.tabs.open(document);
                    app.open_error = None;
                    app
                })
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);

        click_debug(&mut visual, "copy-code-0");
        visual.update(|window, cx| window.draw(cx).clear());

        assert_eq!(
            cx.read_from_clipboard().and_then(|item| item.text()),
            Some(code.to_owned()),
        );
        assert!(visual.debug_bounds("copied-code-0").is_some());
        window
            .update(cx, |app, _, _| assert_eq!(app.copied_code.unwrap().0, 0))
            .unwrap();
    }

    #[gpui::test]
    fn prepared_code_renders_and_copy_keeps_original_source(cx: &mut TestAppContext) {
        let code = "fn main() {\n    println!(\"Hello\");\n}\n";
        let document = prepare_document(parse_document(
            PathBuf::from("/tmp/highlighted.md"),
            format!("```rust\n{code}```\n"),
        ));
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model.tabs.open_prepared(document);
                    app.open_error = None;
                    app
                })
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.draw(cx).clear());

        assert!(visual.debug_bounds("reader-code-0").is_some());
        click_debug(&mut visual, "copy-code-0");
        visual.update(|window, cx| window.draw(cx).clear());
        assert_eq!(
            cx.read_from_clipboard().and_then(|item| item.text()),
            Some(code.to_owned()),
        );
    }

    #[gpui::test]
    fn single_inline_link_is_keyboard_focusable_and_routes_local_markdown(cx: &mut TestAppContext) {
        let directory = tempfile::tempdir().unwrap();
        let start = directory.path().join("start.md");
        let next = directory.path().join("next.md");
        fs::write(&start, "[Next](next.md)\n").unwrap();
        fs::write(&next, "# Next\n").unwrap();
        let mut model = AppModel::default();
        model.open_document(&start).unwrap();
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model = model;
                    app.open_error = None;
                    app
                })
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.draw(cx).clear());
        assert!(visual.debug_bounds("reader-link-focus-0-0").is_some());
        for _ in 0..16 {
            visual.simulate_event(KeyDownEvent {
                keystroke: Keystroke::parse("tab").unwrap(),
                is_held: false,
            });
            visual.update(|window, cx| window.draw(cx).clear());
            if window.update(cx, |app, _, _| app.focused_link).unwrap()
                == Some(block_link_key(0, 0))
            {
                break;
            }
        }
        window
            .update(cx, |app, _, _| {
                assert_eq!(app.focused_link, Some(block_link_key(0, 0)))
            })
            .unwrap();

        activate_focused(&mut visual, "enter");

        window
            .update(cx, |app, _, _| {
                assert_eq!(app.model.tabs.len(), 2);
                assert_eq!(
                    app.model.tabs.active().unwrap().path(),
                    next.canonicalize().unwrap()
                );
            })
            .unwrap();
    }

    #[gpui::test]
    fn zoom_shortcuts_flash_the_zoom_pill_which_then_fades(cx: &mut TestAppContext) {
        use crate::ui::zoom_hud::HudPhase;

        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| MdowApp::new(window, cx))
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.draw(cx).clear());
        assert!(visual.debug_bounds("zoom-hud").is_none());

        window
            .update(cx, |app, _, cx| {
                app.zoom_with_feedback(PrefEdit::ZoomIn, cx)
            })
            .unwrap();
        visual.update(|window, cx| window.draw(cx).clear());
        assert!(visual.debug_bounds("zoom-hud").is_some());

        click_debug(&mut visual, "zoom-hud-reset");
        window
            .update(cx, |app, _, _| {
                assert_eq!(app.prefs_snapshot().zoom.percent(), 100);
                assert_eq!(app.zoom_hud_phase(), HudPhase::Shown);
            })
            .unwrap();

        // The click left the pointer on the pill, which holds it; leaving starts the grace period.
        cx.executor().advance_clock(zoom_hud::VISIBLE_FOR);
        cx.run_until_parked();
        window
            .update(cx, |app, _, _| {
                assert_eq!(app.zoom_hud_phase(), HudPhase::Shown)
            })
            .unwrap();
        visual.simulate_mouse_move(point(px(4.0), px(4.0)), None, Modifiers::none());
        visual.update(|window, cx| window.draw(cx).clear());
        cx.executor().advance_clock(zoom_hud::HOVER_GRACE);
        cx.run_until_parked();
        let expected = if zoom_hud::prefers_reduced_motion() {
            HudPhase::Hidden
        } else {
            HudPhase::Fading
        };
        window
            .update(cx, |app, _, _| assert_eq!(app.zoom_hud_phase(), expected))
            .unwrap();
        cx.executor().advance_clock(zoom_hud::FADE_OUT);
        cx.run_until_parked();
        window
            .update(cx, |app, _, _| {
                assert_eq!(app.zoom_hud_phase(), HudPhase::Hidden)
            })
            .unwrap();

        window
            .update(cx, |app, window, cx| {
                app.click_toggle_overlay(OverlayKind::Settings, window, cx);
                app.zoom_with_feedback(PrefEdit::ZoomIn, cx);
                assert_eq!(app.prefs_snapshot().zoom.percent(), 110);
                assert_eq!(app.zoom_hud_phase(), HudPhase::Hidden);
            })
            .unwrap();
    }

    #[gpui::test]
    fn reload_error_banner_keeps_the_last_document_visible(cx: &mut TestAppContext) {
        let path = PathBuf::from("/tmp/reload-error.md");
        let mut model = AppModel::default();
        model
            .tabs
            .open(parse_document(path.clone(), "# Last good copy\n".into()));
        assert!(model.tabs.set_reload_error(&path, "Invalid UTF-8".into()));
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model = model;
                    app.open_error = None;
                    app
                })
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.draw(cx).clear());

        assert!(visual.debug_bounds("reload-error-banner").is_some());
        assert!(visual.debug_bounds("reader-block-0").is_some());
    }

    #[gpui::test]
    fn closing_active_document_clears_reader_interaction_feedback(cx: &mut TestAppContext) {
        let mut model = AppModel::default();
        model.tabs.open(parse_document(
            PathBuf::from("/tmp/first-reader.md"),
            "# First\n".into(),
        ));
        model.tabs.open(parse_document(
            PathBuf::from("/tmp/second-reader.md"),
            "# Second\n".into(),
        ));
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model = model;
                    app.open_error = None;
                    app.copied_code = Some((4, Instant::now()));
                    app.hovered_link = Some(block_link_key(1, 0));
                    app.focused_link = Some(block_link_key(1, 0));
                    app
                })
            })
            .unwrap()
        });

        window
            .update(cx, |app, window, cx| {
                app.close_active_tab(&CloseTab, window, cx)
            })
            .unwrap();

        window
            .update(cx, |app, _, _| {
                assert!(app.copied_code.is_none());
                assert!(app.hovered_link.is_none());
                assert!(app.focused_link.is_none());
            })
            .unwrap();
    }

    #[gpui::test]
    fn batch_document_open_clears_reader_transient_state(cx: &mut TestAppContext) {
        let directory = tempfile::tempdir().unwrap();
        let first = directory.path().join("first.md");
        let second = directory.path().join("second.md");
        fs::write(&first, "# First\n").unwrap();
        fs::write(&second, "# Second\n").unwrap();
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model.open_document(&first).unwrap();
                    app.copied_code = Some((2, Instant::now()));
                    app.hovered_link = Some(block_link_key(3, 0));
                    app.focused_link = Some(block_link_key(3, 0));
                    app
                })
            })
            .unwrap()
        });

        window
            .update(cx, |app, _, cx| app.open_paths(vec![second.clone()], cx))
            .unwrap();

        window
            .update(cx, |app, _, _| {
                assert_eq!(
                    app.model.tabs.active().unwrap().path(),
                    second.canonicalize().unwrap()
                );
                assert!(app.copied_code.is_none());
                assert!(app.hovered_link.is_none());
                assert!(app.focused_link.is_none());
            })
            .unwrap();
    }

    #[test]
    fn dismissing_active_reload_error_preserves_the_last_good_document() {
        let path = PathBuf::from("/tmp/reload-dismiss.md");
        let mut model = AppModel::default();
        model
            .tabs
            .open(parse_document(path.clone(), "# Last good\n".into()));
        assert!(model.tabs.set_reload_error(&path, "Broken update".into()));

        assert!(model.dismiss_active_reload_error());

        let tab = model.tabs.active().unwrap();
        assert_eq!(tab.document.title, "Last good");
        assert_eq!(tab.document.source, "# Last good\n");
        assert!(tab.reload_error.is_none());
    }

    #[gpui::test]
    fn reload_error_close_is_focusable_and_preserves_the_reader(cx: &mut TestAppContext) {
        let path = PathBuf::from("/tmp/reload-dismiss-ui.md");
        let mut model = AppModel::default();
        model
            .tabs
            .open(parse_document(path.clone(), "# Last good\n".into()));
        assert!(model.tabs.set_reload_error(&path, "Broken update".into()));
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model = model;
                    app.open_error = None;
                    app
                })
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);

        click_debug(&mut visual, "dismiss-reload-error");
        visual.update(|window, cx| window.draw(cx).clear());

        window
            .update(cx, |app, _, _| {
                assert!(app.model.tabs.active().unwrap().reload_error.is_none());
            })
            .unwrap();
        assert!(visual.debug_bounds("reader-block-0").is_some());
        window
            .update(cx, |app, _, _| {
                let tab = app.model.tabs.active().unwrap();
                assert_eq!(tab.document.title, "Last good");
                assert!(tab.reload_error.is_none());
            })
            .unwrap();
    }

    #[gpui::test]
    fn two_inline_links_are_sequential_focus_targets_and_blank_space_is_inert(
        cx: &mut TestAppContext,
    ) {
        let directory = tempfile::tempdir().unwrap();
        let start = directory.path().join("start.md");
        let first = directory.path().join("first.md");
        let second = directory.path().join("second.md");
        fs::write(&start, "[First](first.md) and [Second](second.md)\n").unwrap();
        fs::write(&first, "# First\n").unwrap();
        fs::write(&second, "# Second\n").unwrap();
        let start_canonical = start.canonicalize().unwrap();
        let mut model = AppModel::default();
        model.open_document(&start).unwrap();
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model = model;
                    app
                })
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.draw(cx).clear());

        let blank = visual
            .debug_bounds("reader-inline-0-0")
            .expect("inline surface should be painted");
        assert!(visual.debug_bounds("reader-link-focus-0-0").is_some());
        assert!(visual.debug_bounds("reader-link-focus-0-1").is_some());
        let blank_point = point(blank.right() - px(4.0), blank.center().y);
        visual.simulate_mouse_move(blank_point, None, Modifiers::none());
        visual.simulate_mouse_down(blank_point, MouseButton::Left, Modifiers::none());
        visual.simulate_mouse_up(blank_point, MouseButton::Left, Modifiers::none());
        window
            .update(cx, |app, _, _| {
                assert_eq!(app.model.tabs.len(), 1);
                assert_eq!(app.model.tabs.active().unwrap().path(), start_canonical);
            })
            .unwrap();

        let mut focused_links = Vec::new();
        for _ in 0..16 {
            visual.simulate_event(KeyDownEvent {
                keystroke: Keystroke::parse("tab").unwrap(),
                is_held: false,
            });
            visual.update(|window, cx| window.draw(cx).clear());
            let focused = window.update(cx, |app, _, _| app.focused_link).unwrap();
            if focused.is_some() && focused_links.last() != Some(&focused) {
                focused_links.push(focused);
            }
            if focused_links.len() == 2 {
                break;
            }
        }
        let focused_links = focused_links.into_iter().flatten().collect::<Vec<_>>();
        assert_eq!(
            focused_links,
            vec![block_link_key(0, 0), block_link_key(0, 1)]
        );
        window
            .update(cx, |app, _, _| {
                assert_eq!(app.focused_link, Some(block_link_key(0, 1)))
            })
            .unwrap();

        activate_focused(&mut visual, "enter");
        window
            .update(cx, |app, _, _| {
                assert_eq!(
                    app.model.tabs.active().unwrap().path(),
                    second.canonicalize().unwrap()
                );
            })
            .unwrap();
    }

    #[gpui::test]
    fn leaving_an_inline_surface_clears_its_hovered_link(cx: &mut TestAppContext) {
        let path = PathBuf::from("/tmp/hover-leave.md");
        let mut model = AppModel::default();
        model
            .tabs
            .open(parse_document(path, "[Link](next.md)\n".into()));
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model = model;
                    app
                })
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.draw(cx).clear());
        let bounds = visual.debug_bounds("reader-inline-0-0").unwrap();
        let link_point = point(bounds.left() + px(8.0), bounds.center().y);

        visual.simulate_mouse_move(link_point, None, Modifiers::none());
        window
            .update(cx, |app, _, _| {
                assert_eq!(app.hovered_link, Some(block_link_key(0, 0)))
            })
            .unwrap();

        visual.simulate_mouse_move(
            point(bounds.right() + px(8.0), bounds.bottom() + px(8.0)),
            None,
            Modifiers::none(),
        );
        window
            .update(cx, |app, _, _| assert!(app.hovered_link.is_none()))
            .unwrap();
    }

    #[gpui::test]
    fn closing_and_reopening_a_document_uses_a_fresh_scroll_handle(cx: &mut TestAppContext) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("scroll.md");
        fs::write(&path, "# Scroll\n\nparagraph\n".repeat(80)).unwrap();
        let canonical = path.canonicalize().unwrap();
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model.open_document(&path).unwrap();
                    app
                })
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.draw(cx).clear());
        window
            .update(cx, |app, _, cx| {
                app.reader_list_state(&canonical, cx)
                    .unwrap()
                    .set_offset_from_scrollbar(point(px(0.0), px(-120.0)));
            })
            .unwrap();

        window
            .update(cx, |app, window, cx| {
                app.close_active_tab(&CloseTab, window, cx)
            })
            .unwrap();
        window
            .update(cx, |app, _, _| {
                assert!(!app.reader_panes.contains_key(&canonical));
            })
            .unwrap();

        window
            .update(cx, |app, _, cx| app.open_path(&path, cx))
            .unwrap();
        visual.update(|window, cx| window.draw(cx).clear());

        window
            .update(cx, |app, _, cx| {
                assert_eq!(
                    app.reader_list_state(&canonical, cx)
                        .unwrap()
                        .scroll_px_offset_for_scrollbar()
                        .y,
                    px(0.0)
                );
            })
            .unwrap();
    }

    #[gpui::test]
    fn switching_tabs_retains_each_reader_scroll_offset(cx: &mut TestAppContext) {
        let first = PathBuf::from("/tmp/scroll-first.md");
        let second = PathBuf::from("/tmp/scroll-second.md");
        let mut model = AppModel::default();
        model.tabs.open(parse_document(
            first.clone(),
            "# First\n\nA paragraph for scrolling.\n\n".repeat(80),
        ));
        model.tabs.open(parse_document(
            second.clone(),
            "# Second\n\nAnother paragraph for scrolling.\n\n".repeat(80),
        ));
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model = model;
                    app
                })
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.draw(cx).clear());
        window
            .update(cx, |app, _, cx| app.activate_tab(&first, cx))
            .unwrap();
        visual.update(|window, cx| window.draw(cx).clear());

        window
            .update(cx, |app, _, cx| {
                app.activate_tab(&first, cx);
                app.reader_list_state(&first, cx)
                    .unwrap()
                    .set_offset_from_scrollbar(point(px(0.0), px(-120.0)));
                app.activate_tab(&second, cx);
                app.reader_list_state(&second, cx)
                    .unwrap()
                    .set_offset_from_scrollbar(point(px(0.0), px(-260.0)));
                app.activate_tab(&first, cx);
            })
            .unwrap();

        window
            .update(cx, |app, _, cx| {
                assert_eq!(
                    app.reader_list_state(&first, cx)
                        .unwrap()
                        .scroll_px_offset_for_scrollbar()
                        .y,
                    px(-120.0)
                );
                assert_eq!(
                    app.reader_list_state(&second, cx)
                        .unwrap()
                        .scroll_px_offset_for_scrollbar()
                        .y,
                    px(-260.0)
                );
                assert_eq!(app.model.tabs.active().unwrap().path(), first);
            })
            .unwrap();
    }

    #[gpui::test]
    fn same_path_document_rebuild_reconciles_link_focus_handles(cx: &mut TestAppContext) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("reader.md");
        let destination = directory.path().join("destination.md");
        fs::write(&path, "[Old self link](reader.md)\n").unwrap();
        fs::write(&destination, "# Destination\n").unwrap();
        let canonical = path.canonicalize().unwrap();
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model.open_document(&path).unwrap();
                    app
                })
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.draw(cx).clear());

        for _ in 0..16 {
            visual.simulate_event(KeyDownEvent {
                keystroke: Keystroke::parse("tab").unwrap(),
                is_held: false,
            });
            visual.update(|window, cx| window.draw(cx).clear());
            if window.update(cx, |app, _, _| app.focused_link).unwrap()
                == Some(block_link_key(0, 0))
            {
                break;
            }
        }
        let old_handle = window
            .update(cx, |app, _, _| {
                assert_eq!(app.reader_link_focus_handles.len(), 1);
                app.reader_link_focus_handles
                    .values()
                    .next()
                    .unwrap()
                    .clone()
            })
            .unwrap();
        window
            .update(cx, |_, window, _| assert!(old_handle.is_focused(window)))
            .unwrap();

        activate_focused(&mut visual, "enter");
        visual.update(|window, cx| window.draw(cx).clear());
        window
            .update(cx, |app, _, _| {
                assert_eq!(app.model.tabs.len(), 1);
                assert_eq!(app.model.tabs.active().unwrap().path(), canonical);
            })
            .unwrap();

        fs::write(&path, "No links remain.\n").unwrap();
        window
            .update(cx, |app, _, cx| {
                app.model.open_document(&path).unwrap();
                app.active_document_changed(cx);
                cx.notify();
            })
            .unwrap();
        visual.update(|window, cx| window.draw(cx).clear());
        window
            .update(cx, |app, window, _| {
                assert!(app.focused_link.is_none());
                assert!(app.reader_link_focus_handles.is_empty());
                assert!(!old_handle.is_focused(window));
                assert_eq!(app.model.tabs.len(), 1);
                assert_eq!(app.model.tabs.active().unwrap().path(), canonical);
            })
            .unwrap();
        activate_focused(&mut visual, "enter");
        window
            .update(cx, |app, _, _| assert_eq!(app.model.tabs.len(), 1))
            .unwrap();

        fs::write(
            &path,
            "The link moved to a later block.\n\n[New link](destination.md)\n",
        )
        .unwrap();
        window
            .update(cx, |app, _, cx| {
                app.model.open_document(&path).unwrap();
                app.active_document_changed(cx);
                cx.notify();
            })
            .unwrap();
        visual.update(|window, cx| window.draw(cx).clear());
        window
            .update(cx, |app, _, _| {
                assert_eq!(app.reader_link_focus_handles.len(), 1)
            })
            .unwrap();

        for _ in 0..16 {
            visual.simulate_event(KeyDownEvent {
                keystroke: Keystroke::parse("tab").unwrap(),
                is_held: false,
            });
            visual.update(|window, cx| window.draw(cx).clear());
            if window.update(cx, |app, _, _| app.focused_link).unwrap()
                == Some(block_link_key(1, 0))
            {
                break;
            }
        }
        window
            .update(cx, |app, _, _| {
                assert_eq!(app.focused_link, Some(block_link_key(1, 0)))
            })
            .unwrap();
        activate_focused(&mut visual, "enter");
        window
            .update(cx, |app, _, _| {
                assert_eq!(
                    app.model.tabs.active().unwrap().path(),
                    destination.canonicalize().unwrap(),
                );
            })
            .unwrap();
    }

    fn document_window(cx: &mut TestAppContext, source: &str) -> gpui::WindowHandle<MdowApp> {
        let document = parse_document(PathBuf::from("/tmp/click.md"), source.into());
        cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model.tabs.open(document);
                    app.open_error = None;
                    app
                })
            })
            .unwrap()
        })
    }

    #[gpui::test]
    fn outline_and_contents_links_reach_repeated_headings(cx: &mut TestAppContext) {
        let filler = "A paragraph with enough content to scroll.\n\n".repeat(60);
        let source = format!(
            "# Intro\n\n[Contents](#details-1)\n\n## Details\n\n{filler}## Details\n\n{filler}"
        );
        let window = document_window(cx, &source);
        let mut visual = VisualTestContext::from_window(*window, cx);
        click_debug(&mut visual, "Outline");
        click_debug(&mut visual, "outline-row-2");
        window
            .update(cx, |app, _, cx| {
                let tab = app.model.tabs.active().unwrap();
                let expected = tab.document.heading_block(2).unwrap();
                let pane = app.reader_panes.get(tab.path()).unwrap().read(cx);
                assert_eq!(pane.list_state().logical_scroll_top().item_ix, expected);
                app.jump_to_heading(0, cx);
            })
            .unwrap();
        visual.update(|window, cx| window.draw(cx).clear());
        window
            .update(cx, |app, _, cx| {
                app.activate_link(Path::new("/tmp/click.md"), "#details-1", cx);
            })
            .unwrap();
        visual.update(|window, cx| window.draw(cx).clear());
        window
            .update(cx, |app, _, cx| {
                let tab = app.model.tabs.active().unwrap();
                let expected = tab.document.heading_block(2).unwrap();
                let pane = app.reader_panes.get(tab.path()).unwrap().read(cx);
                assert_eq!(pane.list_state().logical_scroll_top().item_ix, expected);
            })
            .unwrap();
    }

    #[gpui::test]
    fn outline_highlights_the_heading_at_the_reader_scroll_position(cx: &mut TestAppContext) {
        let filler = "A paragraph with enough content to scroll.\n\n".repeat(40);
        let source = format!("# Intro\n\n{filler}## Middle\n\n{filler}## End\n\n{filler}");
        let window = document_window(cx, &source);
        let mut visual = VisualTestContext::from_window(*window, cx);
        click_debug(&mut visual, "Outline");
        visual.update(|window, cx| window.draw(cx).clear());
        let active = |visual: &mut VisualTestContext| {
            window
                .update(visual, |app, _, cx| app.active_outline_heading(cx))
                .unwrap()
        };
        assert_eq!(active(&mut visual), Some(0));

        let bounds = visual
            .debug_bounds("reader-scroll")
            .expect("reader viewport");
        let middle_block = window
            .update(&mut visual, |app, _, _| {
                app.model.tabs.active().unwrap().document.heading_block(1)
            })
            .unwrap()
            .unwrap();
        for _ in 0..200 {
            visual.simulate_event(ScrollWheelEvent {
                position: bounds.center(),
                delta: ScrollDelta::Pixels(point(px(0.0), px(-120.0))),
                ..Default::default()
            });
            visual.update(|window, cx| window.draw(cx).clear());
            let top = window
                .update(&mut visual, |app, _, cx| {
                    app.reader_list_state(app.model.tabs.active().unwrap().path(), cx)
                        .unwrap()
                        .logical_scroll_top()
                        .item_ix
                })
                .unwrap();
            if top >= middle_block {
                break;
            }
        }
        assert_eq!(active(&mut visual), Some(1));

        click_debug(&mut visual, "outline-row-2");
        visual.update(|window, cx| window.draw(cx).clear());
        assert_eq!(active(&mut visual), Some(2));
    }

    #[gpui::test]
    fn chrome_buttons_run_their_handlers_instead_of_dispatching(cx: &mut TestAppContext) {
        let window = document_window(cx, "# Click\n\n## Nested\n");
        let mut visual = VisualTestContext::from_window(*window, cx);

        click_debug(&mut visual, "toggle-sidebar");
        window
            .update(cx, |app, _, _| assert!(!app.sidebar_open))
            .unwrap();
        click_debug(&mut visual, "toggle-sidebar");
        window
            .update(cx, |app, _, _| assert!(app.sidebar_open))
            .unwrap();

        click_debug(&mut visual, "toggle-wide-mode");
        window
            .update(cx, |app, _, _| assert!(app.wide_mode))
            .unwrap();

        click_debug(&mut visual, "Outline");
        window
            .update(cx, |app, _, _| {
                assert_eq!(app.prefs_snapshot().sidebar_mode, SidebarMode::Outline)
            })
            .unwrap();
        visual.update(|window, cx| window.draw(cx).clear());
        assert!(visual.debug_bounds("outline-row-0").is_some());

        click_debug(&mut visual, "sidebar-settings");
        window
            .update(cx, |app, _, _| {
                assert_eq!(app.overlay_kind(), Some(OverlayKind::Settings))
            })
            .unwrap();
        visual.update(|window, cx| window.draw(cx).clear());
        click_debug(&mut visual, "Dark-Theme(Dark)");
        window
            .update(cx, |app, _, _| {
                assert_eq!(app.prefs_snapshot().theme_mode, ThemeMode::Dark)
            })
            .unwrap();
        visual.update(|window, cx| window.draw(cx).clear());
        click_debug(&mut visual, "Full-LineWidth(Full)");
        visual.update(|window, cx| window.draw(cx).clear());
        click_debug(&mut visual, "settings-zoom-in");
        visual.update(|window, cx| window.draw(cx).clear());
        click_debug(&mut visual, "settings-auto-update");
        window
            .update(cx, |app, _, _| {
                let prefs = app.prefs_snapshot();
                assert!(prefs.reader_width.is_full());
                assert_eq!(prefs.zoom.percent(), 110);
                assert!(!prefs.auto_update);
            })
            .unwrap();
        visual.update(|window, cx| window.draw(cx).clear());
        click_debug(&mut visual, "settings-restore-defaults");
        window
            .update(cx, |app, _, _| {
                assert_eq!(app.prefs_snapshot(), Prefs::default())
            })
            .unwrap();
        visual.update(|window, cx| window.draw(cx).clear());
        click_debug(&mut visual, "settings-close");
        window
            .update(cx, |app, _, _| assert_eq!(app.overlay_kind(), None))
            .unwrap();

        click_debug(&mut visual, "sidebar-settings");
        window
            .update(cx, |app, _, _| {
                assert_eq!(app.overlay_kind(), Some(OverlayKind::Settings))
            })
            .unwrap();
        window
            .update(cx, |app, window, cx| {
                app.click_toggle_overlay(OverlayKind::Settings, window, cx);
            })
            .unwrap();

        click_debug(&mut visual, "toggle-find");
        window
            .update(cx, |app, _, _| {
                assert_eq!(app.overlay_kind(), Some(OverlayKind::Find))
            })
            .unwrap();
        visual.update(|window, cx| window.draw(cx).clear());
        click_debug(&mut visual, "find-close");
        window
            .update(cx, |app, _, _| assert_eq!(app.overlay_kind(), None))
            .unwrap();

        click_debug(&mut visual, "toggle-palette");
        window
            .update(cx, |app, _, _| {
                assert_eq!(app.overlay_kind(), Some(OverlayKind::Palette))
            })
            .unwrap();
        visual.update(|window, cx| window.draw(cx).clear());
        click_debug(&mut visual, "palette-item-5");
        window
            .update(cx, |app, _, _| {
                assert_eq!(app.overlay_kind(), None);
                assert!(!app.sidebar_open);
            })
            .unwrap();
    }

    #[gpui::test]
    fn reader_scroll_cost_on_a_large_document(cx: &mut TestAppContext) {
        const BLOCKS: usize = 1_200;
        const SCROLL_FRAMES: usize = 20;
        let document = parse_document(
            PathBuf::from("/tmp/reader-scroll-bench.md"),
            (0..BLOCKS)
                .map(|index| {
                    format!(
                        "Paragraph {index} is long enough to wrap inside the reader column and keep layout busy."
                    )
                })
                .collect::<Vec<_>>()
                .join("\n\n"),
        );
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model.tabs.open(document);
                    app.open_error = None;
                    app
                })
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);

        let first_paint = Instant::now();
        visual.update(|window, cx| window.draw(cx).clear());
        let first_paint_ms = first_paint.elapsed().as_secs_f64() * 1000.0;

        let bounds = visual
            .debug_bounds("reader-scroll")
            .expect("reader viewport");
        let painted_blocks = (0..BLOCKS)
            .filter(|index| {
                let selector = format!("reader-block-{index}");
                visual
                    .debug_bounds(Box::leak(selector.into_boxed_str()))
                    .is_some()
            })
            .count();

        let scroll = Instant::now();
        for _ in 0..SCROLL_FRAMES {
            visual.simulate_event(ScrollWheelEvent {
                position: bounds.center(),
                delta: ScrollDelta::Pixels(point(px(0.0), px(-80.0))),
                ..Default::default()
            });
            visual.update(|window, cx| window.draw(cx).clear());
        }
        let scroll_ms = scroll.elapsed().as_secs_f64() * 1000.0;
        let scroll_frame_ms = scroll_ms / SCROLL_FRAMES as f64;

        let report = serde_json::json!({
            "blocks": BLOCKS,
            "painted_blocks_after_first_paint": painted_blocks,
            "first_paint_ms": first_paint_ms,
            "scroll_frames": SCROLL_FRAMES,
            "scroll_total_ms": scroll_ms,
            "scroll_frame_ms": scroll_frame_ms,
        });
        let report_text = serde_json::to_string_pretty(&report).unwrap();
        eprintln!("MDOW_READER_SCROLL_BENCH {report_text}");
        if let Ok(path) = std::env::var("MDOW_READER_BENCH_OUT") {
            if let Some(parent) = Path::new(&path).parent() {
                let _ = fs::create_dir_all(parent);
            }
            fs::write(path, report_text).unwrap();
        }

        assert!(
            painted_blocks < 40,
            "virtualized reader painted {painted_blocks} of {BLOCKS} blocks"
        );
        assert!(
            first_paint_ms < 80.0,
            "first paint of {BLOCKS} blocks took {first_paint_ms:.1}ms"
        );
        assert!(
            scroll_frame_ms < 50.0,
            "scroll frame of {BLOCKS} blocks took {scroll_frame_ms:.1}ms"
        );
    }

    /// Times the large-document pipeline on the UI thread. Run in release with a generated file:
    /// `MDOW_PIPELINE_BENCH_DOC=/tmp/big.md cargo test --release --lib
    /// large_document_pipeline_bench -- --ignored --nocapture` (see script/bench_gpui_reader.sh).
    /// The test executor runs background work on this thread while parked, so `open_call_ms` and
    /// `reload_call_ms` are the synchronous UI-thread cost and `*_ready_ms` include the parse.
    #[gpui::test]
    #[ignore]
    fn large_document_pipeline_bench(cx: &mut TestAppContext) {
        const SCROLL_FRAMES: usize = 30;
        let Some(source_path) = std::env::var_os("MDOW_PIPELINE_BENCH_DOC").map(PathBuf::from)
        else {
            eprintln!("set MDOW_PIPELINE_BENCH_DOC to a markdown file");
            return;
        };
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("pipeline-bench.md");
        fs::copy(&source_path, &path).unwrap();
        let path = path.canonicalize().unwrap();
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    // No live watcher: the copied file would otherwise trigger a reload that the
                    // test executor runs inside a measured frame.
                    let mut app = MdowApp::boot_with_watcher(
                        Prefs::default(),
                        StateStore::in_memory(),
                        SessionRole::Owner,
                        Err(anyhow::anyhow!("watcher disabled for the benchmark")),
                        window,
                        cx,
                    );
                    app.set_sidebar_mode(SidebarMode::Outline, cx);
                    app
                })
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);
        redraw(&mut visual);

        let open = Instant::now();
        window
            .update(&mut visual, |app, _, cx| app.open_path(&path, cx))
            .unwrap();
        let open_call_ms = open.elapsed().as_secs_f64() * 1000.0;
        let first_paint = Instant::now();
        redraw(&mut visual);
        let first_paint_ms = first_paint.elapsed().as_secs_f64() * 1000.0;
        visual.run_until_parked();
        let open_ready_ms = open.elapsed().as_secs_f64() * 1000.0;
        let full_paint = Instant::now();
        redraw(&mut visual);
        let full_paint_ms = full_paint.elapsed().as_secs_f64() * 1000.0;
        let blocks = window
            .read_with(&visual, |app, _| {
                app.model
                    .tabs
                    .active()
                    .map_or(0, |tab| tab.document.blocks.len())
            })
            .unwrap();

        let bounds = visual
            .debug_bounds("reader-scroll")
            .expect("reader viewport");
        // The real app loads syntax definitions on a background thread; the test executor
        // would run that one-time load inside a measured frame, so load it up front.
        if let crate::syntax::HighlightLookup::Claimed(key) =
            crate::syntax::HighlightCache::global().lookup(
                Some("rust"),
                "fn warm() {}\n",
                crate::theme::ColorScheme::Light,
            )
        {
            crate::syntax::HighlightCache::global().highlight_claimed(
                key,
                "rust",
                "fn warm() {}\n",
            );
        }
        let mut worst_frame_ms = 0.0_f64;
        let scroll = Instant::now();
        for _ in 0..SCROLL_FRAMES {
            let frame = Instant::now();
            visual.simulate_event(ScrollWheelEvent {
                position: bounds.center(),
                delta: ScrollDelta::Pixels(point(px(0.0), px(-240.0))),
                ..Default::default()
            });
            redraw(&mut visual);
            worst_frame_ms = worst_frame_ms.max(frame.elapsed().as_secs_f64() * 1000.0);
        }
        let scroll_frame_ms = scroll.elapsed().as_secs_f64() * 1000.0 / SCROLL_FRAMES as f64;

        let source = fs::read_to_string(&path).unwrap();
        let cut = source[..4096.min(source.len())]
            .rfind("\n\n")
            .map_or(0, |index| index + 2);
        fs::write(
            &path,
            format!(
                "{}Inserted while reading.\n\n{}",
                &source[..cut],
                &source[cut..]
            ),
        )
        .unwrap();
        let reload = Instant::now();
        window
            .update(&mut visual, |app, _, cx| {
                app.handle_watch_messages(vec![WatchMessage::Reload(path.clone())], cx)
            })
            .unwrap();
        let reload_call_ms = reload.elapsed().as_secs_f64() * 1000.0;
        visual.run_until_parked();
        let reload_ready_ms = reload.elapsed().as_secs_f64() * 1000.0;
        let reload_paint = Instant::now();
        redraw(&mut visual);
        let reload_paint_ms = reload_paint.elapsed().as_secs_f64() * 1000.0;

        let report = serde_json::json!({
            "document_bytes": source.len(),
            "blocks": blocks,
            "open_call_ms": open_call_ms,
            "first_paint_ms": first_paint_ms,
            "open_ready_ms": open_ready_ms,
            "full_paint_ms": full_paint_ms,
            "scroll_frame_ms": scroll_frame_ms,
            "scroll_worst_frame_ms": worst_frame_ms,
            "reload_call_ms": reload_call_ms,
            "reload_ready_ms": reload_ready_ms,
            "reload_paint_ms": reload_paint_ms,
        });
        let report_text = serde_json::to_string_pretty(&report).unwrap();
        eprintln!("MDOW_PIPELINE_BENCH {report_text}");
        if let Ok(out) = std::env::var("MDOW_PIPELINE_BENCH_OUT") {
            fs::write(out, report_text).unwrap();
        }
    }

    fn folder_window(cx: &mut TestAppContext) -> (gpui::WindowHandle<MdowApp>, tempfile::TempDir) {
        let root = markdown_workspace();
        fs::write(root.path().join("guides/reading.md"), "# Reading").unwrap();
        let mut model = AppModel::default();
        model.open_workspace(root.path()).unwrap();
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model = model;
                    app.open_error = None;
                    app.set_sidebar_mode(SidebarMode::Folder, cx);
                    app
                })
            })
            .unwrap()
        });
        (window, root)
    }

    fn redraw(visual: &mut VisualTestContext) {
        visual.update(|window, cx| window.draw(cx).clear());
    }

    #[gpui::test]
    fn folder_filter_narrows_expands_counts_and_clears(cx: &mut TestAppContext) {
        let (window, _root) = folder_window(cx);
        let mut visual = VisualTestContext::from_window(*window, cx);
        redraw(&mut visual);
        assert!(visual.debug_bounds("folder-filter").is_some());
        assert!(visual.debug_bounds("folder-filter-count").is_none());
        // Collapsed tree: guides, README.md.
        assert!(visual.debug_bounds("workspace-row-1").is_some());
        assert!(visual.debug_bounds("workspace-row-2").is_none());

        window
            .update(cx, |app, _, cx| app.set_folder_filter("READ", cx))
            .unwrap();
        redraw(&mut visual);
        let rows = window
            .update(cx, |app, _, cx| {
                let query = app.folder_filter_query(cx);
                app.model
                    .workspace
                    .as_ref()
                    .unwrap()
                    .filtered_rows(&query, &app.filter_collapsed)
            })
            .unwrap();
        assert_eq!(rows.match_count, 2);
        assert_eq!(
            rows.rows
                .iter()
                .map(|row| row.row.name.as_str())
                .collect::<Vec<_>>(),
            vec!["guides", "reading.md", "README.md"]
        );
        assert!(visual.debug_bounds("folder-filter-count").is_some());
        assert!(visual.debug_bounds("workspace-row-2").is_some());

        // Collapsing a folder while filtering hides its matches without touching the tree.
        click_debug(&mut visual, "workspace-disclosure-0");
        window
            .update(cx, |app, _, cx| {
                let query = app.folder_filter_query(cx);
                let filtered = app
                    .model
                    .workspace
                    .as_ref()
                    .unwrap()
                    .filtered_rows(&query, &app.filter_collapsed);
                assert_eq!(filtered.rows.len(), 2);
                assert!(!app.model.workspace.as_ref().unwrap().visible_rows()[0].expanded);
            })
            .unwrap();

        window
            .update(cx, |app, _, cx| app.set_folder_filter("zzz", cx))
            .unwrap();
        redraw(&mut visual);
        assert!(visual.debug_bounds("folder-filter-empty").is_some());

        // Escape (the field's Cancel) clears the query.
        window
            .update(cx, |app, window, cx| {
                app.folder_filter.read(cx).focus(window)
            })
            .unwrap();
        redraw(&mut visual);
        visual.dispatch_action(crate::ui::field::Cancel);
        window
            .update(cx, |app, _, cx| assert_eq!(app.folder_filter_query(cx), ""))
            .unwrap();
    }

    #[gpui::test]
    fn tab_context_menu_is_keyboard_navigable_and_runs_actions(cx: &mut TestAppContext) {
        let (window, first, second, _root) = two_tab_window(cx);
        let mut visual = VisualTestContext::from_window(*window, cx);
        redraw(&mut visual);
        let tab = visual.debug_bounds("document-tab-1").unwrap().center();
        visual.simulate_mouse_down(tab, MouseButton::Right, Modifiers::none());
        visual.simulate_mouse_up(tab, MouseButton::Right, Modifiers::none());
        redraw(&mut visual);
        assert!(visual.debug_bounds("context-menu").is_some());
        let labels = window
            .update(cx, |app, _, cx| {
                app.context_menu_view()
                    .unwrap()
                    .read(cx)
                    .entries()
                    .iter()
                    .map(|entry| match entry {
                        ContextMenuEntry::Item { label, enabled } => {
                            format!("{label}{}", if *enabled { "" } else { " (off)" })
                        }
                        ContextMenuEntry::Separator => "-".into(),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap();
        assert_eq!(
            labels,
            vec![
                "Close",
                "Close Others",
                "Close to the Right (off)",
                "Close All",
                "-",
                "Open in Left Pane",
                "Open in Right Pane",
                "-",
                "Copy Path",
                "Reveal in Finder"
            ]
        );

        visual.simulate_keystrokes("down down");
        window
            .update(cx, |app, _, cx| {
                assert_eq!(
                    app.context_menu_view().unwrap().read(cx).highlighted(),
                    Some(1)
                );
            })
            .unwrap();
        visual.simulate_keystrokes("enter");
        window
            .update(cx, |app, _, _| {
                assert!(app.context_menu_view().is_none());
                assert_eq!(app.model.tabs.len(), 1);
                assert_eq!(app.model.tabs.active().unwrap().path(), second);
                assert!(app.model.tabs.get(&first).is_none());
            })
            .unwrap();

        // Escape dismisses without acting.
        redraw(&mut visual);
        let tab = visual.debug_bounds("document-tab-0").unwrap().center();
        visual.simulate_mouse_down(tab, MouseButton::Right, Modifiers::none());
        redraw(&mut visual);
        visual.simulate_keystrokes("escape");
        window
            .update(cx, |app, _, _| {
                assert!(app.context_menu_view().is_none());
                assert_eq!(app.model.tabs.len(), 1);
            })
            .unwrap();

        // Clicking an item runs it; an outside click dismisses.
        visual.simulate_mouse_down(tab, MouseButton::Right, Modifiers::none());
        redraw(&mut visual);
        click_debug(&mut visual, "context-menu-item-8");
        assert_eq!(
            visual.read_from_clipboard().and_then(|item| item.text()),
            Some(second.to_string_lossy().into_owned())
        );
        visual.simulate_mouse_down(tab, MouseButton::Right, Modifiers::none());
        redraw(&mut visual);
        let outside = visual.debug_bounds("reader-scroll").unwrap().center();
        visual.simulate_mouse_down(outside, MouseButton::Left, Modifiers::none());
        visual.simulate_mouse_up(outside, MouseButton::Left, Modifiers::none());
        window
            .update(cx, |app, _, _| assert!(app.context_menu_view().is_none()))
            .unwrap();
    }

    #[gpui::test]
    fn close_to_the_right_and_close_all_follow_strip_order(cx: &mut TestAppContext) {
        let (window, paths) = three_tab_window(cx);
        let menu = tab_context_menu(&paths[0], &paths);
        assert!(matches!(
            &menu[2],
            (ContextMenuEntry::Item { enabled: true, .. }, Some(ContextAction::CloseTabsToRight(path)))
                if *path == paths[0]
        ));

        window
            .update(cx, |app, _, cx| {
                app.run_context_action(ContextAction::CloseTabsToRight(paths[0].clone()), cx);
                assert_eq!(app.tab_paths(), vec![paths[0].clone()]);
                app.run_context_action(ContextAction::CloseAllTabs, cx);
                assert!(app.model.tabs.is_empty());
            })
            .unwrap();
    }

    #[gpui::test]
    fn recents_clear_and_remove_persist(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let state_path = state.path().join("state.json");
        let root = markdown_workspace();
        let first = root.path().join("README.md").canonicalize().unwrap();
        let second = root.path().join("guides/start.md").canonicalize().unwrap();
        let store_path = state_path.clone();
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::boot(
                        Prefs::default(),
                        StateStore::open_at(store_path),
                        SessionRole::Owner,
                        window,
                        cx,
                    );
                    app.model.recents = Recents::from_paths(vec![first.clone(), second.clone()]);
                    app
                })
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);
        redraw(&mut visual);

        let menu = recent_context_menu(&first);
        assert!(menu.iter().any(|(entry, action)| {
            *entry == ContextMenuEntry::item("Remove from Recents")
                && *action == Some(ContextAction::RemoveRecent(first.clone()))
        }));
        window
            .update(cx, |app, _, cx| {
                app.run_context_action(ContextAction::RemoveRecent(first.clone()), cx)
            })
            .unwrap();
        let saved = StateStore::open_at(state_path.clone())
            .load()
            .session
            .recents;
        assert_eq!(saved.iter().collect::<Vec<_>>(), vec![second.as_path()]);

        click_debug(&mut visual, "recents-clear");
        window
            .update(cx, |app, _, _| assert!(app.model.recents.is_empty()))
            .unwrap();
        assert!(
            StateStore::open_at(state_path)
                .load()
                .session
                .recents
                .is_empty()
        );
    }

    #[gpui::test]
    fn welcome_hides_the_tab_row_and_breadcrumb(cx: &mut TestAppContext) {
        let (window, _recent, _root) = recents_only_window(cx);
        let mut visual = VisualTestContext::from_window(*window, cx);
        redraw(&mut visual);

        assert!(visual.debug_bounds("tab-bar").is_none());
        assert!(visual.debug_bounds("breadcrumb").is_none());
        assert!(visual.debug_bounds("toggle-find").is_none());
        let toolbar = visual.debug_bounds("empty-toolbar").expect("empty toolbar");
        assert_eq!(toolbar.top(), px(0.0));
        assert_eq!(toolbar.size.height, px(40.0));
        assert!(visual.debug_bounds("toggle-palette").is_some());
        assert!(visual.debug_bounds("welcome").is_some());
        assert!(visual.debug_bounds("welcome-recent-0").is_some());
    }

    #[gpui::test]
    fn a_deleted_open_file_keeps_its_content_behind_a_deleted_banner(cx: &mut TestAppContext) {
        let root = markdown_workspace();
        let path = root.path().join("README.md").canonicalize().unwrap();
        let mut model = AppModel::default();
        model.open_document(&path).unwrap();
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model = model;
                    app
                })
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);
        redraw(&mut visual);
        assert!(visual.debug_bounds("deleted-banner").is_none());

        fs::remove_file(&path).unwrap();
        window
            .update(cx, |app, _, cx| {
                app.handle_watch_messages(vec![WatchMessage::Reload(path.clone())], cx)
            })
            .unwrap();
        redraw(&mut visual);
        window
            .update(cx, |app, _, _| assert!(app.model.is_deleted(&path)))
            .unwrap();
        assert!(visual.debug_bounds("deleted-banner").is_some());
        assert!(visual.debug_bounds("reload-error-banner").is_none());
        assert!(visual.debug_bounds("reader-block-0").is_some());

        // The file coming back clears the state.
        fs::write(&path, "# Back").unwrap();
        window
            .update(cx, |app, _, cx| {
                app.handle_watch_messages(vec![WatchMessage::Reload(path.clone())], cx);
                assert!(!app.model.is_deleted(&path));
            })
            .unwrap();

        fs::remove_file(&path).unwrap();
        window
            .update(cx, |app, _, cx| {
                app.handle_watch_messages(vec![WatchMessage::Reload(path.clone())], cx)
            })
            .unwrap();
        redraw(&mut visual);
        click_debug(&mut visual, "deleted-close-tab");
        window
            .update(cx, |app, _, _| {
                assert!(app.model.tabs.is_empty());
                assert!(app.model.deleted.is_empty());
            })
            .unwrap();
    }

    #[gpui::test]
    fn the_open_folder_follows_files_created_on_disk(cx: &mut TestAppContext) {
        let dir = watcher_workspace();
        fs::create_dir(dir.path().join("guides")).unwrap();
        fs::write(dir.path().join("guides/start.md"), "# Start").unwrap();
        let root = dir.path().canonicalize().unwrap();
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| MdowApp::new(window, cx))
            })
            .unwrap()
        });
        window
            .update(cx, |app, _, cx| app.open_path(&root, cx))
            .unwrap();
        // The folder scans off the UI thread.
        cx.run_until_parked();
        window
            .update(cx, |app, _, cx| {
                app.toggle_directory(&root.join("guides"), cx);
                app.set_folder_filter("new", cx);
            })
            .unwrap();
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(300));
        cx.executor().advance_clock(Duration::from_millis(100));
        cx.run_until_parked();

        fs::write(root.join("guides/new.md"), "# New").unwrap();
        let mut found = false;
        for _ in 0..30 {
            std::thread::sleep(Duration::from_millis(100));
            cx.executor().advance_clock(Duration::from_millis(100));
            cx.run_until_parked();
            found = window
                .update(cx, |app, _, _| {
                    app.model
                        .workspace
                        .as_ref()
                        .unwrap()
                        .files()
                        .iter()
                        .any(|path| path.ends_with("guides/new.md"))
                })
                .unwrap();
            if found {
                break;
            }
        }
        assert!(found, "new file should appear in the folder tree");
        window
            .update(cx, |app, _, cx| {
                let tree = app.model.workspace.as_ref().unwrap();
                assert!(
                    tree.visible_rows()[0].expanded,
                    "expansion survives the rescan"
                );
                assert_eq!(
                    app.folder_filter_query(cx),
                    "new",
                    "filter survives the rescan"
                );
            })
            .unwrap();
    }

    #[gpui::test]
    fn dragging_external_paths_shows_the_drop_target(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| MdowApp::new(window, cx))
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window(*window, cx);
        redraw(&mut visual);
        assert!(visual.debug_bounds("drop-overlay").is_none());

        window
            .update(cx, |app, window, cx| {
                app.drag_moved(
                    DropSummary {
                        markdown: 3,
                        html: 0,
                        folders: 1,
                    },
                    window,
                    cx,
                );
                assert_eq!(
                    app.drop_state.summary().label(),
                    "3 Markdown files · 1 folder"
                );
            })
            .unwrap();
        redraw(&mut visual);
        assert!(visual.debug_bounds("drop-overlay").is_some());
        assert!(visual.debug_bounds("drop-overlay-summary").is_some());
    }

    #[gpui::test]
    fn outline_uses_indent_guides_and_follows_the_active_heading(cx: &mut TestAppContext) {
        let filler = "A paragraph with enough content to scroll.\n\n".repeat(4);
        let mut source = String::from("# Top\n\n");
        for index in 0..80 {
            source.push_str(&format!(
                "## Section {index}\n\n### Detail {index}\n\n{filler}"
            ));
        }
        let window = document_window(cx, &source);
        let mut visual = VisualTestContext::from_window(*window, cx);
        click_debug(&mut visual, "Outline");
        redraw(&mut visual);
        assert!(visual.debug_bounds("outline-guide-1").is_some());
        assert!(visual.debug_bounds("outline-guide-0").is_none());

        let target = 150;
        window
            .update(cx, |app, _, cx| app.jump_to_heading(target, cx))
            .unwrap();
        for _ in 0..3 {
            redraw(&mut visual);
        }
        let active = window
            .update(cx, |app, _, cx| app.active_outline_heading(cx))
            .unwrap();
        assert_eq!(active, Some(target));
        let list = visual.debug_bounds("outline-scroll").expect("outline list");
        let selector: &'static str = Box::leak(format!("outline-row-{target}").into_boxed_str());
        let row = visual.debug_bounds(selector).expect("active outline row");
        assert!(
            row.top() >= list.top() && row.bottom() <= list.bottom(),
            "row {row:?} should be scrolled into {list:?}"
        );
    }

    #[gpui::test]
    fn right_clicking_another_tab_replaces_the_open_menu(cx: &mut TestAppContext) {
        let (window, _first, _second, _root) = two_tab_window(cx);
        let mut visual = VisualTestContext::from_window(*window, cx);
        redraw(&mut visual);
        let first_tab = visual.debug_bounds("document-tab-0").unwrap().center();
        let second_tab = visual.debug_bounds("document-tab-1").unwrap().center();
        // Open on the right-hand tab first so the menu does not cover the left one.
        visual.simulate_mouse_down(second_tab, MouseButton::Right, Modifiers::none());
        redraw(&mut visual);
        let first_menu = window
            .update(cx, |app, _, _| app.context_menu_view().unwrap())
            .unwrap();
        visual.simulate_mouse_down(first_tab, MouseButton::Right, Modifiers::none());
        redraw(&mut visual);
        window
            .update(cx, |app, _, cx| {
                let menu = app.context_menu_view().expect("a menu stays open");
                assert_ne!(menu, first_menu);
                // The first tab has a neighbour to its right.
                assert_eq!(
                    menu.read(cx).entries()[2],
                    ContextMenuEntry::item("Close to the Right")
                );
            })
            .unwrap();
    }

    fn active_pane(
        window: gpui::WindowHandle<MdowApp>,
        visual: &mut VisualTestContext,
    ) -> Entity<ReaderPane> {
        window
            .update(visual, |app, _, _| app.active_reader_pane().unwrap())
            .unwrap()
    }

    /// Where the caret before byte `offset` of surface `surface` of block `block` was painted.
    fn caret(
        window: gpui::WindowHandle<MdowApp>,
        visual: &mut VisualTestContext,
        block: usize,
        surface: usize,
        offset: usize,
    ) -> Point<Pixels> {
        let pane = active_pane(window, visual);
        visual.update(|_, cx| {
            pane.read(cx)
                .painted_caret(TextPoint::new(SurfaceId::new(block, surface), offset))
                .unwrap_or_else(|| panic!("surface {block}/{surface} should be painted"))
        })
    }

    fn selected_text(
        window: gpui::WindowHandle<MdowApp>,
        visual: &mut VisualTestContext,
    ) -> Option<String> {
        let pane = active_pane(window, visual);
        visual.update(|_, cx| pane.read(cx).selected_text())
    }

    fn drag(visual: &mut VisualTestContext, from: Point<Pixels>, to: Point<Pixels>) {
        visual.simulate_mouse_move(from, None, Modifiers::none());
        visual.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
        visual.simulate_mouse_move(to, Some(MouseButton::Left), Modifiers::none());
        visual.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
        redraw(visual);
    }

    fn selection_window(
        cx: &mut TestAppContext,
        source: &str,
    ) -> (gpui::WindowHandle<MdowApp>, VisualTestContext) {
        let window = document_window(cx, source);
        let mut visual = VisualTestContext::from_window(*window, cx);
        redraw(&mut visual);
        redraw(&mut visual);
        (window, visual)
    }

    #[gpui::test]
    fn dragging_selects_text_within_a_paragraph(cx: &mut TestAppContext) {
        let (window, mut visual) =
            selection_window(cx, "First paragraph here.\n\nSecond paragraph text.");
        let from = caret(window, &mut visual, 0, 0, 6);
        let to = caret(window, &mut visual, 0, 0, 15);
        drag(&mut visual, from, to);
        assert_eq!(
            selected_text(window, &mut visual).as_deref(),
            Some("paragraph")
        );

        // Selection paints behind the glyphs of exactly the selected range.
        let pane = active_pane(window, &mut visual);
        let rects = visual.update(|_, cx| pane.read(cx).painted_rects(SurfaceId::new(0, 0), 6..15));
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].left(), from.x);
        assert_eq!(rects[0].right(), to.x);

        // A plain click elsewhere collapses (clears) the selection.
        let elsewhere = caret(window, &mut visual, 1, 0, 3);
        drag(&mut visual, elsewhere, elsewhere);
        assert_eq!(selected_text(window, &mut visual), None);
    }

    #[gpui::test]
    fn dragging_across_blocks_copies_a_blank_line_between_paragraphs(cx: &mut TestAppContext) {
        let (window, mut visual) =
            selection_window(cx, "First paragraph here.\n\nSecond paragraph text.");
        let from = caret(window, &mut visual, 0, 0, 6);
        let to = caret(window, &mut visual, 1, 0, 6);
        drag(&mut visual, from, to);
        assert_eq!(
            selected_text(window, &mut visual).as_deref(),
            Some("paragraph here.\n\nSecond")
        );

        // Dragging backwards selects the same text.
        drag(&mut visual, to, from);
        assert_eq!(
            selected_text(window, &mut visual).as_deref(),
            Some("paragraph here.\n\nSecond")
        );

        // Shift-click extends from the anchor (the backward drag anchored after "Second").
        let end = caret(window, &mut visual, 1, 0, 16);
        visual.simulate_mouse_down(end, MouseButton::Left, Modifiers::shift());
        visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::shift());
        redraw(&mut visual);
        assert_eq!(
            selected_text(window, &mut visual).as_deref(),
            Some(" paragraph")
        );
    }

    #[gpui::test]
    fn dragging_selects_inside_code_blocks(cx: &mut TestAppContext) {
        let (window, mut visual) =
            selection_window(cx, "Intro.\n\n```rust\nfn main() {\n    run();\n}\n```\n");
        let from = caret(window, &mut visual, 1, 0, 3);
        let to = caret(window, &mut visual, 1, 0, 22);
        drag(&mut visual, from, to);
        assert_eq!(
            selected_text(window, &mut visual).as_deref(),
            Some("main() {\n    run();")
        );
    }

    #[gpui::test]
    fn double_click_selects_a_word_and_triple_click_the_block(cx: &mut TestAppContext) {
        let (window, mut visual) = selection_window(cx, "Alpha bravo_two charlie.");
        let position = caret(window, &mut visual, 0, 0, 8);
        for click_count in 1..=2 {
            visual.simulate_event(MouseDownEvent {
                button: MouseButton::Left,
                position,
                modifiers: Modifiers::none(),
                click_count,
                first_mouse: false,
            });
            visual.simulate_event(MouseUpEvent {
                button: MouseButton::Left,
                position,
                modifiers: Modifiers::none(),
                click_count,
            });
        }
        redraw(&mut visual);
        assert_eq!(
            selected_text(window, &mut visual).as_deref(),
            Some("bravo_two")
        );

        visual.simulate_event(MouseDownEvent {
            button: MouseButton::Left,
            position,
            modifiers: Modifiers::none(),
            click_count: 3,
            first_mouse: false,
        });
        visual.simulate_event(MouseUpEvent {
            button: MouseButton::Left,
            position,
            modifiers: Modifiers::none(),
            click_count: 3,
        });
        redraw(&mut visual);
        assert_eq!(
            selected_text(window, &mut visual).as_deref(),
            Some("Alpha bravo_two charlie.")
        );
    }

    #[gpui::test]
    fn select_all_and_copy_put_the_rendered_document_on_the_clipboard(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.bind_keys([
                gpui::KeyBinding::new("cmd-a", field::SelectAll, None),
                gpui::KeyBinding::new("cmd-c", field::Copy, None),
                gpui::KeyBinding::new("cmd-a", field::SelectAll, Some("Field")),
            ])
        });
        let (window, mut visual) = selection_window(
            cx,
            "# Title\n\nSome *styled* text.\n\n- one\n- two\n\n| A | B |\n| - | - |\n| 1 | 2 |\n\n```\ncode line\n```\n\n> quoted",
        );
        visual.simulate_keystrokes("cmd-a cmd-c");
        assert_eq!(
            visual.read_from_clipboard().and_then(|item| item.text()),
            Some(
                "Title\n\nSome styled text.\n\n• one\n• two\n\nA\tB\n1\t2\n\ncode line\n\nquoted"
                    .to_owned()
            )
        );

        // Escape clears the selection.
        visual.dispatch_action(Dismiss);
        assert_eq!(selected_text(window, &mut visual), None);

        // With the find field focused, Cmd+A selects the field's text, not the document.
        visual.dispatch_action(ToggleFind);
        redraw(&mut visual);
        visual.simulate_keystrokes("x cmd-a");
        assert_eq!(selected_text(window, &mut visual), None);
    }

    #[gpui::test]
    fn edit_menu_copy_and_select_all_route_to_the_reader(cx: &mut TestAppContext) {
        let (window, mut visual) = selection_window(cx, "Menu driven text.");
        visual.dispatch_action(field::SelectAll);
        assert_eq!(
            selected_text(window, &mut visual).as_deref(),
            Some("Menu driven text.")
        );
        visual.dispatch_action(field::Copy);
        assert_eq!(
            visual.read_from_clipboard().and_then(|item| item.text()),
            Some("Menu driven text.".to_owned())
        );
    }

    #[gpui::test]
    fn reader_context_menu_offers_copy_only_with_a_selection(cx: &mut TestAppContext) {
        let (window, mut visual) = selection_window(cx, "Right click me please.");
        let position = caret(window, &mut visual, 0, 0, 2);
        visual.simulate_mouse_down(position, MouseButton::Right, Modifiers::none());
        redraw(&mut visual);
        window
            .update(&mut visual, |app, _, cx| {
                let menu = app.context_menu_view().expect("reader menu");
                assert_eq!(
                    menu.read(cx).entries(),
                    &[
                        ContextMenuEntry::disabled("Copy"),
                        ContextMenuEntry::item("Select All"),
                    ]
                );
                app.context_menu = None;
            })
            .unwrap();

        let from = caret(window, &mut visual, 0, 0, 0);
        let to = caret(window, &mut visual, 0, 0, 5);
        drag(&mut visual, from, to);
        visual.simulate_mouse_down(position, MouseButton::Right, Modifiers::none());
        redraw(&mut visual);
        window
            .update(&mut visual, |app, _, cx| {
                let menu = app.context_menu_view().expect("reader menu");
                assert_eq!(menu.read(cx).entries()[0], ContextMenuEntry::item("Copy"));
                app.run_context_action(
                    ContextAction::CopySelection(PathBuf::from("/tmp/click.md")),
                    cx,
                );
            })
            .unwrap();
        assert_eq!(
            visual.read_from_clipboard().and_then(|item| item.text()),
            Some("Right".to_owned())
        );
    }

    #[gpui::test]
    fn link_clicks_activate_but_drags_starting_on_a_link_select(cx: &mut TestAppContext) {
        let root = tempfile::tempdir().unwrap();
        let guide = root.path().join("guide.md");
        fs::write(&guide, "# Guide").unwrap();
        let (window, mut visual) =
            selection_window(cx, &format!("Read [the guide]({}) now.", guide.display()));
        // Drag from inside the link to past it: a selection, no navigation.
        let from = caret(window, &mut visual, 0, 0, 5);
        let to = caret(window, &mut visual, 0, 0, 17);
        drag(&mut visual, from, to);
        assert_eq!(
            selected_text(window, &mut visual).as_deref(),
            Some("the guide no")
        );
        window
            .update(&mut visual, |app, _, _| assert_eq!(app.model.tabs.len(), 1))
            .unwrap();

        // A drag that stays within the link still selects rather than navigating.
        let inner = caret(window, &mut visual, 0, 0, 11);
        drag(&mut visual, from, inner);
        window
            .update(&mut visual, |app, _, _| assert_eq!(app.model.tabs.len(), 1))
            .unwrap();

        // A plain click on the link opens it.
        let click = caret(window, &mut visual, 0, 0, 8);
        drag(&mut visual, click, click);
        window
            .update(&mut visual, |app, _, _| assert_eq!(app.model.tabs.len(), 2))
            .unwrap();
    }

    #[gpui::test]
    fn find_highlights_match_ranges_in_nested_blocks_and_code(cx: &mut TestAppContext) {
        let (window, mut visual) = selection_window(
            cx,
            "> - quoted needle\n>   nested\n\n```\nlet needle = 1;\n```\n\n| a | needle |\n| - | - |\n| x | y |",
        );
        visual.dispatch_action(ToggleFind);
        redraw(&mut visual);
        visual.simulate_keystrokes("n e e d l e");
        redraw(&mut visual);
        redraw(&mut visual);
        let hits = window
            .update(&mut visual, |app, _, cx| {
                app.overlays
                    .find()
                    .unwrap()
                    .read(cx)
                    .matches()
                    .hits()
                    .to_vec()
            })
            .unwrap();
        assert_eq!(hits.len(), 3);
        let pane = active_pane(window, &mut visual);
        for hit in &hits {
            let text = visual
                .update(|_, cx| pane.read(cx).painted_text(hit.surface_id()))
                .expect("every match's surface is painted");
            assert_eq!(&text[hit.range()], "needle", "{hit:?}");
            let rects =
                visual.update(|_, cx| pane.read(cx).painted_rects(hit.surface_id(), hit.range()));
            assert_eq!(rects.len(), 1);
        }
        // Nested quote/list, the code block, and the table header cell (surface 1).
        assert_eq!(
            hits.iter()
                .map(|hit| (hit.block, hit.surface))
                .collect::<Vec<_>>(),
            vec![(0, 0), (1, 0), (2, 1)]
        );
    }

    #[gpui::test]
    fn inline_math_finds_and_copies_as_its_tex_source(cx: &mut TestAppContext) {
        let (window, mut visual) = selection_window(cx, "Area $x^2$ grows.\n\n$$\ny = 1\n$$");
        visual.dispatch_action(ToggleFind);
        redraw(&mut visual);
        visual.simulate_keystrokes("x ^ 2");
        redraw(&mut visual);
        redraw(&mut visual);
        let hits = window
            .update(&mut visual, |app, _, cx| {
                app.overlays
                    .find()
                    .unwrap()
                    .read(cx)
                    .matches()
                    .hits()
                    .to_vec()
            })
            .unwrap();
        assert_eq!(hits.len(), 1);
        let hit = hits[0];
        assert_eq!((hit.block, hit.surface, hit.range()), (0, 0, 6..9));
        let pane = active_pane(window, &mut visual);
        let rects =
            visual.update(|_, cx| pane.read(cx).painted_rects(hit.surface_id(), hit.range()));
        assert_eq!(rects.len(), 1, "the match covers the typeset formula");

        visual.dispatch_action(Dismiss);
        visual.dispatch_action(field::SelectAll);
        assert_eq!(
            selected_text(window, &mut visual).as_deref(),
            Some("Area $x^2$ grows.\n\n$$y = 1$$")
        );
    }

    #[gpui::test]
    fn dragging_past_the_viewport_edge_autoscrolls_and_keeps_the_anchor(cx: &mut TestAppContext) {
        let window = long_reader_window(cx);
        let mut visual = VisualTestContext::from_window(*window, cx);
        redraw(&mut visual);
        redraw(&mut visual);
        let from = caret(window, &mut visual, 0, 0, 0);
        let pane = active_pane(window, &mut visual);
        let viewport = visual.update(|_, cx| pane.read(cx).list_state().viewport_bounds());
        let below = point(from.x, viewport.bottom() + px(40.0));
        visual.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
        visual.simulate_mouse_move(below, Some(MouseButton::Left), Modifiers::none());
        for _ in 0..20 {
            visual.executor().advance_clock(Duration::from_millis(16));
            redraw(&mut visual);
        }
        visual.simulate_mouse_up(below, MouseButton::Left, Modifiers::none());
        redraw(&mut visual);
        assert!(active_reader_offset(window, &mut visual) < px(0.0));
        let selection = visual.update(|_, cx| pane.read(cx).selection()).unwrap();
        // The anchor stays on the first paragraph although it scrolled out of view.
        assert_eq!(selection.anchor, TextPoint::default());
        assert!(selection.head.block > 0);
    }

    #[gpui::test]
    fn stepping_find_scrolls_the_active_match_clear_of_the_find_bar(cx: &mut TestAppContext) {
        let source = (0..80)
            .map(|index| {
                if index == 60 {
                    "Paragraph with the uniquetarget word.".to_owned()
                } else {
                    format!("Paragraph {index} keeps the native reader overflowing.")
                }
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        let (window, mut visual) = selection_window(cx, &source);
        visual.dispatch_action(ToggleFind);
        redraw(&mut visual);
        visual.simulate_keystrokes("u n i q u e t a r g e t");
        for _ in 0..4 {
            redraw(&mut visual);
        }
        let hit = window
            .update(&mut visual, |app, _, cx| {
                app.overlays
                    .find()
                    .unwrap()
                    .read(cx)
                    .matches()
                    .active()
                    .unwrap()
            })
            .unwrap();
        assert_eq!(hit.block, 60);
        let pane = active_pane(window, &mut visual);
        let (rects, viewport) = visual.update(|_, cx| {
            let pane = pane.read(cx);
            (
                pane.painted_rects(hit.surface_id(), hit.range()),
                pane.list_state().viewport_bounds(),
            )
        });
        let rect = rects.first().expect("the active match is painted");
        assert!(
            rect.top() >= viewport.top() + px(60.0) && rect.bottom() <= viewport.bottom(),
            "match {rect:?} should sit below the find bar inside {viewport:?}"
        );
    }

    fn split_window(cx: &mut TestAppContext) -> (gpui::WindowHandle<MdowApp>, Vec<PathBuf>) {
        let paths = ["left", "right", "third"]
            .map(|name| PathBuf::from(format!("/tmp/split-{name}.md")))
            .to_vec();
        let long = (0..80)
            .map(|index| format!("Paragraph {index} keeps each split pane overflowing."))
            .collect::<Vec<_>>()
            .join("\n\n");
        let mut model = AppModel::default();
        for path in &paths {
            model
                .tabs
                .open(parse_document(path.clone(), format!("# Split\n\n{long}\n")));
        }
        model.tabs.activate(&paths[0]);
        let window = cx.update(|cx| {
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(gpui::WindowBounds::Windowed(gpui::Bounds::new(
                        point(px(0.0), px(0.0)),
                        gpui::size(px(1400.0), px(800.0)),
                    ))),
                    ..Default::default()
                },
                |window, cx| {
                    cx.new(|cx| {
                        let mut app = MdowApp::new(window, cx);
                        app.model = model;
                        app
                    })
                },
            )
            .unwrap()
        });
        (window, paths)
    }

    fn split_panes(
        window: gpui::WindowHandle<MdowApp>,
        visual: &mut VisualTestContext,
    ) -> (bool, Option<PathBuf>, Option<PathBuf>, PaneId, PathBuf) {
        window
            .update(visual, |app, _, _| {
                (
                    app.split.is_enabled(),
                    app.split.pane_path(PaneId::Primary).map(Path::to_owned),
                    app.split.pane_path(PaneId::Secondary).map(Path::to_owned),
                    app.split.active_pane(),
                    app.model.tabs.active().unwrap().path().to_owned(),
                )
            })
            .unwrap()
    }

    #[gpui::test]
    fn toggle_split_view_shows_two_panes_under_one_tab_strip(cx: &mut TestAppContext) {
        let (window, paths) = split_window(cx);
        let mut visual = VisualTestContext::from_window(*window, cx);
        redraw(&mut visual);
        assert!(visual.debug_bounds("split-view").is_none());
        assert!(visual.debug_bounds("breadcrumb").is_some());

        visual.dispatch_action(ToggleSplitView);
        redraw(&mut visual);
        assert_eq!(
            split_panes(window, &mut visual),
            (
                true,
                Some(paths[0].clone()),
                Some(paths[1].clone()),
                PaneId::Primary,
                paths[0].clone()
            )
        );
        // Each pane has its own header; the single breadcrumb gives way, like Electron, so the
        // panes start right under the tab strip.
        let tab_bar = visual.debug_bounds("tab-bar").unwrap();
        let left = visual.debug_bounds("split-pane-primary").unwrap();
        let right = visual.debug_bounds("split-pane-secondary").unwrap();
        assert_eq!(left.top(), tab_bar.bottom());
        assert_eq!(right.top(), tab_bar.bottom());
        assert!(right.left() > left.right() - px(0.5));
        assert!((left.size.width - right.size.width).abs() <= px(1.0));
        assert!(visual.debug_bounds("split-pane-header-primary").is_some());
        assert!(visual.debug_bounds("close-split-view").is_some());

        // The tab-bar button closes it again, keeping the focused document.
        click_debug(&mut visual, "toggle-split-view");
        redraw(&mut visual);
        let (enabled, _, secondary, _, active) = split_panes(window, &mut visual);
        assert!(!enabled);
        assert_eq!(secondary, None);
        assert_eq!(active, paths[0]);
    }

    #[gpui::test]
    fn palette_command_toggles_split_view(cx: &mut TestAppContext) {
        let (window, _paths) = split_window(cx);
        let mut visual = VisualTestContext::from_window(*window, cx);
        redraw(&mut visual);
        window
            .update(cx, |app, window, cx| {
                app.run_command(CommandId::ToggleSplitView, window, cx)
            })
            .unwrap();
        assert!(split_panes(window, &mut visual).0);
        assert!(
            command_catalog()
                .iter()
                .any(|spec| spec.id == CommandId::ToggleSplitView
                    && spec.title == "Toggle Split View")
        );
    }

    #[gpui::test]
    fn tab_menu_assigns_documents_to_the_left_and_right_panes(cx: &mut TestAppContext) {
        let (window, paths) = split_window(cx);
        let mut visual = VisualTestContext::from_window(*window, cx);
        redraw(&mut visual);
        let third = visual.debug_bounds("document-tab-2").unwrap().center();
        visual.simulate_mouse_down(third, MouseButton::Right, Modifiers::none());
        redraw(&mut visual);
        // Close, Close Others, Close to the Right, Close All, -, Open in Left, Open in Right.
        click_debug(&mut visual, "context-menu-item-6");
        redraw(&mut visual);
        assert_eq!(
            split_panes(window, &mut visual),
            (
                true,
                Some(paths[0].clone()),
                Some(paths[2].clone()),
                PaneId::Secondary,
                paths[2].clone()
            )
        );

        window
            .update(cx, |app, _, cx| {
                app.run_context_action(
                    ContextAction::OpenInPane(paths[1].clone(), PaneId::Primary),
                    cx,
                )
            })
            .unwrap();
        assert_eq!(
            split_panes(window, &mut visual),
            (
                true,
                Some(paths[1].clone()),
                Some(paths[2].clone()),
                PaneId::Primary,
                paths[1].clone()
            )
        );

        // Clicking a tab retargets only the focused (left) pane.
        click_debug(&mut visual, "document-tab-0");
        let (_, primary, secondary, _, _) = split_panes(window, &mut visual);
        assert_eq!(primary, Some(paths[0].clone()));
        assert_eq!(secondary, Some(paths[2].clone()));
    }

    #[gpui::test]
    fn clicking_a_pane_focuses_it_for_find_outline_and_keyboard_scrolling(cx: &mut TestAppContext) {
        let (window, paths) = split_window(cx);
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.dispatch_action(ToggleSplitView);
        redraw(&mut visual);
        redraw(&mut visual);

        let right = visual
            .debug_bounds("split-pane-secondary")
            .unwrap()
            .center();
        visual.simulate_mouse_down(right, MouseButton::Left, Modifiers::none());
        visual.simulate_mouse_up(right, MouseButton::Left, Modifiers::none());
        redraw(&mut visual);
        let (_, _, _, pane, active) = split_panes(window, &mut visual);
        assert_eq!(pane, PaneId::Secondary);
        assert_eq!(active, paths[1]);

        // Keyboard scrolling moves only the focused pane; each pane keeps its own offset.
        visual.simulate_keystrokes("down down");
        let offsets = |visual: &mut VisualTestContext| {
            window
                .update(visual, |app, _, cx| {
                    (
                        app.reader_list_state(&paths[0], cx)
                            .unwrap()
                            .scroll_px_offset_for_scrollbar()
                            .y,
                        app.reader_list_state(&paths[1], cx)
                            .unwrap()
                            .scroll_px_offset_for_scrollbar()
                            .y,
                    )
                })
                .unwrap()
        };
        assert_eq!(offsets(&mut visual), (px(0.0), px(-80.0)));

        // Find opens against the focused document and follows focus to the other pane.
        visual.dispatch_action(ToggleFind);
        let find_document = |visual: &mut VisualTestContext| {
            window
                .update(visual, |app, _, cx| {
                    app.overlays
                        .find()
                        .unwrap()
                        .read(cx)
                        .document_path()
                        .map(Path::to_owned)
                })
                .unwrap()
        };
        assert_eq!(find_document(&mut visual), Some(paths[1].clone()));
        let left = visual.debug_bounds("split-pane-primary").unwrap().center();
        visual.simulate_mouse_down(left, MouseButton::Left, Modifiers::none());
        visual.simulate_mouse_up(left, MouseButton::Left, Modifiers::none());
        assert_eq!(find_document(&mut visual), Some(paths[0].clone()));
        visual.simulate_keystrokes("escape");
        visual.simulate_keystrokes("down");
        assert_eq!(offsets(&mut visual), (px(-40.0), px(-80.0)));

        // The outline follows the focused document.
        window
            .update(cx, |app, _, cx| {
                assert_eq!(app.model.tabs.active().unwrap().path(), paths[0]);
                assert!(app.active_outline_heading(cx).is_some());
            })
            .unwrap();
    }

    #[gpui::test]
    fn dragging_the_divider_resizes_the_panes_and_double_click_resets(cx: &mut TestAppContext) {
        let (window, _paths) = split_window(cx);
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.dispatch_action(ToggleSplitView);
        redraw(&mut visual);
        let split = visual.debug_bounds("split-view").unwrap();
        let divider = visual.debug_bounds("split-divider").unwrap().center();
        let before = visual
            .debug_bounds("split-pane-primary")
            .unwrap()
            .size
            .width;

        visual.simulate_mouse_move(divider, None, Modifiers::none());
        visual.simulate_mouse_down(divider, MouseButton::Left, Modifiers::none());
        let target = point(divider.x - px(150.0), divider.y);
        visual.simulate_mouse_move(
            point(divider.x - px(20.0), divider.y),
            Some(MouseButton::Left),
            Modifiers::none(),
        );
        visual.simulate_mouse_move(target, Some(MouseButton::Left), Modifiers::none());
        visual.simulate_mouse_up(target, MouseButton::Left, Modifiers::none());
        redraw(&mut visual);
        let after = visual
            .debug_bounds("split-pane-primary")
            .unwrap()
            .size
            .width;
        assert!(
            (after - (before - px(150.0))).abs() <= px(1.5),
            "dragged {before:?} -> {after:?}"
        );

        // Dragging far left stops at the minimum pane width.
        let divider = visual.debug_bounds("split-divider").unwrap().center();
        visual.simulate_mouse_down(divider, MouseButton::Left, Modifiers::none());
        let far = point(split.left() + px(10.0), divider.y);
        visual.simulate_mouse_move(
            point(divider.x - px(20.0), divider.y),
            Some(MouseButton::Left),
            Modifiers::none(),
        );
        visual.simulate_mouse_move(far, Some(MouseButton::Left), Modifiers::none());
        visual.simulate_mouse_up(far, MouseButton::Left, Modifiers::none());
        redraw(&mut visual);
        let min = visual
            .debug_bounds("split-pane-primary")
            .unwrap()
            .size
            .width;
        assert!(
            (min - px(crate::split::MIN_PANE_WIDTH)).abs() <= px(1.0),
            "{min:?}"
        );

        // Double-click puts it back to an even split.
        let divider = visual.debug_bounds("split-divider").unwrap().center();
        visual.simulate_event(gpui::MouseDownEvent {
            position: divider,
            button: MouseButton::Left,
            modifiers: Modifiers::none(),
            click_count: 1,
            first_mouse: false,
        });
        visual.simulate_event(gpui::MouseUpEvent {
            position: divider,
            button: MouseButton::Left,
            modifiers: Modifiers::none(),
            click_count: 1,
        });
        visual.simulate_event(gpui::MouseDownEvent {
            position: divider,
            button: MouseButton::Left,
            modifiers: Modifiers::none(),
            click_count: 2,
            first_mouse: false,
        });
        visual.simulate_event(gpui::MouseUpEvent {
            position: divider,
            button: MouseButton::Left,
            modifiers: Modifiers::none(),
            click_count: 2,
        });
        redraw(&mut visual);
        let reset = visual
            .debug_bounds("split-pane-primary")
            .unwrap()
            .size
            .width;
        assert!((reset - before).abs() <= px(1.0), "{reset:?} vs {before:?}");
    }

    #[gpui::test]
    fn split_view_is_saved_in_the_session_and_restored(cx: &mut TestAppContext) {
        let (window, paths) = split_window(cx);
        let mut visual = VisualTestContext::from_window(*window, cx);
        redraw(&mut visual);
        window
            .update(cx, |app, _, cx| {
                app.open_in_pane(&paths[2], PaneId::Secondary, cx);
                app.split.drag_divider_to(600.0, 1000.0);
            })
            .unwrap();
        let session = window
            .update(cx, |app, _, _| app.session_snapshot())
            .unwrap();
        let saved = session.split.clone().expect("split saved");
        assert_eq!(saved.primary, paths[0]);
        assert_eq!(saved.secondary, paths[2]);
        assert_eq!(saved.active_pane, PaneId::Secondary);
        assert_eq!(saved.ratio, 0.6);

        // A fresh window restores the panes from the session (documents reopen from disk).
        let root = markdown_workspace();
        let first = root.path().join("README.md").canonicalize().unwrap();
        let second = root.path().join("guides/start.md").canonicalize().unwrap();
        let restored = Session::from_parts(
            [first.clone(), second.clone()],
            Some(first.clone()),
            None,
            Recents::default(),
            None,
        )
        .with_split(Some(crate::split::SessionSplit {
            primary: first.clone(),
            secondary: second.clone(),
            active_pane: PaneId::Secondary,
            ratio: 0.4,
        }));
        let fresh = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.restore_session(restored, cx);
                    app
                })
            })
            .unwrap()
        });
        fresh
            .update(cx, |app, _, _| {
                assert!(app.split.is_enabled());
                assert_eq!(app.split.pane_path(PaneId::Primary), Some(first.as_path()));
                assert_eq!(
                    app.split.pane_path(PaneId::Secondary),
                    Some(second.as_path())
                );
                assert_eq!(app.split.active_pane(), PaneId::Secondary);
                assert_eq!(app.split.ratio(), 0.4);
                assert_eq!(app.model.tabs.active().unwrap().path(), second);
            })
            .unwrap();
    }

    #[gpui::test]
    fn closing_a_pane_document_backfills_then_collapses_the_split(cx: &mut TestAppContext) {
        let (window, paths) = split_window(cx);
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.dispatch_action(ToggleSplitView);
        redraw(&mut visual);
        window
            .update(cx, |app, _, cx| app.close_tab(&paths[1], cx))
            .unwrap();
        let (enabled, primary, secondary, _, _) = split_panes(window, &mut visual);
        assert!(enabled);
        assert_eq!(primary, Some(paths[0].clone()));
        assert_eq!(secondary, Some(paths[2].clone()));

        window
            .update(cx, |app, _, cx| app.close_tab(&paths[2], cx))
            .unwrap();
        redraw(&mut visual);
        let (enabled, primary, secondary, _, active) = split_panes(window, &mut visual);
        assert!(!enabled);
        assert_eq!(primary, Some(paths[0].clone()));
        assert_eq!(secondary, None);
        assert_eq!(active, paths[0]);
    }

    #[gpui::test]
    fn holding_command_alone_shows_the_cheat_sheet_after_the_electron_delay(
        cx: &mut TestAppContext,
    ) {
        let (window, _first, _second, _root) = two_tab_window(cx);
        let mut visual = VisualTestContext::from_window(*window, cx);
        redraw(&mut visual);
        let visible = |visual: &mut VisualTestContext| {
            window
                .update(visual, |app, _, _| app.cheat_sheet_visible())
                .unwrap()
        };

        visual.simulate_modifiers_change(Modifiers::command());
        visual
            .executor()
            .advance_clock(cheat_sheet::HOLD_DELAY - Duration::from_millis(50));
        visual.run_until_parked();
        assert!(!visible(&mut visual), "too early");
        visual.executor().advance_clock(Duration::from_millis(60));
        visual.run_until_parked();
        assert!(visible(&mut visual));
        redraw(&mut visual);
        let sheet = visual
            .debug_bounds("cheat-sheet")
            .expect("cheat sheet painted");
        let viewport = visual.update(|window, _| window.viewport_size());
        // Bottom-centred like Electron's `bottom-6 left-1/2 -translate-x-1/2`.
        assert!((sheet.center().x - viewport.width / 2.0).abs() <= px(1.0));
        assert!((viewport.height - sheet.bottom() - px(24.0)).abs() <= px(4.0));

        // Releasing ⌘ dismisses it.
        visual.simulate_modifiers_change(Modifiers::none());
        assert!(!visible(&mut visual));

        // Any other key while it is up dismisses it too.
        visual.simulate_modifiers_change(Modifiers::command());
        visual.executor().advance_clock(cheat_sheet::HOLD_DELAY);
        visual.run_until_parked();
        assert!(visible(&mut visual));
        visual.simulate_event(KeyDownEvent {
            keystroke: Keystroke::parse("cmd-j").unwrap(),
            is_held: false,
        });
        assert!(!visible(&mut visual));
        visual.simulate_modifiers_change(Modifiers::none());
    }

    #[gpui::test]
    fn command_combos_and_open_modals_never_show_the_cheat_sheet(cx: &mut TestAppContext) {
        let (window, _first, _second, _root) = two_tab_window(cx);
        let mut visual = VisualTestContext::from_window(*window, cx);
        redraw(&mut visual);
        let visible = |visual: &mut VisualTestContext| {
            window
                .update(visual, |app, _, _| app.cheat_sheet_visible())
                .unwrap()
        };

        // A ⌘ combo (⌘2 switches tabs in the app); holding ⌘ afterwards does not pop the sheet.
        visual.simulate_modifiers_change(Modifiers::command());
        visual.simulate_keystrokes("cmd-2");
        visual.executor().advance_clock(cheat_sheet::HOLD_DELAY * 2);
        visual.run_until_parked();
        assert!(!visible(&mut visual));
        visual.simulate_modifiers_change(Modifiers::none());

        // Releasing early cancels the pending peek.
        visual.simulate_modifiers_change(Modifiers::command());
        visual.executor().advance_clock(Duration::from_millis(200));
        visual.simulate_modifiers_change(Modifiers::none());
        visual.executor().advance_clock(cheat_sheet::HOLD_DELAY);
        visual.run_until_parked();
        assert!(!visible(&mut visual));

        // ⌘⇧ is a chord, not a peek.
        visual.simulate_modifiers_change(Modifiers::command_shift());
        visual.executor().advance_clock(cheat_sheet::HOLD_DELAY);
        visual.run_until_parked();
        assert!(!visible(&mut visual));
        visual.simulate_modifiers_change(Modifiers::none());

        // With the palette open the sheet stays away.
        visual.dispatch_action(TogglePalette);
        visual.simulate_modifiers_change(Modifiers::command());
        visual.executor().advance_clock(cheat_sheet::HOLD_DELAY);
        visual.run_until_parked();
        assert!(!visible(&mut visual));
    }
}

#[cfg(test)]
#[path = "pipeline_tests.rs"]
mod pipeline_tests;
