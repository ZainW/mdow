use crate::{
    actions::{CloseTab, OpenFile, OpenFolder, ToggleSidebar, ToggleWideMode},
    document::{DocumentError, load_source, parse_document},
    tabs::TabSet,
    theme::{Metrics, ShellLayout, Theme},
    ui::{
        chrome::{
            render_breadcrumb, render_error_banner, render_error_state, render_sidebar,
            render_tab_bar,
        },
        reader::{LinkRoute, classify_link, clear_expired_code_copy_feedback, render_document},
        welcome::welcome,
    },
    workspace::{WorkspaceError, WorkspaceTree, scan_workspace},
};
use gpui::{
    ClipboardItem, Context, ExternalPaths, FocusHandle, Focusable, IntoElement, PathPromptOptions,
    Render, ScrollHandle, Subscription, Timer, Window, div, prelude::*, px,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
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
}

impl BatchOpenResult {
    pub fn document_attempted(&self) -> bool {
        self.document_attempted
    }
}

impl AppModel {
    pub fn open_document(&mut self, path: &Path) -> Result<(), AppOpenError> {
        let loaded = load_source(path)?;
        self.tabs
            .open(parse_document(loaded.canonical_path, loaded.source));
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
                if let Err(AppOpenError::Document(error)) = self.open_document(path)
                    && result.document_error.is_none()
                {
                    result.document_error = Some(error);
                }
            }
        }
        if workspace_attempted {
            self.workspace_error = result.workspace_error.clone();
        }
        result
    }
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
    hovered_link: Option<(usize, usize)>,
    reader_scroll_handles: HashMap<PathBuf, ScrollHandle>,
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

        let mut model = AppModel::default();
        let open_error = std::env::args_os().nth(1).and_then(|path| {
            model.open_path(Path::new(&path)).err().and_then(|error| {
                if matches!(error, AppOpenError::Document(_)) {
                    Some(error.into_view())
                } else {
                    None
                }
            })
        });

        Self {
            model,
            sidebar_open: true,
            wide_mode: false,
            drop_state: DropState::default(),
            open_error,
            copied_code: None,
            hovered_link: None,
            reader_scroll_handles: HashMap::new(),
            theme: Theme::for_appearance(window.appearance()),
            focus_handle,
            _appearance_subscription: appearance_subscription,
        }
    }

    pub fn open_path(&mut self, path: &Path, cx: &mut Context<Self>) {
        match self.model.open_path(path) {
            Ok(()) if !path.is_dir() => {
                self.open_error = None;
                self.copied_code = None;
                self.hovered_link = None;
            }
            Ok(()) => {}
            Err(AppOpenError::Document(error)) => self.open_error = Some(error),
            Err(AppOpenError::Workspace(_)) => {}
        }
        cx.notify();
    }

    fn open_paths(&mut self, paths: impl IntoIterator<Item = PathBuf>, cx: &mut Context<Self>) {
        let result = self.model.open_paths(paths);
        if result.document_attempted() {
            self.open_error = result.document_error;
        }
        self.drop_state.dropped();
        cx.notify();
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

    pub fn close_active_tab(&mut self, _: &CloseTab, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = self.model.tabs.active().map(|tab| tab.path().to_owned()) {
            self.model.tabs.close(&path);
            self.copied_code = None;
            self.hovered_link = None;
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
            self.copied_code = None;
            self.hovered_link = None;
            cx.notify();
        }
    }

    pub(crate) fn close_tab(&mut self, path: &Path, cx: &mut Context<Self>) {
        if self.model.tabs.close(path).is_some() {
            self.copied_code = None;
            self.hovered_link = None;
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
        hovered_link: Option<(usize, usize)>,
        cx: &mut Context<Self>,
    ) {
        if self.hovered_link != hovered_link {
            self.hovered_link = hovered_link;
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
                surface = surface.child(
                    div()
                        .id("reload-error-banner")
                        .debug_selector(|| "reload-error-banner".into())
                        .child(render_error_banner(
                            self.theme,
                            &UserFacingError {
                                title: "Couldn't reload this file".into(),
                                body,
                                path: path.clone(),
                            },
                        )),
                );
            }
            let scroll_handle = self.reader_scroll_handles.entry(path).or_default().clone();
            surface
                .child(render_document(
                    document,
                    self.wide_mode,
                    self.theme,
                    self.copied_code,
                    self.hovered_link,
                    &scroll_handle,
                    cx,
                ))
                .into_any_element()
        };
        let drop_theme = self.theme;

        div()
            .id("mdow-root")
            .track_focus(&self.focus_handle)
            .capture_key_down(|event, window, cx| {
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
                    window.prevent_default();
                    cx.stop_propagation();
                }
            })
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
        FileDropEvent, KeyUpEvent, Keystroke, Modifiers, MouseButton, TestAppContext,
        VisualTestContext, point,
    };
    use std::{fs, os::unix::fs::PermissionsExt, path::Path};

    fn markdown_workspace() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("guides")).unwrap();
        fs::write(root.path().join("README.md"), "# Home").unwrap();
        fs::write(root.path().join("guides/start.md"), "# Start").unwrap();
        root
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
        let link_surface = visual
            .debug_bounds("reader-inline-0-0")
            .expect("single-link inline surface should be painted");
        let blank_focus_point = point(link_surface.right() - px(2.0), link_surface.center().y);
        visual.simulate_mouse_move(blank_focus_point, None, Modifiers::none());
        visual.simulate_mouse_down(blank_focus_point, MouseButton::Left, Modifiers::none());
        visual.simulate_mouse_up(blank_focus_point, MouseButton::Left, Modifiers::none());
        assert!(visual.update(|window, cx| window.focused(cx).is_some()));

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
                    app.hovered_link = Some((1, 0));
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
            })
            .unwrap();
    }
}
