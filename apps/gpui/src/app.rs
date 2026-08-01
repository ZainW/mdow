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
        welcome::welcome,
    },
    workspace::{WorkspaceError, WorkspaceTree, scan_workspace},
};
use gpui::{
    Context, ExternalPaths, FocusHandle, Focusable, IntoElement, PathPromptOptions, Render,
    Subscription, Window, div, prelude::*, px,
};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserFacingError {
    pub title: String,
    pub body: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppOpenError {
    pub view: UserFacingError,
}

impl From<DocumentError> for AppOpenError {
    fn from(error: DocumentError) -> Self {
        Self {
            view: UserFacingError {
                title: error.title().into(),
                body: error.body().into(),
                path: error.path().to_owned(),
            },
        }
    }
}

impl From<WorkspaceError> for AppOpenError {
    fn from(error: WorkspaceError) -> Self {
        Self {
            view: UserFacingError {
                title: error.title().into(),
                body: error.body().into(),
                path: error.path().to_owned(),
            },
        }
    }
}

#[derive(Debug, Default)]
pub struct AppModel {
    pub tabs: TabSet,
    pub workspace: Option<WorkspaceTree>,
}

impl AppModel {
    pub fn open_path(&mut self, path: &Path) -> Result<(), AppOpenError> {
        if path.is_dir() {
            let workspace = scan_workspace(path)?;
            self.workspace = Some(workspace);
        } else {
            let loaded = load_source(path)?;
            self.tabs
                .open(parse_document(loaded.canonical_path, loaded.source));
        }
        Ok(())
    }

    pub fn open_paths<I, P>(&mut self, paths: I) -> Result<(), AppOpenError>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut first_error = None;
        for path in paths {
            if let Err(error) = self.open_path(path.as_ref())
                && first_error.is_none()
            {
                first_error = Some(error);
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

pub struct MdowApp {
    pub model: AppModel,
    pub sidebar_open: bool,
    pub wide_mode: bool,
    pub drop_active: bool,
    pub open_error: Option<UserFacingError>,
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
            model
                .open_path(Path::new(&path))
                .err()
                .map(|error| error.view)
        });

        Self {
            model,
            sidebar_open: true,
            wide_mode: false,
            drop_active: false,
            open_error,
            theme: Theme::for_appearance(window.appearance()),
            focus_handle,
            _appearance_subscription: appearance_subscription,
        }
    }

    pub fn open_path(&mut self, path: &Path, cx: &mut Context<Self>) {
        match self.model.open_path(path) {
            Ok(()) => self.open_error = None,
            Err(error) => self.open_error = Some(error.view),
        }
        cx.notify();
    }

    fn open_paths(&mut self, paths: impl IntoIterator<Item = PathBuf>, cx: &mut Context<Self>) {
        match self.model.open_paths(paths) {
            Ok(()) => self.open_error = None,
            Err(error) => self.open_error = Some(error.view),
        }
        self.drop_active = false;
        cx.notify();
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
                this.update(cx, |this, cx| this.open_paths(paths, cx)).ok();
            }
            Ok(Ok(None)) => {}
            Ok(Err(_)) | Err(_) => {
                this.update(cx, |this, cx| {
                    this.open_error = Some(UserFacingError {
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
            cx.notify();
        }
    }

    pub(crate) fn close_tab(&mut self, path: &Path, cx: &mut Context<Self>) {
        if self.model.tabs.close(path).is_some() {
            cx.notify();
        }
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
            active_path.as_deref(),
            layout.sidebar.width,
            cx,
        );
        let tab_bar = render_tab_bar(self.theme, self, cx);
        let breadcrumb = render_breadcrumb(self.theme, self);
        let content = if self.model.tabs.is_empty() {
            if let Some(error) = self.open_error.as_ref() {
                render_error_state(self.theme, error, self.drop_active)
            } else {
                welcome(self.theme, self.drop_active)
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
            surface.child(div().flex_grow()).into_any_element()
        };
        let drop_theme = self.theme;

        div()
            .id("mdow-root")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::open_file))
            .on_action(cx.listener(Self::open_folder))
            .on_action(cx.listener(Self::toggle_sidebar))
            .on_action(cx.listener(Self::close_active_tab))
            .on_action(cx.listener(Self::toggle_wide_mode))
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
    use std::{fs, path::Path};

    fn markdown_workspace() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("guides")).unwrap();
        fs::write(root.path().join("README.md"), "# Home").unwrap();
        fs::write(root.path().join("guides/start.md"), "# Start").unwrap();
        root
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

        assert_eq!(error.view.title, "This file is not UTF-8");
        assert_eq!(error.view.path, invalid);
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

        let error = model
            .open_paths([unsupported.as_path(), first.as_path(), second.as_path()])
            .unwrap_err();

        assert_eq!(error.view.title, "Unsupported file type");
        assert_eq!(error.view.path, unsupported);
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

        assert_eq!(error.view.title, "File not found");
        assert_eq!(error.view.body, "This file may have been moved or renamed.");
        assert_eq!(error.view.path, missing);
        assert!(!error.view.body.contains("DocumentError"));
    }
}
