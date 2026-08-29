use crate::{
    app::{MdowApp, UserFacingError},
    document::Heading,
    overlay::OverlayKind,
    prefs::SidebarMode,
    session::Recents,
    tabs::DocumentTab,
    theme::{Metrics, Theme},
    ui::{
        primitives::{compact_icon_button, icon, icon_button, outline_button},
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreadcrumbDisplay {
    pub primary: String,
    pub secondary: Option<String>,
}

pub fn breadcrumb_display(tab: &DocumentTab) -> BreadcrumbDisplay {
    let filename = tab
        .path()
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Untitled")
        .to_owned();
    match tab.document.frontmatter_title.as_ref() {
        Some(title) => BreadcrumbDisplay {
            primary: title.clone(),
            secondary: Some(filename),
        },
        None => BreadcrumbDisplay {
            primary: filename,
            secondary: None,
        },
    }
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
    mode: SidebarMode,
    recents: &Recents,
    workspace: Option<&WorkspaceTree>,
    workspace_error: Option<&UserFacingError>,
    headings: Option<&[Heading]>,
    active_path: Option<&Path>,
    width: f32,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let folder_name = workspace
        .map(|tree| tree.root.name.clone())
        .unwrap_or_else(|| "No folder".into());
    let mode_bar = div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .px(px(8.0))
        .h(px(36.0))
        .flex_none()
        .border_b_1()
        .border_color(theme.border_subtle)
        .child(sidebar_mode_chip(
            "Recents",
            "icons/clock.svg",
            mode == SidebarMode::Recents,
            SidebarMode::Recents,
            theme,
            cx,
        ))
        .child(sidebar_mode_chip(
            "Folder",
            "icons/folder.svg",
            mode == SidebarMode::Folder,
            SidebarMode::Folder,
            theme,
            cx,
        ))
        .child(sidebar_mode_chip(
            "Outline",
            "icons/list.svg",
            mode == SidebarMode::Outline,
            SidebarMode::Outline,
            theme,
            cx,
        ));
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
        let empty_state = if workspace.is_some() {
            div()
                .px(px(12.0))
                .pt(px(36.0))
                .text_center()
                .font_family(Metrics::FONT_SANS)
                .text_size(px(12.0))
                .line_height(px(18.0))
                .text_color(theme.muted_foreground)
                .child("No Markdown files in this folder.")
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .items_center()
                .pt(px(36.0))
                .px(px(20.0))
                .child(icon(
                    "icons/folder.svg",
                    theme.muted_foreground.opacity(0.55),
                    22.0,
                ))
                .child(
                    div()
                        .mt(px(10.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_size(px(13.0))
                        .text_color(theme.foreground)
                        .child("No folder open"),
                )
                .child(
                    div()
                        .mt(px(6.0))
                        .max_w(px(190.0))
                        .text_center()
                        .text_size(px(12.0))
                        .line_height(px(18.0))
                        .text_color(theme.muted_foreground)
                        .child("Open or drop a folder to browse its Markdown files."),
                )
                .child(div().mt(px(14.0)).child(outline_button(
                    "sidebar-empty-open-folder",
                    "Open Folder",
                    "icons/folder-open.svg",
                    theme,
                    cx.listener(|this, _, _, cx| this.open_folder_prompt(cx)),
                )))
                .into_any_element()
        };
        tree = tree.child(empty_state);
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
            let disclosure_icon = "icons/chevron-right.svg";

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
        .child(mode_bar)
        .when(mode == SidebarMode::Folder, |sidebar| {
            sidebar.child(
                div()
                    .flex()
                    .items_center()
                    .h(px(28.0))
                    .px(px(8.0))
                    .gap(px(6.0))
                    .flex_none()
                    .font_family(Metrics::FONT_SANS)
                    .text_size(px(11.0))
                    .text_color(theme.muted_foreground)
                    .child(
                        div()
                            .min_w_0()
                            .flex_grow()
                            .truncate()
                            .child(if workspace.is_some() {
                                folder_name.clone()
                            } else {
                                "No folder".into()
                            }),
                    )
                    .child(compact_icon_button(
                        "sidebar-open-folder",
                        "icons/folder-open.svg",
                        24.0,
                        14.0,
                        theme,
                        cx.listener(|this, _, _, cx| this.open_folder_prompt(cx)),
                    )),
            )
        })
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
        .child(match mode {
            SidebarMode::Folder => tree.into_any_element(),
            SidebarMode::Recents => render_recents_list(theme, recents, active_path, cx),
            SidebarMode::Outline => {
                render_outline_list(theme, headings.unwrap_or(&[]), active_path.is_some(), cx)
            }
        })
        .child(
            div()
                .flex()
                .flex_none()
                .border_t_1()
                .border_color(theme.border_subtle)
                .p(px(8.0))
                .child(
                    div()
                        .id("sidebar-settings")
                        .debug_selector(|| "sidebar-settings".into())
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .h(px(28.0))
                        .px(px(8.0))
                        .w_full()
                        .rounded(px(6.0))
                        .cursor_pointer()
                        .hover(move |style| style.bg(theme.sidebar_accent))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.click_toggle_overlay(OverlayKind::Settings, window, cx);
                        }))
                        .child(icon("icons/settings.svg", theme.muted_foreground, 14.0))
                        .child(
                            div()
                                .font_family(Metrics::FONT_SANS)
                                .text_size(px(12.0))
                                .text_color(theme.muted_foreground)
                                .child("Settings"),
                        ),
                ),
        )
        .into_any_element()
}

fn sidebar_empty(
    theme: Theme,
    icon_path: &'static str,
    title: &'static str,
    hint: &'static str,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .pt(px(36.0))
        .px(px(20.0))
        .child(icon(icon_path, theme.muted_foreground.opacity(0.55), 22.0))
        .child(
            div()
                .mt(px(10.0))
                .font_weight(FontWeight::MEDIUM)
                .text_size(px(13.0))
                .text_color(theme.foreground)
                .child(title),
        )
        .child(
            div()
                .mt(px(6.0))
                .max_w(px(190.0))
                .text_center()
                .text_size(px(12.0))
                .line_height(px(18.0))
                .text_color(theme.muted_foreground)
                .child(hint),
        )
        .into_any_element()
}

fn sidebar_mode_chip(
    label: &'static str,
    icon_path: &'static str,
    selected: bool,
    mode: SidebarMode,
    theme: Theme,
    cx: &Context<MdowApp>,
) -> impl IntoElement {
    div()
        .id(label)
        .debug_selector(move || label.to_string())
        .tab_index(0)
        .focusable()
        .px(px(6.0))
        .h(px(28.0))
        .flex()
        .flex_grow()
        .items_center()
        .justify_center()
        .gap(px(5.0))
        .min_w_0()
        .rounded(px(6.0))
        .bg(if selected {
            theme.sidebar_accent
        } else {
            theme.sidebar_accent.opacity(0.0)
        })
        .hover(move |style| {
            style.bg(theme
                .sidebar_accent
                .opacity(if selected { 1.0 } else { 0.7 }))
        })
        .font_family(Metrics::FONT_SANS)
        .text_size(px(11.0))
        .text_color(if selected {
            theme.foreground
        } else {
            theme.muted_foreground
        })
        .cursor_pointer()
        .focus(move |style| style.border_1().border_color(theme.primary))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.set_sidebar_mode(mode, cx);
        }))
        .child(icon(
            icon_path,
            if selected {
                theme.foreground
            } else {
                theme.muted_foreground
            },
            14.0,
        ))
        .child(label)
}

fn render_recents_list(
    theme: Theme,
    recents: &Recents,
    active_path: Option<&Path>,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let mut list = div()
        .id("recents-scroll")
        .flex()
        .flex_col()
        .flex_grow()
        .min_h_0()
        .overflow_y_scroll()
        .px(px(4.0))
        .py(px(4.0));
    if recents.is_empty() {
        list = list.child(sidebar_empty(
            theme,
            "icons/clock.svg",
            "No recents yet",
            "Files you open will appear here.",
        ));
    } else {
        for (index, path) in recents.iter().enumerate() {
            let path_buf = path.to_owned();
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Untitled")
                .to_owned();
            let is_active = active_path.is_some_and(|active| active == path);
            let parent = path
                .parent()
                .and_then(|parent| parent.file_name())
                .and_then(|name| name.to_str())
                .unwrap_or("")
                .to_owned();
            list = list.child(
                div()
                    .id(("recent-row", index))
                    .debug_selector(move || format!("recent-row-{index}"))
                    .flex()
                    .flex_col()
                    .justify_center()
                    .gap(px(1.0))
                    .min_h(px(36.0))
                    .px(px(8.0))
                    .py(px(5.0))
                    .rounded(px(5.0))
                    .bg(if is_active {
                        theme.sidebar_accent
                    } else {
                        theme.sidebar_accent.opacity(0.0)
                    })
                    .font_family(Metrics::FONT_SANS)
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.open_path(&path_buf, cx);
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .min_w_0()
                            .child(icon(
                                "icons/file.svg",
                                theme.muted_foreground.opacity(0.45),
                                13.0,
                            ))
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_grow()
                                    .truncate()
                                    .text_size(px(12.0))
                                    .text_color(theme.foreground)
                                    .child(name),
                            ),
                    )
                    .when(!parent.is_empty(), |row| {
                        row.child(
                            div()
                                .pl(px(19.0))
                                .truncate()
                                .text_size(px(10.0))
                                .text_color(theme.muted_foreground.opacity(0.62))
                                .child(parent),
                        )
                    }),
            );
        }
    }
    list.into_any_element()
}

fn render_outline_list(
    theme: Theme,
    headings: &[Heading],
    has_document: bool,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let mut list = div()
        .id("outline-scroll")
        .flex()
        .flex_col()
        .flex_grow()
        .min_h_0()
        .overflow_y_scroll()
        .px(px(4.0))
        .py(px(4.0));
    if headings.is_empty() {
        list = list.child(if has_document {
            sidebar_empty(
                theme,
                "icons/list.svg",
                "No headings",
                "This document has no headings to show.",
            )
        } else {
            sidebar_empty(
                theme,
                "icons/list.svg",
                "No document open",
                "Open a document to see its outline.",
            )
        });
    } else {
        for (index, heading) in headings.iter().enumerate() {
            let text = heading.text.clone();
            list = list.child(
                div()
                    .id(("outline-row", index))
                    .debug_selector(move || format!("outline-row-{index}"))
                    .h(px(28.0))
                    .px(px(8.0 + heading.level.saturating_sub(1) as f32 * 8.0))
                    .rounded(px(5.0))
                    .font_family(Metrics::FONT_SANS)
                    .text_size(px(12.0))
                    .text_color(theme.foreground)
                    .truncate()
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.jump_to_heading(&text, cx);
                    }))
                    .child(heading.text.clone()),
            );
        }
    }
    list.into_any_element()
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
        .debug_selector(|| "tabs-scroll".into())
        .flex()
        .items_start()
        .min_w_0()
        .flex_grow()
        .h_full()
        .pt(px(4.0))
        .gap(px(Metrics::TAB_GAP))
        .px(px(Metrics::TAB_LIST_INSET))
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
                .rounded(px(Metrics::TAB_RADIUS))
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
                        .gap(px(Metrics::TAB_CONTENT_GAP))
                        .pl(px(Metrics::TAB_CONTENT_INSET))
                        .child(icon(
                            "icons/file.svg",
                            theme
                                .muted_foreground
                                .opacity(if is_active { 0.82 } else { 0.62 }),
                            Metrics::TAB_ICON_SIZE,
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
                        .size(px(Metrics::TAB_CLOSE_SIZE))
                        .mr(px(Metrics::TAB_CLOSE_END_MARGIN))
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

    let toggle_slot = div()
        .debug_selector(|| "sidebar-toggle-slot".into())
        .flex()
        .items_center()
        .justify_center()
        .w(px(Metrics::TAB_TOGGLE_SLOT))
        .h_full()
        .flex_none()
        .border_r_1()
        .border_color(theme.border_subtle)
        .child(icon_button(
            "toggle-sidebar",
            "icons/sidebar.svg",
            theme,
            cx.listener(|this, _, _, cx| this.click_toggle_sidebar(cx)),
        ));

    div()
        .debug_selector(|| "tab-bar".into())
        .flex()
        .items_center()
        .h(px(Metrics::TAB_BAR_HEIGHT))
        .flex_none()
        .border_b_1()
        .border_color(theme.border_subtle)
        .bg(theme.background)
        .child(toggle_slot)
        .child(tabs)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(2.0))
                .h_full()
                .px(px(6.0))
                .flex_none()
                .border_l_1()
                .border_color(theme.border_subtle)
                .child(icon_button(
                    "toggle-find",
                    "icons/search.svg",
                    theme,
                    cx.listener(|this, _, window, cx| {
                        this.click_toggle_overlay(OverlayKind::Find, window, cx);
                    }),
                ))
                .child(icon_button(
                    "toggle-palette",
                    "icons/command.svg",
                    theme,
                    cx.listener(|this, _, window, cx| {
                        this.click_toggle_overlay(OverlayKind::Palette, window, cx);
                    }),
                ))
                .child(icon_button(
                    "toggle-settings",
                    "icons/settings.svg",
                    theme,
                    cx.listener(|this, _, window, cx| {
                        this.click_toggle_overlay(OverlayKind::Settings, window, cx);
                    }),
                )),
        )
        .into_any_element()
}

pub fn render_breadcrumb(theme: Theme, app: &MdowApp, cx: &Context<MdowApp>) -> AnyElement {
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
    let display = breadcrumb_display(tab);
    let mut trail = div()
        .flex()
        .items_center()
        .min_w_0()
        .flex_grow()
        .gap(px(2.0))
        .overflow_hidden();
    for (index, segment) in segments.into_iter().enumerate() {
        let reveal = segment.path.clone();
        trail = trail
            .child(
                div()
                    .id(("breadcrumb-segment", index))
                    .max_w(px(128.0))
                    .truncate()
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .hover(move |style| style.bg(theme.muted).text_color(theme.foreground))
                    .text_color(theme.muted_foreground.opacity(0.78))
                    .on_click(cx.listener(move |this, _, _, _| this.reveal_path(&reveal)))
                    .child(segment.name),
            )
            .child(icon(
                "icons/chevron-right.svg",
                theme.muted_foreground.opacity(0.38),
                10.0,
            ));
    }
    let reveal_current = tab.path().to_owned();
    let mut current = div()
        .id("breadcrumb-current")
        .flex()
        .items_center()
        .min_w_0()
        .px(px(2.0))
        .rounded(px(4.0))
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .hover(move |style| style.bg(theme.muted))
        .on_click(cx.listener(move |this, _, _, _| this.reveal_path(&reveal_current)))
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_size(px(11.0))
                .text_color(theme.foreground.opacity(0.85))
                .child(display.primary),
        );
    if let Some(filename) = display.secondary {
        current = current.child(
            div()
                .ml(px(4.0))
                .min_w_0()
                .truncate()
                .text_size(px(10.0))
                .font_weight(FontWeight::NORMAL)
                .text_color(theme.muted_foreground.opacity(0.60))
                .child(filename),
        );
    }
    trail = trail.child(current);

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
            cx.listener(|this, _, _, cx| this.click_toggle_wide_mode(cx)),
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

pub fn render_error_state(
    theme: Theme,
    error: &UserFacingError,
    drop_active: bool,
    cx: &Context<MdowApp>,
) -> AnyElement {
    error_state(theme, error, drop_active, cx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{document::parse_document, syntax::PreparedDocument, tabs::DocumentTab};
    use std::sync::Arc;

    fn document_tab(path: &str, source: &str) -> DocumentTab {
        let parsed = parse_document(PathBuf::from(path), source.to_owned());
        let last_source = Arc::from(parsed.source.clone());
        DocumentTab {
            document: Arc::new(PreparedDocument::plain(parsed)),
            last_source,
            reload_error: None,
        }
    }

    #[test]
    fn breadcrumb_uses_filename_until_frontmatter_supplies_a_title() {
        let plain = document_tab("/tmp/showcase.md", "# Heading\n");
        assert_eq!(
            breadcrumb_display(&plain),
            BreadcrumbDisplay {
                primary: "showcase.md".into(),
                secondary: None,
            }
        );

        let titled = document_tab(
            "/tmp/showcase.md",
            "---\ntitle: Reader title\n---\n# Heading\n",
        );
        assert_eq!(
            breadcrumb_display(&titled),
            BreadcrumbDisplay {
                primary: "Reader title".into(),
                secondary: Some("showcase.md".into()),
            }
        );
    }

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
