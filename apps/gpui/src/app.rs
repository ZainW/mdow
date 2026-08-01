use crate::{
    actions::{CloseTab, OpenFile, OpenFolder, ToggleSidebar, ToggleWideMode},
    document::{DocumentError, ParsedDocument, load_source, parse_document},
    tabs::TabSet,
    theme::{Metrics, ShellLayout, Theme},
    ui::{
        chrome::{
            render_breadcrumb, render_error_banner, render_error_state, render_reload_error_banner,
            render_sidebar, render_tab_bar,
        },
        reader::{
            LinkFocusKey, LinkRoute, LinkSurfaceKey, ReaderLinkState, classify_link,
            clear_expired_code_copy_feedback, document_link_focus_targets, render_document,
        },
        welcome::welcome,
    },
    watcher::{FileWatcher, WatchMessage},
    workspace::{WorkspaceError, WorkspaceTree, scan_workspace},
};
use gpui::{
    ClipboardItem, Context, ExternalPaths, FocusHandle, Focusable, IntoElement, PathPromptOptions,
    Render, ScrollHandle, Subscription, Task, Timer, Window, div, prelude::*, px,
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
        self.tabs
            .open(parse_document(loaded.canonical_path, loaded.source));
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
        let document = parse_document(loaded.canonical_path, loaded.source);
        self.tabs.replace_document(document);
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
        self.tabs.replace_document((*tab.document).clone())
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
    pub drop_state: DropState,
    pub open_error: Option<UserFacingError>,
    copied_code: Option<(usize, Instant)>,
    hovered_link: Option<LinkFocusKey>,
    focused_link: Option<LinkFocusKey>,
    reader_scroll_handles: HashMap<PathBuf, ScrollHandle>,
    reader_link_focus_handles: HashMap<(PathBuf, LinkFocusKey), FocusHandle>,
    file_watcher: FileWatcher,
    _watch_messages: Arc<Mutex<Receiver<WatchMessage>>>,
    _watch_poll_task: Task<()>,
    theme: Theme,
    focus_handle: FocusHandle,
    _appearance_subscription: Subscription,
}

impl MdowApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        focus_handle.focus(window);
        let appearance_subscription = cx.observe_window_appearance(window, |this, window, cx| {
            this.theme = Theme::for_appearance(window.appearance());
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

        Self {
            model: AppModel::default(),
            sidebar_open: true,
            wide_mode: false,
            drop_state: DropState::default(),
            open_error: None,
            copied_code: None,
            hovered_link: None,
            focused_link: None,
            reader_scroll_handles: HashMap::new(),
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
                self.clear_reader_transient_state();
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
            self.clear_reader_transient_state();
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
        self.sidebar_open = !self.sidebar_open;
        cx.notify();
    }

    fn toggle_wide_mode(&mut self, _: &ToggleWideMode, _: &mut Window, cx: &mut Context<Self>) {
        self.wide_mode = !self.wide_mode;
        cx.notify();
    }

    fn clear_reader_transient_state(&mut self) {
        self.copied_code = None;
        self.hovered_link = None;
        self.focused_link = None;
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

    fn reconcile_reader_link_focus_handles(
        &mut self,
        document: &ParsedDocument,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> HashMap<LinkFocusKey, FocusHandle> {
        let targets = document_link_focus_targets(document);
        let active_keys = targets
            .iter()
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

        targets
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
            self.reader_scroll_handles.remove(&path);
            self.reader_link_focus_handles
                .retain(|(document_path, _), _| document_path != &path);
            self.clear_reader_transient_state();
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
            self.clear_reader_transient_state();
            cx.notify();
        }
    }

    pub(crate) fn close_tab(&mut self, path: &Path, cx: &mut Context<Self>) {
        if self.model.tabs.close(path).is_some() {
            self.reader_scroll_handles.remove(path);
            self.reader_link_focus_handles
                .retain(|(document_path, _), _| document_path != path);
            self.clear_reader_transient_state();
            cx.notify();
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
        self.theme = Theme::for_appearance(window.appearance());
        let layout = ShellLayout::for_width(
            f32::from(window.viewport_size().width),
            self.sidebar_open,
            self.wide_mode,
        );
        let active_path = self.model.tabs.active().map(|tab| tab.path().to_owned());
        let sidebar = render_sidebar(
            self.theme,
            self.model.workspace.as_ref(),
            self.model.workspace_error.as_ref(),
            active_path.as_deref(),
            layout.sidebar.width,
            cx,
        );
        let tab_bar = render_tab_bar(self.theme, self, cx);
        let breadcrumb = render_breadcrumb(self.theme, self);
        let active_tab = self.model.tabs.active().map(|tab| {
            (
                tab.document.clone(),
                tab.path().to_owned(),
                tab.reload_error.clone(),
            )
        });
        let content = if self.model.tabs.is_empty() {
            if let Some(error) = self.open_error.as_ref() {
                render_error_state(self.theme, error, self.drop_state.is_active())
            } else {
                welcome(self.theme, self.drop_state.is_active())
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
            let scroll_handle = self.reader_scroll_handles.entry(path).or_default().clone();
            let active_focus_handles =
                self.reconcile_reader_link_focus_handles(&document, window, cx);
            let link_state = ReaderLinkState {
                hovered: self.hovered_link,
                focused: self.focused_link,
                focus_handles: &active_focus_handles,
            };
            surface
                .child(render_document(
                    document,
                    self.wide_mode,
                    self.theme,
                    self.copied_code,
                    &link_state,
                    &scroll_handle,
                    cx,
                ))
                .into_any_element()
        };
        let drop_theme = self.theme;

        div()
            .id("mdow-root")
            .track_focus(&self.focus_handle)
            .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                let modifiers = event.keystroke.modifiers;
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
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .bg(self.theme.background)
            .font_family(Metrics::FONT_SANS)
            .text_size(px(Metrics::APP_FONT_SIZE))
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        FileDropEvent, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, MouseButton, TestAppContext,
        VisualTestContext, point,
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
            let mut visual = VisualTestContext::from_window((*window).into(), cx);
            visual.update(|window, cx| window.draw(cx).clear());
        }
        let scroll_handle = window
            .update(cx, |app, _, _| {
                app.reader_scroll_handles.get(&first).unwrap().clone()
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
            .update(cx, |app, _, _| {
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
                app.reader_scroll_handles
                    .get(&first)
                    .unwrap()
                    .set_offset(point(px(0.0), px(-64.0)));
            })
            .unwrap();
        assert_eq!(scroll_handle.offset(), point(px(0.0), px(-64.0)));
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
        let mut visual = VisualTestContext::from_window((*window).into(), cx);

        // The nested close target follows the top-level controls in GPUI's grouped tab order.
        focus_next(&mut visual, 6);
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
        let mut visual = VisualTestContext::from_window((*window).into(), cx);

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
                    app
                })
            })
            .unwrap()
        });
        let mut visual = VisualTestContext::from_window((*window).into(), cx);

        // The nested disclosure follows the top-level controls in GPUI's grouped tab order.
        focus_next(&mut visual, 7);
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
        let mut visual = VisualTestContext::from_window((*window).into(), cx);
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
                    .repeat(8),
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
        let mut visual = VisualTestContext::from_window((*window).into(), cx);
        visual.update(|window, cx| window.draw(cx).clear());

        assert!(visual.debug_bounds("reader-scroll").is_some());
        assert!(visual.debug_bounds("reader-block-0").is_some());
        let paragraph = visual
            .debug_bounds("reader-inline-1-0")
            .expect("paragraph inline surface should be painted");
        assert!(paragraph.size.height > px(40.0));
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
        let mut visual = VisualTestContext::from_window((*window).into(), cx);

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
        let mut visual = VisualTestContext::from_window((*window).into(), cx);
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
        let mut visual = VisualTestContext::from_window((*window).into(), cx);
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
        let mut visual = VisualTestContext::from_window((*window).into(), cx);

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
        let mut visual = VisualTestContext::from_window((*window).into(), cx);
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
        let mut visual = VisualTestContext::from_window((*window).into(), cx);
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
        let mut visual = VisualTestContext::from_window((*window).into(), cx);
        visual.update(|window, cx| window.draw(cx).clear());
        window
            .update(cx, |app, _, _| {
                app.reader_scroll_handles[&canonical].set_offset(point(px(0.0), px(-120.0)));
            })
            .unwrap();

        window
            .update(cx, |app, window, cx| {
                app.close_active_tab(&CloseTab, window, cx)
            })
            .unwrap();
        window
            .update(cx, |app, _, _| {
                assert!(!app.reader_scroll_handles.contains_key(&canonical));
            })
            .unwrap();

        window
            .update(cx, |app, _, cx| app.open_path(&path, cx))
            .unwrap();
        visual.update(|window, cx| window.draw(cx).clear());

        window
            .update(cx, |app, _, _| {
                assert_eq!(app.reader_scroll_handles[&canonical].offset().y, px(0.0));
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
        let mut visual = VisualTestContext::from_window((*window).into(), cx);
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
}
