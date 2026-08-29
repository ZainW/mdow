use crate::{
    actions::{
        CloseTab, Dismiss, FindNext, FindPrevious, OpenFile, OpenFolder, SidebarFolder,
        SidebarOutline, SidebarRecents, ToggleFind, TogglePalette, ToggleSettings, ToggleShortcuts,
        ToggleSidebar, ToggleWideMode, ZoomIn, ZoomOut, ZoomReset,
    },
    document::{DocumentError, ParsedDocument, load_source, parse_document},
    overlay::{
        CommandId, FindEvent, FindOverlay, OpenOverlay, OverlayHost, OverlayKind, PaletteAction,
        PaletteEvent, PaletteOverlay, SettingsEvent, SettingsPanel, ShortcutsCard, ShortcutsEvent,
    },
    persist::{StateStore, StoredPrefs},
    prefs::{ColumnWidth, PrefEdit, Prefs, SidebarMode, ThemeMode},
    session::{Recents, SavedWindowBounds, Session},
    syntax::prepare_document,
    tabs::TabSet,
    theme::{Metrics, ShellLayout, Theme},
    ui::{
        chrome::{
            render_breadcrumb, render_error_banner, render_error_state, render_reload_error_banner,
            render_sidebar, render_tab_bar,
        },
        reader::{
            LinkFocusKey, LinkRoute, LinkSurfaceKey, ReaderPane, classify_link,
            clear_expired_code_copy_feedback, document_link_focus_targets,
        },
        welcome::welcome,
    },
    watcher::{FileWatcher, WatchMessage},
    workspace::{WorkspaceError, WorkspaceTree, scan_workspace},
};
use gpui::{
    App, ClipboardItem, Context, Entity, ExternalPaths, FocusHandle, Focusable, IntoElement,
    PathPromptOptions, Render, Subscription, Task, Timer, Window, div, prelude::*, px,
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

#[derive(Debug, Default)]
pub struct AppModel {
    pub tabs: TabSet,
    pub workspace: Option<WorkspaceTree>,
    pub workspace_error: Option<UserFacingError>,
    pub recents: Recents,
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
        let parsed = parse_document(loaded.canonical_path, loaded.source);
        self.tabs.open_prepared(prepare_document(parsed));
        if let Some(tab) = self.tabs.active() {
            self.recents.note(tab.path());
        }
        Ok(())
    }

    pub fn reload_path(&mut self, path: &Path) -> Result<(), AppOpenError> {
        let tab_path = canonical_file_identity(path);
        let loaded = match load_source(path) {
            Ok(loaded) => loaded,
            Err(error) => {
                let error = AppOpenError::from(error);
                self.tabs
                    .set_reload_error(&tab_path, error.view().body.clone());
                return Err(error);
            }
        };
        let parsed = parse_document(loaded.canonical_path, loaded.source);
        self.tabs.replace_prepared(prepare_document(parsed));
        Ok(())
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

    pub fn open_path(&mut self, path: &Path) -> Result<(), AppOpenError> {
        if path.is_dir() {
            self.open_workspace(path)
        } else {
            self.open_document(path)
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
                match self.open_document(path) {
                    Ok(()) => result.document_opened = true,
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

    pub fn dismiss_active_reload_error(&mut self) -> bool {
        let Some(tab) = self.tabs.active().filter(|tab| tab.reload_error.is_some()) else {
            return false;
        };
        self.tabs.replace_prepared((*tab.document).clone())
    }
}

fn canonical_file_identity(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| {
        path.parent()
            .and_then(|parent| parent.canonicalize().ok())
            .and_then(|parent| path.file_name().map(|name| parent.join(name)))
            .unwrap_or_else(|| path.to_owned())
    })
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DropState {
    active: bool,
}

impl DropState {
    pub fn is_active(self) -> bool {
        self.active
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

pub struct MdowApp {
    pub model: AppModel,
    pub sidebar_open: bool,
    pub wide_mode: bool,
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
    file_watcher: FileWatcher,
    _watch_messages: Arc<Mutex<Receiver<WatchMessage>>>,
    _watch_poll_task: Task<()>,
    theme: Theme,
    focus_handle: FocusHandle,
    _appearance_subscription: Subscription,
}

pub(crate) struct ReaderPaintState {
    pub copied_code: Option<(usize, Instant)>,
    pub hovered_link: Option<LinkFocusKey>,
    pub focused_link: Option<LinkFocusKey>,
    pub find_block: Option<usize>,
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
        Self::boot(Prefs::default(), StateStore::in_memory(), window, cx)
    }

    pub fn boot(
        prefs: Prefs,
        store: StateStore,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        focus_handle.focus(window);
        let appearance_subscription = cx.observe_window_appearance(window, |this, window, cx| {
            this.theme = Theme::resolve(this.prefs.get().theme_mode, window.appearance());
            cx.notify();
        });
        let file_watcher = FileWatcher::new().expect("create Mdow file watcher");
        let watch_messages = file_watcher.messages();
        let poll_messages = watch_messages.clone();
        let watch_poll_task = cx.spawn(async move |this, cx| {
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
                        let mut changed = false;
                        for WatchMessage::Reload(path) in messages {
                            if this.model.tabs.get(&path).is_some() {
                                let _ = this.model.reload_path(&path);
                                changed = true;
                            }
                        }
                        if changed {
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        });

        let wide_mode = prefs.reader_width.is_full();
        Self {
            model: AppModel::default(),
            sidebar_open: true,
            wide_mode,
            prefs: StoredPrefs::restore(prefs, store),
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
            _watch_messages: watch_messages,
            _watch_poll_task: watch_poll_task,
            theme: Theme::for_appearance(window.appearance()),
            focus_handle,
            _appearance_subscription: appearance_subscription,
        }
    }

    pub fn open_path(&mut self, path: &Path, cx: &mut Context<Self>) {
        match self.model.open_path(path) {
            Ok(()) if !path.is_dir() => {
                let watch_error = self
                    .model
                    .tabs
                    .active()
                    .map(|tab| tab.path().to_owned())
                    .and_then(|path| self.watch_document(&path).err());
                self.open_error = watch_error;
                self.active_document_changed(cx);
            }
            Ok(()) => {}
            Err(AppOpenError::Document(error)) => self.open_error = Some(error),
            Err(AppOpenError::Workspace(_)) => {}
        }
        cx.notify();
    }

    fn open_paths(&mut self, paths: impl IntoIterator<Item = PathBuf>, cx: &mut Context<Self>) {
        let result = self.model.open_paths(paths);
        let document_opened = result.document_opened();
        let watch_error = document_opened
            .then(|| self.watch_all_documents())
            .flatten();
        if result.document_attempted() {
            self.open_error = result.document_error.or(watch_error);
        }
        if document_opened {
            self.active_document_changed(cx);
        }
        self.drop_state.dropped();
        cx.notify();
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
        self.file_watcher
            .watch(path)
            .map_err(|error| UserFacingError {
                title: "Couldn't watch this file".into(),
                body: error.to_string(),
                path: path.to_owned(),
            })
    }

    fn open_workspace_path(&mut self, path: &Path, cx: &mut Context<Self>) {
        self.model.open_workspace(path).ok();
        self.apply_pref(PrefEdit::Sidebar(SidebarMode::Folder), cx);
        self.prefs.save_session(&self.session_snapshot());
        cx.notify();
    }

    fn drag_moved(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.drop_state.enter() {
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
        let _ = std::process::Command::new("open").arg("-R").arg(path).spawn();
    }

    pub fn restore_session(&mut self, session: Session, cx: &mut Context<Self>) {
        self.model.recents = session.recents.clone();
        if let Some(folder) = session.last_folder.as_ref() {
            self.model.open_workspace(folder).ok();
        }
        if let Some(tabs) = session.tabs.as_ref() {
            for path in tabs.iter() {
                let _ = self.model.open_document(path);
            }
            self.model.tabs.activate(tabs.active());
            let _ = self.watch_all_documents();
        }
        self.last_window_bounds = session.window;
        self.clear_reader_transient_state();
        cx.notify();
    }

    fn apply_pref(&mut self, edit: PrefEdit, cx: &mut Context<Self>) {
        let session = self.session_snapshot();
        if !self.prefs.apply(edit, &session) {
            return;
        }
        self.wide_mode = self.prefs.get().reader_width.is_full();
        self.overlays.refresh_settings(self.prefs.get(), cx);
        cx.notify();
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
                let view = cx.new(|cx| FindOverlay::new(document, window, cx));
                let events = cx.subscribe_in(&view, window, |this, _, event, _, cx| {
                    this.on_find_event(event, cx);
                });
                OpenOverlay::find(view, events)
            }
            OverlayKind::Palette => {
                let view = cx.new(|cx| PaletteOverlay::new(self.model.recents.clone(), window, cx));
                let events = cx.subscribe_in(&view, window, |this, _, event, window, cx| {
                    this.on_palette_event(event, window, cx);
                });
                OpenOverlay::palette(view, events)
            }
            OverlayKind::Settings => {
                let view = cx.new(|cx| SettingsPanel::new(*self.prefs.get(), window, cx));
                let events = cx.subscribe_in(&view, window, |this, _, event, _, cx| {
                    this.on_settings_event(event, cx);
                });
                OpenOverlay::settings(view, events)
            }
            OverlayKind::Shortcuts => {
                let view = cx.new(|cx| ShortcutsCard::new(window, cx));
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
        if self.overlays.close(Some(window)) {
            cx.notify();
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
            CommandId::ToggleSidebar => self.toggle_sidebar(&ToggleSidebar, window, cx),
            CommandId::SidebarRecents => {
                self.apply_pref(PrefEdit::Sidebar(SidebarMode::Recents), cx)
            }
            CommandId::SidebarFolder => self.apply_pref(PrefEdit::Sidebar(SidebarMode::Folder), cx),
            CommandId::SidebarOutline => {
                self.apply_pref(PrefEdit::Sidebar(SidebarMode::Outline), cx)
            }
            CommandId::ToggleWideMode => self.apply_pref(PrefEdit::ToggleFull, cx),
            CommandId::ColumnStandard => {
                self.apply_pref(PrefEdit::Column(ColumnWidth::Standard), cx)
            }
            CommandId::ColumnComfortable => {
                self.apply_pref(PrefEdit::Column(ColumnWidth::Comfortable), cx)
            }
            CommandId::ColumnWide => self.apply_pref(PrefEdit::Column(ColumnWidth::Wide), cx),
            CommandId::ThemeSystem => self.apply_pref(PrefEdit::Theme(ThemeMode::System), cx),
            CommandId::ThemeLight => self.apply_pref(PrefEdit::Theme(ThemeMode::Light), cx),
            CommandId::ThemeDark => self.apply_pref(PrefEdit::Theme(ThemeMode::Dark), cx),
            CommandId::ZoomIn => self.apply_pref(PrefEdit::ZoomIn, cx),
            CommandId::ZoomOut => self.apply_pref(PrefEdit::ZoomOut, cx),
            CommandId::ZoomReset => self.apply_pref(PrefEdit::ZoomReset, cx),
            CommandId::FindInDocument => self.toggle_overlay(OverlayKind::Find, window, cx),
            CommandId::OpenSettings => self.toggle_overlay(OverlayKind::Settings, window, cx),
            CommandId::OpenShortcuts => self.toggle_overlay(OverlayKind::Shortcuts, window, cx),
        }
    }

    fn active_document_changed(&mut self, cx: &mut Context<Self>) {
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
            FindEvent::ActiveHit(hit) => self.scroll_reader_to_block(hit.block, cx),
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
        self.apply_pref(PrefEdit::ZoomIn, cx);
    }

    fn on_zoom_out(&mut self, _: &ZoomOut, _: &mut Window, cx: &mut Context<Self>) {
        self.apply_pref(PrefEdit::ZoomOut, cx);
    }

    fn on_zoom_reset(&mut self, _: &ZoomReset, _: &mut Window, cx: &mut Context<Self>) {
        self.apply_pref(PrefEdit::ZoomReset, cx);
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

    pub(crate) fn reader_paint_state(&self, cx: &App) -> ReaderPaintState {
        ReaderPaintState {
            copied_code: self.copied_code,
            hovered_link: self.hovered_link,
            focused_link: self.focused_link,
            find_block: self
                .overlays
                .find()
                .and_then(|find| find.read(cx).matches().active().map(|hit| hit.block)),
        }
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
        self.reader_panes.retain(|open_path, _| live.contains(open_path));
        self.reader_link_focus_handles
            .retain(|(open_path, _), _| live.contains(open_path));
        if let Some(pane) = self.reader_panes.get(&path).cloned() {
            if !pane.read(cx).hosts_document(&document) {
                self.retain_reader_link_focus_handles(&document, window);
            }
            pane.update(cx, |pane, cx| pane.sync(document, style, theme, cx));
            pane
        } else {
            self.retain_reader_link_focus_handles(&document, window);
            let app = cx.weak_entity();
            let pane = cx.new(|_| ReaderPane::new(app, document, style, theme));
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
        document: &ParsedDocument,
        window: &mut Window,
    ) {
        let active_keys = document_link_focus_targets(document)
            .into_iter()
            .map(|target| target.key)
            .collect::<HashSet<_>>();
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
            self.model.tabs.close(&path);
            self.reader_panes.remove(&path);
            self.reader_link_focus_handles
                .retain(|(document_path, _), _| document_path != &path);
            self.active_document_changed(cx);
            cx.notify();
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

    fn scroll_active_reader(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        let Some(path) = self.model.tabs.active().map(|tab| tab.path().to_owned()) else {
            return false;
        };
        let Some(pane) = self.reader_panes.get(&path).cloned() else {
            return false;
        };
        let scrolled = pane.update(cx, |pane, cx| {
            let scrolled = pane.scroll_by_key(key);
            if scrolled {
                cx.notify();
            }
            scrolled
        });
        scrolled
    }

    pub(crate) fn close_tab(&mut self, path: &Path, cx: &mut Context<Self>) {
        if self.model.tabs.close(path).is_some() {
            self.reader_panes.remove(path);
            self.reader_link_focus_handles
                .retain(|(document_path, _), _| document_path != path);
            self.active_document_changed(cx);
            cx.notify();
        }
    }

    pub(crate) fn jump_to_heading(&mut self, text: &str, cx: &mut Context<Self>) {
        let Some(document) = self.model.tabs.active().map(|tab| tab.document.clone()) else {
            return;
        };
        let mut heading_index = 0usize;
        for (block_index, block) in document.blocks.iter().enumerate() {
            if matches!(block, crate::document::DocumentBlock::Heading { .. }) {
                if document
                    .headings
                    .get(heading_index)
                    .is_some_and(|heading| heading.text == text)
                {
                    self.scroll_reader_to_block(block_index, cx);
                    return;
                }
                heading_index += 1;
            }
        }
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

impl Focusable for MdowApp {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MdowApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bounds = window.bounds();
        self.last_window_bounds = Some(SavedWindowBounds {
            x: f32::from(bounds.origin.x),
            y: f32::from(bounds.origin.y),
            width: f32::from(bounds.size.width),
            height: f32::from(bounds.size.height),
        });
        self.theme = Theme::resolve(self.prefs.get().theme_mode, window.appearance());
        let layout = ShellLayout::for_width(
            f32::from(window.viewport_size().width),
            self.sidebar_open,
            self.wide_mode,
        );
        let active_path = self.model.tabs.active().map(|tab| tab.path().to_owned());
        let headings = self
            .model
            .tabs
            .active()
            .map(|tab| tab.document.headings.as_slice());
        let sidebar = render_sidebar(
            self.theme,
            self.prefs.get().sidebar_mode,
            &self.model.recents,
            self.model.workspace.as_ref(),
            self.model.workspace_error.as_ref(),
            headings,
            active_path.as_deref(),
            layout.sidebar.width,
            cx,
        );
        let tab_bar = render_tab_bar(self.theme, self, cx);
        let breadcrumb = render_breadcrumb(self.theme, self, cx);
        let active_tab = self.model.tabs.active().map(|tab| {
            (
                tab.document.clone(),
                tab.path().to_owned(),
                tab.reload_error.clone(),
            )
        });
        let content = if self.model.tabs.is_empty() {
            if let Some(error) = self.open_error.as_ref() {
                render_error_state(self.theme, error, self.drop_state.is_active(), cx)
            } else {
                welcome(self.theme, self.drop_state.is_active(), cx)
            }
        } else {
            let mut surface = div()
                .flex()
                .flex_col()
                .flex_grow()
                .min_w_0()
                .min_h_0()
                .bg(self.theme.background);
            if let Some(error) = self.open_error.as_ref() {
                surface = surface.child(render_error_banner(self.theme, error));
            }
            let (document, path, reload_error) =
                active_tab.expect("a non-empty tab set always has an active document");
            if let Some(body) = reload_error {
                surface = surface.child(render_reload_error_banner(
                    self.theme,
                    &UserFacingError {
                        title: "Couldn't reload this file".into(),
                        body,
                        path: path.clone(),
                    },
                    cx,
                ));
            }
            let pane = self.ensure_reader_pane(document, window, cx);
            surface.child(pane).into_any_element()
        };
        let drop_theme = self.theme;

        div()
            .id("mdow-root")
            .track_focus(&self.focus_handle)
            .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                let modifiers = event.keystroke.modifiers;
                if reader_key_modifiers_are_allowed(
                    &event.keystroke.key,
                    modifiers.control,
                    modifiers.alt,
                    modifiers.platform,
                    modifiers.function,
                ) && this.scroll_active_reader(&event.keystroke.key, cx)
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
            .on_action(cx.listener(Self::on_toggle_find))
            .on_action(cx.listener(Self::on_toggle_palette))
            .on_action(cx.listener(Self::on_toggle_settings))
            .on_action(cx.listener(Self::on_toggle_shortcuts))
            .on_action(cx.listener(Self::on_dismiss))
            .on_action(cx.listener(Self::on_find_next))
            .on_action(cx.listener(Self::on_find_previous))
            .on_action(cx.listener(Self::on_zoom_in))
            .on_action(cx.listener(Self::on_zoom_out))
            .on_action(cx.listener(Self::on_zoom_reset))
            .on_action(cx.listener(Self::on_sidebar_recents))
            .on_action(cx.listener(Self::on_sidebar_folder))
            .on_action(cx.listener(Self::on_sidebar_outline))
            .on_drag_move::<ExternalPaths>(cx.listener(|this, _, window, cx| {
                this.drag_moved(window, cx);
            }))
            .drag_over::<ExternalPaths>(move |style, _, _, _| {
                style
                    .bg(drop_theme.primary.opacity(0.06))
                    .border_1()
                    .border_color(drop_theme.primary.opacity(0.46))
            })
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| {
                this.open_paths(paths.paths().to_vec(), cx);
            }))
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .bg(self.theme.background)
            .font_family(Metrics::FONT_SANS)
            .text_size(px(self.prefs.get().interface_scale.tokens().control_font))
            .text_color(self.theme.foreground)
            .child(
                div()
                    .h(px(Metrics::TITLEBAR_INSET))
                    .w_full()
                    .flex_none()
                    .border_b_1()
                    .border_color(self.theme.border_subtle)
                    .bg(self.theme.background),
            )
            .child(
                div()
                    .flex()
                    .flex_grow()
                    .min_h_0()
                    .when(layout.sidebar.width > 0.0, |shell| shell.child(sidebar))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .min_w_0()
                            .min_h_0()
                            .flex_grow()
                            .child(tab_bar)
                            .child(breadcrumb)
                            .child(content),
                    ),
            )
            .children(self.overlays.render_layer(self.theme))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::reader::reader_key_target;
    use gpui::{
        FileDropEvent, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, MouseButton, ScrollDelta,
        ScrollWheelEvent, TestAppContext, VisualTestContext, point,
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
        tempfile::Builder::new()
            .prefix("mdow-app-watch-")
            .tempdir_in("/private/tmp")
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
    fn nested_workspace_read_failure_preserves_workspace_and_tabs_with_sidebar_error() {
        let root = markdown_workspace();
        let file = root.path().join("README.md");
        let failing = tempfile::tempdir().unwrap();
        let denied = failing.path().join("denied");
        fs::create_dir(&denied).unwrap();
        fs::write(denied.join("hidden.md"), "# Hidden").unwrap();
        let canonical_denied = denied.canonicalize().unwrap();
        let _restore = PermissionRestore::deny(&denied);
        let mut model = AppModel::default();
        model.open_workspace(root.path()).unwrap();
        model.open_document(&file).unwrap();
        let workspace_path = model.workspace.as_ref().unwrap().root.path.clone();

        let error = model.open_workspace(failing.path()).unwrap_err();

        assert_eq!(error.view().title, "Couldn't read folder");
        assert_eq!(error.view().path, canonical_denied);
        assert_eq!(model.workspace.as_ref().unwrap().root.path, workspace_path);
        assert_eq!(model.tabs.len(), 1);
        assert_eq!(model.tabs.active().unwrap().document.title, "Home");
        assert_eq!(model.workspace_error.as_ref(), Some(error.view()));
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
            .update(cx, |app, _, cx| {
                app.reader_list_state(&first, cx).unwrap()
            })
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
        assert_eq!(
            scroll_handle.scroll_px_offset_for_scrollbar().y,
            px(-64.0)
        );
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

    #[gpui::test]
    fn tab_close_target_is_reachable_and_activatable_by_keyboard(cx: &mut TestAppContext) {
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
        let mut visual = VisualTestContext::from_window(*window, cx);

        // The nested close target follows the top-level controls in GPUI's grouped tab order.
        focus_next(&mut visual, 9);
        activate_focused(&mut visual, "space");

        window
            .update(cx, |app, _, _| {
                assert_eq!(app.model.tabs.len(), 1);
                assert_eq!(
                    app.model.tabs.active().unwrap().path(),
                    second.canonicalize().unwrap()
                );
            })
            .unwrap();
    }

    #[gpui::test]
    fn inactive_tab_is_reachable_and_activatable_by_keyboard(cx: &mut TestAppContext) {
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
        let mut visual = VisualTestContext::from_window(*window, cx);

        // Open-folder, sidebar-toggle, first tab, then second tab.
        focus_next(&mut visual, 4);
        activate_focused(&mut visual, "enter");

        window
            .update(cx, |app, _, _| {
                assert_eq!(app.model.tabs.len(), 2);
                assert_eq!(
                    app.model.tabs.active().unwrap().path(),
                    second.canonicalize().unwrap()
                );
            })
            .unwrap();
    }

    #[gpui::test]
    fn tab_rail_keeps_the_sidebar_toggle_and_tab_list_in_measured_slots(cx: &mut TestAppContext) {
        let document = parse_document(
            PathBuf::from("/tmp/measured-tab.md"),
            "# Measured tab\n".into(),
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

        let toggle_slot = visual
            .debug_bounds("sidebar-toggle-slot")
            .expect("fixed sidebar toggle slot");
        let tabs = visual.debug_bounds("tabs-scroll").expect("tab list");
        let tab = visual
            .debug_bounds("document-tab-0")
            .expect("first document tab");

        assert_eq!(toggle_slot.size.width, px(36.0));
        assert_eq!(tabs.origin.x, toggle_slot.origin.x + toggle_slot.size.width);
        assert_eq!(tab.origin.x, tabs.origin.x + px(6.0));
        assert_eq!(tab.size.height, px(28.0));
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

        // The nested disclosure follows the top-level controls in GPUI's grouped tab order.
        focus_next(&mut visual, 10);
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
            px(4.0),
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
        for _ in 0..12 {
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

        for _ in 0..12 {
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
            .update(cx, |app, window, _| {
                assert!(app.focused_link.is_none());
                assert_eq!(app.reader_link_focus_handles.len(), 1);
                assert!(!old_handle.is_focused(window));
                assert_eq!(app.model.tabs.len(), 1);
                assert_eq!(app.model.tabs.active().unwrap().path(), canonical);
            })
            .unwrap();

        fs::write(&path, "No links remain.\n").unwrap();
        window
            .update(cx, |app, _, cx| app.open_path(&path, cx))
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
            .update(cx, |app, _, cx| app.open_path(&path, cx))
            .unwrap();
        visual.update(|window, cx| window.draw(cx).clear());
        window
            .update(cx, |app, _, _| {
                assert_eq!(app.reader_link_focus_handles.len(), 1)
            })
            .unwrap();

        for _ in 0..12 {
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

        click_debug(&mut visual, "toggle-settings");
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
        click_debug(&mut visual, "palette-item-3");
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
}
