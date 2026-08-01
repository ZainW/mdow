use crate::{
    actions::{CloseTab, OpenFile, OpenFolder, ToggleSidebar, ToggleWideMode},
    document::DocumentError,
    tabs::TabSet,
    theme::{Metrics, ShellLayout, Theme},
    ui::{
        primitives::{brand_logo, icon, icon_button},
        welcome::welcome,
    },
    workspace::WorkspaceTree,
};
use gpui::{
    Context, FocusHandle, Focusable, IntoElement, Render, Subscription, Window, div, prelude::*, px,
};

pub struct MdowApp {
    pub tabs: TabSet,
    pub workspace: Option<WorkspaceTree>,
    pub sidebar_open: bool,
    pub wide_mode: bool,
    pub drop_active: bool,
    pub document_error: Option<DocumentError>,
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

        Self {
            tabs: TabSet::default(),
            workspace: None,
            sidebar_open: true,
            wide_mode: false,
            drop_active: false,
            document_error: None,
            theme: Theme::for_appearance(window.appearance()),
            focus_handle,
            _appearance_subscription: appearance_subscription,
        }
    }

    fn open_file(&mut self, _: &OpenFile, _: &mut Window, _: &mut Context<Self>) {
        // Native path prompting and file operations arrive in Task 5.
    }

    fn open_folder(&mut self, _: &OpenFolder, _: &mut Window, _: &mut Context<Self>) {
        // Native path prompting and folder operations arrive in Task 5.
    }

    fn toggle_sidebar(&mut self, _: &ToggleSidebar, _: &mut Window, cx: &mut Context<Self>) {
        self.sidebar_open = !self.sidebar_open;
        cx.notify();
    }

    fn toggle_wide_mode(&mut self, _: &ToggleWideMode, _: &mut Window, cx: &mut Context<Self>) {
        self.wide_mode = !self.wide_mode;
        cx.notify();
    }

    fn close_active_tab(&mut self, _: &CloseTab, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = self.tabs.active().map(|tab| tab.path().to_owned()) {
            self.tabs.close(&path);
            cx.notify();
        }
    }

    fn render_sidebar(&self, width: f32) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w(px(width))
            .h_full()
            .flex_none()
            .border_r_1()
            .border_color(self.theme.border_subtle)
            .bg(self.theme.sidebar)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .h(px(36.0))
                    .px(px(12.0))
                    .border_b_1()
                    .border_color(self.theme.border_subtle)
                    .font_family(Metrics::FONT_SANS)
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_size(px(11.0))
                    .text_color(self.theme.muted_foreground)
                    .child(brand_logo(18.0))
                    .child("LIBRARY"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .px(px(24.0))
                    .pt(px(52.0))
                    .text_center()
                    .font_family(Metrics::FONT_SANS)
                    .text_size(px(12.0))
                    .line_height(px(18.0))
                    .text_color(self.theme.muted_foreground)
                    .child("Open a folder to browse its Markdown files."),
            )
    }

    fn render_tab_bar(&self) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .h(px(Metrics::TAB_BAR_HEIGHT))
            .px(px(7.0))
            .gap(px(6.0))
            .flex_none()
            .border_b_1()
            .border_color(self.theme.border_subtle)
            .bg(self.theme.background)
            .child(icon_button(
                "toggle-sidebar",
                "icons/sidebar.svg",
                self.theme,
                |_, _, cx| cx.dispatch_action(&ToggleSidebar),
            ))
            .child(
                div()
                    .h(px(Metrics::TAB_HEIGHT))
                    .flex()
                    .items_center()
                    .px(px(9.0))
                    .font_family(Metrics::FONT_SANS)
                    .text_size(px(11.0))
                    .text_color(self.theme.muted_foreground)
                    .child("No document"),
            )
    }

    fn render_breadcrumb(&self) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .h(px(Metrics::BREADCRUMB_HEIGHT))
            .px(px(13.0))
            .gap(px(6.0))
            .flex_none()
            .border_b_1()
            .border_color(self.theme.border_subtle)
            .font_family(Metrics::FONT_SANS)
            .text_size(px(11.0))
            .text_color(self.theme.muted_foreground)
            .child(icon("icons/file.svg", self.theme.muted_foreground, 13.0))
            .child("Welcome")
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

        div()
            .id("mdow-root")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::open_file))
            .on_action(cx.listener(Self::open_folder))
            .on_action(cx.listener(Self::toggle_sidebar))
            .on_action(cx.listener(Self::close_active_tab))
            .on_action(cx.listener(Self::toggle_wide_mode))
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
                    .when(self.sidebar_open && layout.sidebar.width > 0.0, |shell| {
                        shell.child(self.render_sidebar(layout.sidebar.width))
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .min_w_0()
                            .min_h_0()
                            .flex_grow()
                            .child(self.render_tab_bar())
                            .child(self.render_breadcrumb())
                            .child(welcome(self.theme, self.drop_active)),
                    ),
            )
    }
}
