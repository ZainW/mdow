use crate::{
    actions::{OpenFolder, ToggleSidebar, ToggleWideMode},
    app::{MdowApp, UserFacingError},
    theme::{Metrics, Theme},
    ui::{
        primitives::{compact_icon_button, icon, icon_button},
        welcome::error_state,
    },
    workspace::{WorkspaceEntryKind, WorkspaceTree},
};
use gpui::{
    AnyElement, Context, FontWeight, IntoElement, StatefulInteractiveElement, Transformation, div,
    percentage, prelude::*, px,
};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreadcrumbSegment {
    pub name: String,
    pub path: PathBuf,
}

pub fn breadcrumb_segments(path: &Path) -> Vec<BreadcrumbSegment> {
    let Some(parent) = path.parent() else {
        return Vec::new();
    };
    let mut current = PathBuf::new();
    let mut segments = Vec::new();
    for component in parent.components() {
        current.push(component.as_os_str());
        if let std::path::Component::Normal(name) = component {
            segments.push(BreadcrumbSegment {
                name: name.to_string_lossy().into_owned(),
                path: current.clone(),
            });
        }
    }
    segments
        .into_iter()
        .rev()
        .take(3)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

pub fn render_sidebar(
    theme: Theme,
    workspace: Option<&WorkspaceTree>,
    workspace_error: Option<&UserFacingError>,
    active_path: Option<&Path>,
    width: f32,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let folder_name = workspace
        .map(|tree| tree.root.name.clone())
        .unwrap_or_else(|| "No folder".into());
    let rows = workspace
        .map(WorkspaceTree::visible_rows)
        .unwrap_or_default();

    let mut tree = div()
        .id("workspace-scroll")
        .flex()
        .flex_col()
        .flex_grow()
        .min_h_0()
        .overflow_y_scroll()
        .scrollbar_width(px(4.0))
        .px(px(4.0))
        .py(px(4.0));

    if rows.is_empty() {
        tree = tree.child(
            div()
                .px(px(12.0))
                .pt(px(36.0))
                .text_center()
                .font_family(Metrics::FONT_SANS)
                .text_size(px(12.0))
                .line_height(px(18.0))
                .text_color(theme.muted_foreground)
                .child(if workspace.is_some() {
                    "No Markdown files in this folder."
                } else {
                    "Open a folder to browse its Markdown files."
                }),
        );
    } else {
        for (index, row) in rows.into_iter().enumerate() {
            let is_active = row.kind == WorkspaceEntryKind::File
                && active_path.is_some_and(|path| path == row.path);
            let marker_color = if is_active {
                theme.accent
            } else {
                theme.accent.opacity(0.0)
            };
            let icon_path = match row.kind {
                WorkspaceEntryKind::Directory if row.expanded => "icons/folder-open.svg",
                WorkspaceEntryKind::Directory => "icons/folder.svg",
                WorkspaceEntryKind::File => "icons/file.svg",
            };
            let row_path = row.path.clone();
            let click_path = row.path.clone();
            let directory = row.kind == WorkspaceEntryKind::Directory;
            let disclosure_icon = if row.expanded {
                "icons/chevron-right.svg"
            } else {
                "icons/chevron-right.svg"
            };

            let disclosure = div()
                .id(("workspace-disclosure", index))
                .debug_selector(move || format!("workspace-disclosure-{index}"))
                .flex()
                .items_center()
                .justify_center()
                .size(px(18.0))
                .flex_none()
                .rounded(px(4.0))
                .cursor_pointer()
                .hover(move |style| style.bg(theme.muted))
                .when(directory, |button| {
                    button
                        .tab_index(0)
                        .focusable()
                        .active(|style| style.opacity(0.78))
                        .focus(move |style| style.border_1().border_color(theme.primary))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.toggle_directory(&row_path, cx);
                        }))
                })
                .when(!directory, |space| space.cursor_default())
                .child(
                    icon(disclosure_icon, theme.muted_foreground.opacity(0.7), 10.0).when(
                        row.expanded,
                        |chevron| {
                            chevron.with_transformation(Transformation::rotate(percentage(0.25)))
                        },
                    ),
                )
                .when(!directory, |space| space.invisible());

            let row_element = div()
                .id(("workspace-row", index))
                .debug_selector(move || format!("workspace-row-{index}"))
                .tab_index(0)
                .tab_group()
                .focusable()
                .flex()
                .w_full()
                .min_w_0()
                .rounded(px(5.0))
                .bg(if is_active {
                    theme.sidebar_accent
                } else {
                    theme.sidebar_accent.opacity(0.0)
                })
                .cursor_pointer()
                .hover(move |style| style.bg(theme.sidebar_accent))
                .active(|style| style.opacity(0.82))
                .focus(move |style| style.border_1().border_color(theme.primary))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if directory {
                        this.toggle_directory(&click_path, cx);
                    } else {
                        this.open_path(&click_path, cx);
                    }
                }))
                .child(
                    div()
                        .w(px(2.0))
                        .flex_none()
                        .rounded(px(1.0))
                        .bg(marker_color),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .min_w_0()
                        .flex_grow()
                        .gap(px(4.0))
                        .py(px(4.0))
                        .pr(px(6.0))
                        .pl(px(6.0 + row.depth as f32 * 10.0))
                        .font_family(Metrics::FONT_SANS)
                        .font_weight(FontWeight::NORMAL)
                        .text_size(px(12.0))
                        .line_height(px(16.0))
                        .text_color(if is_active {
                            theme.foreground
                        } else {
                            theme.muted_foreground
                        })
                        .child(disclosure)
                        .child(icon(icon_path, theme.muted_foreground, 14.0))
                        .child(div().min_w_0().flex_grow().truncate().child(row.name)),
                );
            tree = tree.child(row_element);
        }
    }

    div()
        .flex()
        .flex_col()
        .w(px(width))
        .h_full()
        .flex_none()
        .border_r_1()
        .border_color(theme.border_subtle)
        .bg(theme.sidebar)
        .child(
            div()
                .flex()
                .items_center()
                .h(px(36.0))
                .px(px(8.0))
                .gap(px(6.0))
                .flex_none()
                .border_b_1()
                .border_color(theme.border_subtle)
                .font_family(Metrics::FONT_SANS)
                .text_size(px(12.0))
                .child(icon("icons/folder.svg", theme.muted_foreground, 14.0))
                .child(
                    div()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.foreground)
                        .child("Folder"),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_grow()
                        .truncate()
                        .text_size(px(11.0))
                        .text_color(theme.muted_foreground.opacity(0.7))
                        .child(folder_name),
                )
                .child(compact_icon_button(
                    "sidebar-open-folder",
                    "icons/folder-open.svg",
                    24.0,
                    14.0,
                    theme,
                    |_, _, cx| cx.dispatch_action(&OpenFolder),
                )),
        )
        .when_some(workspace_error.cloned(), |sidebar, error| {
            sidebar.child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(7.0))
                    .mx(px(8.0))
                    .mt(px(8.0))
                    .px(px(8.0))
                    .py(px(7.0))
                    .flex_none()
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(theme.destructive.opacity(0.32))
                    .bg(theme.destructive.opacity(0.08))
                    .font_family(Metrics::FONT_SANS)
                    .text_size(px(11.0))
                    .child(icon("icons/alert-circle.svg", theme.destructive, 13.0))
                    .child(
                        div()
                            .min_w_0()
                            .flex_grow()
                            .flex()
                            .flex_col()
                            .gap(px(1.0))
                            .child(
                                div()
                                    .truncate()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.foreground)
                                    .child(error.title),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_color(theme.muted_foreground)
                                    .child(error.body),
                            ),
                    ),
            )
        })
        .child(tree)
        .into_any_element()
}

pub fn render_tab_bar(theme: Theme, app: &MdowApp, cx: &Context<MdowApp>) -> AnyElement {
    let active_path = app.model.tabs.active().map(|tab| tab.path().to_owned());
    let tab_paths = app
        .model
        .tabs
        .paths()
        .map(Path::to_owned)
        .collect::<Vec<_>>();
    let mut tabs = div()
        .id("tabs-scroll")
        .flex()
        .items_center()
        .min_w_0()
        .flex_grow()
        .h_full()
        .gap(px(1.0))
        .px(px(6.0))
        .overflow_x_scroll()
        .scrollbar_width(px(6.0));

    if tab_paths.is_empty() {
        tabs = tabs.child(
            div()
                .h(px(Metrics::TAB_HEIGHT))
                .flex()
                .items_center()
                .px(px(4.0))
                .font_family(Metrics::FONT_SANS)
                .text_size(px(11.0))
                .text_color(theme.muted_foreground.opacity(0.72))
                .child("No document"),
        );
    } else {
        for (index, path) in tab_paths.into_iter().enumerate() {
            let is_active = active_path.as_deref() == Some(path.as_path());
            let filename = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Untitled".into());
            let activate_path = path.clone();
            let close_path = path.clone();
            let tab = div()
                .id(("document-tab", index))
                .debug_selector(move || format!("document-tab-{index}"))
                .tab_index(0)
                .tab_group()
                .focusable()
                .flex()
                .items_center()
                .h(px(Metrics::TAB_HEIGHT))
                .max_w(px(Metrics::TAB_MAX_WIDTH))
                .min_w(px(92.0))
                .flex_none()
                .rounded(px(Metrics::RADIUS))
                .border_1()
                .border_color(if is_active {
                    theme.border_subtle
                } else {
                    theme.border_subtle.opacity(0.0)
                })
                .bg(if is_active {
                    theme.card
                } else {
                    theme.card.opacity(0.0)
                })
                .when(is_active, |tab| tab.shadow_sm())
                .font_family(Metrics::FONT_SANS)
                .font_weight(FontWeight::NORMAL)
                .text_size(px(12.0))
                .text_color(if is_active {
                    theme.foreground
                } else {
                    theme.muted_foreground
                })
                .cursor_pointer()
                .hover(move |style| style.bg(theme.muted))
                .active(|style| style.opacity(0.82))
                .focus(move |style| style.border_color(theme.primary))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.activate_tab(&activate_path, cx);
                }))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .min_w_0()
                        .flex_grow()
                        .gap(px(6.0))
                        .pl(px(10.0))
                        .child(icon(
                            "icons/file.svg",
                            theme
                                .muted_foreground
                                .opacity(if is_active { 0.82 } else { 0.62 }),
                            14.0,
                        ))
                        .child(div().min_w_0().flex_grow().truncate().child(filename)),
                )
                .child(
                    div()
                        .id(("close-document-tab", index))
                        .debug_selector(move || format!("close-document-tab-{index}"))
                        .tab_index(0)
                        .focusable()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(24.0))
                        .mr(px(2.0))
                        .flex_none()
                        .rounded(px(4.0))
                        .text_color(theme.muted_foreground)
                        .cursor_pointer()
                        .hover(move |style| style.bg(theme.muted).text_color(theme.foreground))
                        .active(|style| style.opacity(0.76))
                        .focus(move |style| style.border_1().border_color(theme.primary))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.close_tab(&close_path, cx);
                        }))
                        .child(icon("icons/x.svg", theme.muted_foreground, 12.0)),
                );
            tabs = tabs.child(tab);
        }
    }

    div()
        .flex()
        .items_center()
        .h(px(Metrics::TAB_BAR_HEIGHT))
        .flex_none()
        .border_b_1()
        .border_color(theme.border_subtle)
        .bg(theme.background)
        .child(icon_button(
            "toggle-sidebar",
            "icons/sidebar.svg",
            theme,
            |_, _, cx| cx.dispatch_action(&ToggleSidebar),
        ))
        .child(tabs)
        .into_any_element()
}

pub fn render_breadcrumb(theme: Theme, app: &MdowApp) -> AnyElement {
    let Some(tab) = app.model.tabs.active() else {
        return div()
            .flex()
            .items_center()
            .h(px(Metrics::BREADCRUMB_HEIGHT))
            .px(px(12.0))
            .flex_none()
            .border_b_1()
            .border_color(theme.border_subtle)
            .font_family(Metrics::FONT_SANS)
            .text_size(px(11.0))
            .text_color(theme.muted_foreground)
            .child("Welcome")
            .into_any_element();
    };

    let segments = breadcrumb_segments(tab.path());
    let mut trail = div()
        .flex()
        .items_center()
        .min_w_0()
        .flex_grow()
        .gap(px(2.0))
        .overflow_hidden();
    for segment in segments {
        trail = trail
            .child(
                div()
                    .max_w(px(128.0))
                    .truncate()
                    .text_color(theme.muted_foreground.opacity(0.78))
                    .child(segment.name),
            )
            .child(icon(
                "icons/chevron-right.svg",
                theme.muted_foreground.opacity(0.38),
                10.0,
            ));
    }
    trail = trail.child(
        div()
            .min_w_0()
            .truncate()
            .px(px(2.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(theme.foreground.opacity(0.88))
            .child(tab.document.title.clone()),
    );

    div()
        .flex()
        .items_center()
        .h(px(Metrics::BREADCRUMB_HEIGHT))
        .px(px(12.0))
        .gap(px(8.0))
        .flex_none()
        .border_b_1()
        .border_color(theme.border_subtle)
        .bg(theme.background)
        .font_family(Metrics::FONT_SANS)
        .font_weight(FontWeight::NORMAL)
        .text_size(px(11.0))
        .child(trail)
        .child(compact_icon_button(
            "toggle-wide-mode",
            "icons/expand.svg",
            20.0,
            12.0,
            theme,
            |_, _, cx| cx.dispatch_action(&ToggleWideMode),
        ))
        .into_any_element()
}

pub fn render_error_banner(theme: Theme, error: &UserFacingError) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .mx(px(10.0))
        .mt(px(8.0))
        .px(px(10.0))
        .py(px(7.0))
        .flex_none()
        .rounded(px(7.0))
        .border_1()
        .border_color(theme.destructive.opacity(0.35))
        .bg(theme.destructive.opacity(0.08))
        .font_family(Metrics::FONT_SANS)
        .text_size(px(11.0))
        .text_color(theme.foreground)
        .child(icon("icons/alert-circle.svg", theme.destructive, 14.0))
        .child(
            div()
                .font_weight(FontWeight::MEDIUM)
                .child(error.title.clone()),
        )
        .child(
            div()
                .min_w_0()
                .flex_grow()
                .truncate()
                .text_color(theme.muted_foreground)
                .child(error.body.clone()),
        )
        .into_any_element()
}

pub fn render_reload_error_banner(
    theme: Theme,
    error: &UserFacingError,
    cx: &Context<MdowApp>,
) -> AnyElement {
    div()
        .id("reload-error-banner")
        .debug_selector(|| "reload-error-banner".into())
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .mx(px(10.0))
                .mt(px(8.0))
                .px(px(10.0))
                .py(px(7.0))
                .flex_none()
                .rounded(px(7.0))
                .border_1()
                .border_color(theme.destructive.opacity(0.35))
                .bg(theme.destructive.opacity(0.08))
                .font_family(Metrics::FONT_SANS)
                .text_size(px(11.0))
                .text_color(theme.foreground)
                .child(icon("icons/alert-circle.svg", theme.destructive, 14.0))
                .child(
                    div()
                        .font_weight(FontWeight::MEDIUM)
                        .child(error.title.clone()),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_grow()
                        .truncate()
                        .text_color(theme.muted_foreground)
                        .child(error.body.clone()),
                )
                .child(compact_icon_button(
                    "dismiss-reload-error",
                    "icons/x.svg",
                    22.0,
                    12.0,
                    theme,
                    cx.listener(|this, _, _, cx| this.dismiss_reload_error(cx)),
                )),
        )
        .into_any_element()
}

pub fn render_error_state(theme: Theme, error: &UserFacingError, drop_active: bool) -> AnyElement {
    error_state(theme, error, drop_active)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breadcrumb_keeps_only_the_final_three_parent_segments() {
        let segments = breadcrumb_segments(Path::new("/Users/zain/vault/guides/rust/start.md"));

        assert_eq!(
            segments
                .iter()
                .map(|segment| segment.name.as_str())
                .collect::<Vec<_>>(),
            vec!["vault", "guides", "rust"]
        );
        assert_eq!(segments[2].path, Path::new("/Users/zain/vault/guides/rust"));
    }

    #[test]
    fn breadcrumb_handles_a_document_without_parent_segments() {
        assert!(breadcrumb_segments(Path::new("README.md")).is_empty());
    }
}
