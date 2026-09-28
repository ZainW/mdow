use crate::{
    app::{MdowApp, UserFacingError},
    document::Heading,
    overlay::OverlayKind,
    prefs::SidebarMode,
    session::Recents,
    sparkle::UpdateUi,
    tabs::DocumentTab,
    theme::{Metrics, ShellLayout, Theme, TitlebarLeading},
    ui::{
        field::Field,
        primitives::{
            border_width, compact_icon_button, count_label, icon, icon_button, kbd, outline_button,
            pluralize, segment, segmented_track, text_button,
        },
    },
    workspace::{
        FilteredRow, MAX_WORKSPACE_FILES, WorkspaceEntryKind, WorkspaceRow, WorkspaceTree,
        normalize_filter_query,
    },
};
use gpui::{
    AnyElement, App, ClickEvent, Context, Div, Entity, FocusHandle, FontWeight, HighlightStyle,
    IntoElement, MouseButton, MouseDownEvent, ScrollHandle, Stateful, StatefulInteractiveElement,
    StyledText, Transformation, Window, div, percentage, prelude::*, px,
};
use std::{
    collections::HashSet,
    ops::Range,
    path::{Path, PathBuf},
};

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

/// The close control is always laid out (so widths never change) but only drawn on the
/// active tab, the hovered tab (handled at paint time), or while keyboard focus is on the tab.
pub fn tab_close_visible(active: bool, tab_focused: bool, close_focused: bool) -> bool {
    active || tab_focused || close_focused
}

/// File name and its parent folder name, for single-line recents rows.
pub fn recent_labels(path: &Path) -> (String, String) {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Untitled".into());
    let parent = path
        .parent()
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    (name, parent)
}

fn zoom_on_double_click(event: &ClickEvent, window: &mut Window, _: &mut App) {
    if event.click_count() == 2 {
        window.titlebar_double_click();
    }
}

fn sidebar_toggle(theme: Theme, cx: &Context<MdowApp>) -> impl IntoElement {
    icon_button(
        "toggle-sidebar",
        "icons/sidebar.svg",
        theme,
        cx.listener(|this, _, _, cx| {
            cx.stop_propagation();
            this.click_toggle_sidebar(cx);
        }),
    )
}

/// Traffic-light clearance plus the sidebar toggle, when the sidebar is hidden.
fn titlebar_leading(leading: TitlebarLeading, theme: Theme, cx: &Context<MdowApp>) -> Div {
    div()
        .flex()
        .flex_none()
        .items_center()
        .h_full()
        .child(div().w(px(leading.clearance)).h_full().flex_none())
        .when(leading.sidebar_toggle, |row| {
            row.child(
                div()
                    .debug_selector(|| "titlebar-sidebar-toggle".into())
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .size(px(Metrics::TITLEBAR_BUTTON))
                    .child(sidebar_toggle(theme, cx)),
            )
        })
}

fn titlebar_drag_area(id: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .flex_grow()
        .min_w(px(24.0))
        .h_full()
        .on_click(zoom_on_double_click)
}

fn section_header() -> Div {
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(6.0))
        .h(px(Metrics::SECTION_HEADER_HEIGHT))
        .pl(px(14.0))
        .pr(px(8.0))
        .font_family(Metrics::FONT_SANS)
        .text_size(px(11.0))
}

fn section_title(text: impl Into<gpui::SharedString>, theme: Theme, strong: bool) -> Div {
    div()
        .min_w_0()
        .flex_grow()
        .truncate()
        .font_weight(if strong {
            FontWeight::SEMIBOLD
        } else {
            FontWeight::NORMAL
        })
        .text_size(px(12.0))
        .text_color(if strong {
            theme.foreground
        } else {
            theme.muted_foreground
        })
        .child(text.into())
}

/// Sidebar row chrome shared by the tree, outline and recents: rounded, with a 2px accent
/// marker for the active entry and a focus ring that never changes the row's size.
fn sidebar_row(
    id: (&'static str, usize),
    active: bool,
    height: f32,
    theme: Theme,
) -> Stateful<Div> {
    border_width(
        div()
            .id(id)
            .debug_selector(move || format!("{}-{}", id.0, id.1))
            .tab_index(0)
            .focusable()
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .w_full()
            .min_w_0()
            .h(px(height))
            .rounded(px(5.0))
            .border_color(theme.primary.opacity(0.0))
            .bg(if active {
                theme.sidebar_accent
            } else {
                theme.sidebar_accent.opacity(0.0)
            })
            .font_family(Metrics::FONT_SANS)
            .text_size(px(12.0))
            .cursor_pointer()
            .when(!active, |row| {
                row.hover(move |style| style.bg(theme.sidebar_accent.opacity(0.7)))
            })
            .active(|style| style.opacity(0.82))
            .focus(move |style| style.border_color(theme.primary)),
        1.0,
    )
    .when(active, |row| {
        row.child(
            div()
                .absolute()
                .left(px(0.0))
                .top(px(4.0))
                .bottom(px(4.0))
                .w(px(2.0))
                .rounded(px(1.0))
                .bg(theme.accent),
        )
    })
}

pub struct SidebarProps<'a> {
    pub mode: SidebarMode,
    pub recents: &'a Recents,
    pub workspace: Option<&'a WorkspaceTree>,
    pub workspace_error: Option<&'a UserFacingError>,
    pub document_title: Option<&'a str>,
    pub headings: Option<&'a [Heading]>,
    pub active_heading: Option<usize>,
    pub active_path: Option<&'a Path>,
    pub filter: &'a Entity<Field>,
    pub filter_query: &'a str,
    pub filter_collapsed: &'a HashSet<PathBuf>,
    pub outline_scroll: &'a ScrollHandle,
    pub width: f32,
}

pub fn render_sidebar(theme: Theme, props: SidebarProps<'_>, cx: &Context<MdowApp>) -> AnyElement {
    let mode = props.mode;
    let header = div()
        .id("sidebar-header")
        .debug_selector(|| "sidebar-header".into())
        .flex()
        .flex_none()
        .items_center()
        .justify_end()
        .h(px(Metrics::SIDEBAR_HEADER_HEIGHT))
        .pr(px(Metrics::SIDEBAR_HEADER_END_INSET))
        .on_click(zoom_on_double_click)
        .child(sidebar_toggle(theme, cx));

    let modes = [
        (SidebarMode::Recents, "Recents", "icons/clock.svg"),
        (SidebarMode::Folder, "Folder", "icons/folder.svg"),
        (SidebarMode::Outline, "Outline", "icons/list.svg"),
    ];
    let mut track = segmented_track(theme).debug_selector(|| "sidebar-modes".into());
    for (segment_mode, label, icon_path) in modes {
        track = track.child(segment(
            label,
            label,
            icon_path,
            mode == segment_mode,
            theme,
            cx.listener(move |this, _, _, cx| this.set_sidebar_mode(segment_mode, cx)),
        ));
    }
    let segmented = div()
        .flex_none()
        .mx(px(Metrics::SEGMENTED_INSET))
        .mb(px(Metrics::SEGMENTED_GAP_BELOW))
        .child(track);

    let (section, body) = match mode {
        SidebarMode::Folder => folder_section(theme, &props, cx),
        SidebarMode::Outline => outline_section(theme, &props, cx),
        SidebarMode::Recents => recents_section(theme, &props, cx),
    };

    let footer = div()
        .flex()
        .flex_none()
        .border_t_1()
        .border_color(theme.border_subtle)
        .p(px(8.0))
        .child(border_width(
            div()
                .id("sidebar-settings")
                .debug_selector(|| "sidebar-settings".into())
                .tab_index(0)
                .focusable()
                .flex()
                .items_center()
                .gap(px(8.0))
                .h(px(28.0))
                .px(px(8.0))
                .w_full()
                .rounded(px(6.0))
                .border_color(theme.primary.opacity(0.0))
                .cursor_pointer()
                .hover(move |style| style.bg(theme.sidebar_accent))
                .focus(move |style| style.border_color(theme.primary))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.click_toggle_overlay(OverlayKind::Settings, window, cx);
                }))
                .child(icon("icons/settings.svg", theme.muted_foreground, 14.0))
                .child(
                    div()
                        .flex_grow()
                        .font_family(Metrics::FONT_SANS)
                        .text_size(px(12.0))
                        .text_color(theme.muted_foreground)
                        .child("Settings"),
                )
                .child(kbd("⌘,", theme)),
            1.0,
        ));

    div()
        .id("sidebar")
        .debug_selector(|| "sidebar".into())
        .flex()
        .flex_col()
        .w(px(props.width))
        .h_full()
        .flex_none()
        .border_r_1()
        .border_color(theme.border_subtle)
        .bg(theme.sidebar)
        .child(header)
        .child(segmented)
        .child(section.debug_selector(|| "sidebar-section-header".into()))
        .children(
            props
                .workspace_error
                .cloned()
                .map(|error| workspace_error_note(theme, error)),
        )
        .child(body)
        .child(footer)
        .into_any_element()
}

fn workspace_error_note(theme: Theme, error: UserFacingError) -> AnyElement {
    div()
        .flex()
        .items_start()
        .gap(px(7.0))
        .mx(px(8.0))
        .mb(px(6.0))
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
        )
        .into_any_element()
}

fn scroll_list(id: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .flex()
        .flex_col()
        .flex_grow()
        .min_h_0()
        .overflow_y_scroll()
        .scrollbar_width(px(4.0))
        .p(px(4.0))
}

fn sidebar_empty(
    theme: Theme,
    icon_path: &'static str,
    title: &'static str,
    hint: &'static str,
) -> Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        .pt(px(36.0))
        .px(px(20.0))
        .font_family(Metrics::FONT_SANS)
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
}

fn folder_section(
    theme: Theme,
    props: &SidebarProps<'_>,
    cx: &Context<MdowApp>,
) -> (Div, AnyElement) {
    let open_folder = compact_icon_button(
        "sidebar-open-folder",
        "icons/folder-open.svg",
        24.0,
        14.0,
        theme,
        cx.listener(|this, _, _, cx| this.open_folder_prompt(cx)),
    );
    let Some(workspace) = props.workspace else {
        let header = section_header()
            .child(section_title("No folder", theme, false))
            .child(open_folder);
        let body = scroll_list("workspace-scroll")
            .child(
                sidebar_empty(
                    theme,
                    "icons/folder.svg",
                    "No folder open",
                    "Open or drop a folder to browse its Markdown files.",
                )
                .child(div().mt(px(14.0)).child(outline_button(
                    "sidebar-empty-open-folder",
                    "Open Folder",
                    "icons/folder-open.svg",
                    theme,
                    cx.listener(|this, _, _, cx| this.open_folder_prompt(cx)),
                ))),
            )
            .into_any_element();
        return (header, body);
    };

    let file_count = workspace.file_count();
    let header = section_header()
        .child(section_title(workspace.root.name.clone(), theme, true))
        .child(count_label(pluralize(file_count, "file", "files"), theme))
        .child(open_folder);

    let filtering = normalize_filter_query(props.filter_query).is_some();
    let filtered =
        filtering.then(|| workspace.filtered_rows(props.filter_query, props.filter_collapsed));
    let filter_field = div()
        .id("folder-filter")
        .debug_selector(|| "folder-filter".into())
        .flex()
        .flex_none()
        .items_center()
        .gap(px(6.0))
        .h(px(28.0))
        .mx(px(10.0))
        .mb(px(4.0))
        .pl(px(8.0))
        .pr(px(8.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface_well)
        .overflow_hidden()
        .font_family(Metrics::FONT_SANS)
        .text_size(px(12.0))
        .text_color(theme.foreground)
        .child(
            div()
                .flex()
                .items_center()
                .min_w_0()
                .flex_grow()
                .child(props.filter.clone()),
        )
        .children(filtered.as_ref().map(|filtered| {
            count_label(pluralize(filtered.match_count, "match", "matches"), theme)
                .debug_selector(|| "folder-filter-count".into())
        }));

    let mut list = scroll_list("workspace-scroll");
    match filtered {
        Some(filtered) if filtered.rows.is_empty() => {
            list = list.child(
                div()
                    .debug_selector(|| "folder-filter-empty".into())
                    .pt(px(28.0))
                    .px(px(12.0))
                    .text_center()
                    .font_family(Metrics::FONT_SANS)
                    .text_size(px(12.0))
                    .text_color(theme.muted_foreground)
                    .child("No matching files"),
            );
        }
        Some(filtered) => {
            for (index, FilteredRow { row, name_match }) in filtered.rows.into_iter().enumerate() {
                list = list.child(workspace_row(
                    index, row, name_match, true, props, theme, cx,
                ));
            }
        }
        None => {
            let rows = workspace.visible_rows();
            if rows.is_empty() {
                list = list.child(
                    div()
                        .px(px(12.0))
                        .pt(px(36.0))
                        .text_center()
                        .font_family(Metrics::FONT_SANS)
                        .text_size(px(12.0))
                        .line_height(px(18.0))
                        .text_color(theme.muted_foreground)
                        .child("No Markdown files in this folder."),
                );
            }
            for (index, row) in rows.into_iter().enumerate() {
                list = list.child(workspace_row(index, row, None, false, props, theme, cx));
            }
        }
    }
    if workspace.truncated {
        list = list.child(
            div()
                .debug_selector(|| "workspace-truncated".into())
                .flex_none()
                .mt(px(6.0))
                .px(px(10.0))
                .py(px(6.0))
                .font_family(Metrics::FONT_SANS)
                .text_size(px(11.0))
                .text_color(theme.muted_foreground)
                .child(format!(
                    "Showing the first {} files",
                    crate::ui::primitives::format_count(MAX_WORKSPACE_FILES)
                )),
        );
    }
    let body = div()
        .flex()
        .flex_col()
        .flex_grow()
        .min_h_0()
        .child(filter_field)
        .child(list)
        .into_any_element();
    (header, body)
}

fn highlighted_name(name: String, name_match: Option<Range<usize>>, theme: Theme) -> StyledText {
    let text = StyledText::new(name);
    match name_match {
        Some(range) => text.with_highlights(vec![(
            range,
            HighlightStyle {
                color: Some(theme.foreground),
                font_weight: Some(FontWeight::SEMIBOLD),
                ..Default::default()
            },
        )]),
        None => text,
    }
}

fn workspace_row(
    index: usize,
    row: WorkspaceRow,
    name_match: Option<Range<usize>>,
    filtering: bool,
    props: &SidebarProps<'_>,
    theme: Theme,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let directory = row.kind == WorkspaceEntryKind::Directory;
    let is_active = !directory && props.active_path.is_some_and(|path| path == row.path);
    let icon_path = match row.kind {
        WorkspaceEntryKind::Directory if row.expanded => "icons/folder-open.svg",
        WorkspaceEntryKind::Directory => "icons/folder.svg",
        WorkspaceEntryKind::File => "icons/file.svg",
    };
    let toggle_path = row.path.clone();
    let click_path = row.path.clone();
    let menu_path = row.path.clone();

    let disclosure = div()
        .id(("workspace-disclosure", index))
        .debug_selector(move || format!("workspace-disclosure-{index}"))
        .flex()
        .items_center()
        .justify_center()
        .size(px(18.0))
        .flex_none()
        .rounded(px(4.0))
        .when(directory, |button| {
            border_width(
                button
                    .tab_index(0)
                    .focusable()
                    .cursor_pointer()
                    .border_color(theme.primary.opacity(0.0))
                    .hover(move |style| style.bg(theme.muted))
                    .active(|style| style.opacity(0.78))
                    .focus(move |style| style.border_color(theme.primary))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.toggle_tree_directory(&toggle_path, filtering, cx);
                    })),
                1.0,
            )
        })
        .child(
            icon(
                "icons/chevron-right.svg",
                theme.muted_foreground.opacity(0.7),
                10.0,
            )
            .when(row.expanded, |chevron| {
                chevron.with_transformation(Transformation::rotate(percentage(0.25)))
            }),
        )
        .when(!directory, |space| space.invisible());

    sidebar_row(
        ("workspace-row", index),
        is_active,
        Metrics::TREE_ROW_HEIGHT,
        theme,
    )
    .tab_group()
    .gap(px(4.0))
    .pl(px(8.0 + row.depth as f32 * 10.0))
    .pr(px(6.0))
    .text_color(if is_active {
        theme.foreground
    } else {
        theme.muted_foreground
    })
    .on_click(cx.listener(move |this, _, _, cx| {
        if directory {
            this.toggle_tree_directory(&click_path, filtering, cx);
        } else {
            this.open_path(&click_path, cx);
        }
    }))
    .on_mouse_down(
        MouseButton::Right,
        cx.listener(move |this, event: &MouseDownEvent, window, cx| {
            cx.stop_propagation();
            this.open_tree_context_menu(&menu_path, directory, event.position, window, cx);
        }),
    )
    .child(disclosure)
    .child(icon(icon_path, theme.muted_foreground, 14.0))
    .child(
        div()
            .min_w_0()
            .flex_grow()
            .truncate()
            .child(highlighted_name(row.name, name_match, theme)),
    )
    .into_any_element()
}

fn outline_section(
    theme: Theme,
    props: &SidebarProps<'_>,
    cx: &Context<MdowApp>,
) -> (Div, AnyElement) {
    let headings = props.headings.unwrap_or(&[]);
    let header = match props.document_title {
        Some(title) => section_header()
            .child(section_title(title.to_owned(), theme, true))
            .child(count_label(
                pluralize(headings.len(), "heading", "headings"),
                theme,
            )),
        None => section_header().child(section_title("Outline", theme, false)),
    };
    let mut list = scroll_list("outline-scroll").track_scroll(props.outline_scroll);
    if headings.is_empty() {
        list = list.child(if props.document_title.is_some() {
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
        let base_level = headings
            .iter()
            .map(|heading| heading.level)
            .min()
            .unwrap_or(1);
        for (index, heading) in headings.iter().enumerate() {
            let active = props.active_heading == Some(index);
            let depth = heading.level.saturating_sub(base_level) as usize;
            let color = if active || heading.level <= 2 {
                theme.foreground
            } else {
                theme.muted_foreground
            };
            let mut row = sidebar_row(
                ("outline-row", index),
                active,
                Metrics::OUTLINE_ROW_HEIGHT,
                theme,
            )
            .pl(px(8.0))
            .pr(px(6.0))
            .text_color(color)
            .on_click(cx.listener(move |this, _, _, cx| this.jump_to_heading(index, cx)));
            for _ in 0..depth {
                row = row.child(
                    div()
                        .debug_selector(move || format!("outline-guide-{index}"))
                        .w(px(1.0))
                        .h_full()
                        .flex_none()
                        .ml(px(6.0))
                        .mr(px(5.0))
                        .bg(theme.border_subtle),
                );
            }
            list = list.child(
                row.child(
                    div()
                        .flex()
                        .items_center()
                        .min_w_0()
                        .flex_grow()
                        .pl(px(4.0))
                        .child(div().min_w_0().truncate().child(heading.text.clone())),
                ),
            );
        }
    }
    (header, list.into_any_element())
}

fn recents_section(
    theme: Theme,
    props: &SidebarProps<'_>,
    cx: &Context<MdowApp>,
) -> (Div, AnyElement) {
    let has_recents = !props.recents.is_empty();
    let header = section_header()
        .child(section_title("Recent files", theme, true))
        .child(
            text_button(
                "recents-clear",
                "Clear",
                theme,
                cx.listener(|this, _, _, cx| this.clear_recents(cx)),
            )
            .when(!has_recents, |button| button.invisible()),
        );
    let mut list = scroll_list("recents-scroll");
    if !has_recents {
        list = list.child(sidebar_empty(
            theme,
            "icons/clock.svg",
            "No recents yet",
            "Files you open will appear here.",
        ));
    }
    for (index, path) in props.recents.iter().enumerate() {
        let active = props.active_path.is_some_and(|active| active == path);
        list = list.child(recent_row(
            ("recent-row", index),
            path,
            active,
            RecentRowStyle::Sidebar,
            theme,
            cx,
        ));
    }
    (header, list.into_any_element())
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RecentRowStyle {
    Sidebar,
    Welcome,
}

/// One-line recent file: name on the left, muted parent folder on the right.
pub fn recent_row(
    id: (&'static str, usize),
    path: &Path,
    active: bool,
    style: RecentRowStyle,
    theme: Theme,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let (name, parent) = recent_labels(path);
    let open_path = path.to_owned();
    let menu_path = path.to_owned();
    let welcome = style == RecentRowStyle::Welcome;
    sidebar_row(id, active, Metrics::RECENT_ROW_HEIGHT, theme)
        .gap(px(if welcome { 9.0 } else { 8.0 }))
        .pl(px(if welcome { 10.0 } else { 8.0 }))
        .pr(px(if welcome { 10.0 } else { 6.0 }))
        .when(welcome, |row| row.text_size(px(13.0)))
        .on_click(cx.listener(move |this, _, _, cx| this.open_path(&open_path, cx)))
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                this.open_recent_context_menu(&menu_path, event.position, window, cx);
            }),
        )
        .child(icon(
            "icons/file.svg",
            theme.muted_foreground,
            if welcome { 14.0 } else { 13.0 },
        ))
        .child(
            div()
                .min_w_0()
                .flex_shrink()
                .truncate()
                .text_color(theme.foreground)
                .child(name),
        )
        .child(div().flex_grow().min_w(px(8.0)))
        .child(
            div()
                .flex_none()
                .max_w(px(if welcome { 140.0 } else { 80.0 }))
                .truncate()
                .text_size(px(if welcome { 11.5 } else { 11.0 }))
                .text_color(theme.muted_foreground)
                .child(parent),
        )
        .into_any_element()
}

/// Per-tab keyboard focus, supplied by the app so the close control can show while focused.
pub struct TabFocus {
    pub tab: FocusHandle,
    pub close: FocusHandle,
}

/// The tab row is also the titlebar row: empty space drags/zooms the window.
pub fn render_tab_bar(
    theme: Theme,
    app: &MdowApp,
    layout: &ShellLayout,
    focus: &[TabFocus],
    window: &Window,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let active_path = app.model.tabs.active().map(|tab| tab.path().to_owned());
    let mut tabs = div()
        .id("tabs-scroll")
        .debug_selector(|| "tabs-scroll".into())
        .flex()
        // Pinned from the top: the horizontal scrollbar gutter must not re-centre the tabs.
        .items_start()
        .pt(px((Metrics::TAB_BAR_HEIGHT - Metrics::TAB_HEIGHT) / 2.0))
        .min_w_0()
        .flex_grow()
        .h_full()
        .gap(px(Metrics::TAB_GAP))
        .px(px(Metrics::TAB_LIST_INSET))
        .overflow_x_scroll()
        .scrollbar_width(px(6.0));

    for (index, path) in app.model.tabs.paths().enumerate() {
        let is_active = active_path.as_deref() == Some(path);
        let deleted = app.model.deleted.contains(path);
        let filename = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".into());
        let activate_path = path.to_owned();
        let close_path = path.to_owned();
        let middle_close_path = path.to_owned();
        let menu_path = path.to_owned();
        let group: gpui::SharedString = format!("document-tab-{index}").into();
        let (tab_focused, close_focused) = focus
            .get(index)
            .map(|focus| (focus.tab.is_focused(window), focus.close.is_focused(window)))
            .unwrap_or_default();
        let close_visible = tab_close_visible(is_active, tab_focused, close_focused);
        let close_icon_color = if close_visible {
            theme.muted_foreground
        } else {
            theme.muted_foreground.opacity(0.0)
        };
        let mut tab = div()
            .id(("document-tab", index))
            .debug_selector(move || format!("document-tab-{index}"))
            .group(group.clone())
            .tab_group()
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
                match theme.color_scheme {
                    crate::theme::ColorScheme::Dark => theme.muted,
                    crate::theme::ColorScheme::Light => theme.card,
                }
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
            .when(!is_active, |tab| {
                tab.hover(move |style| style.bg(theme.muted))
            })
            .active(|style| style.opacity(0.82))
            .focus(move |style| style.border_color(theme.primary))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.activate_tab(&activate_path, cx);
            }))
            .on_mouse_up(
                MouseButton::Middle,
                cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.close_tab(&middle_close_path, cx);
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.open_tab_context_menu(&menu_path, event.position, window, cx);
                }),
            );
        tab = match focus.get(index) {
            Some(focus) => tab.track_focus(&focus.tab),
            None => tab.tab_index(0).focusable(),
        };
        let mut close = div()
            .id(("close-document-tab", index))
            .debug_selector(move || format!("close-document-tab-{index}"))
            .flex()
            .items_center()
            .justify_center()
            .size(px(Metrics::TAB_CLOSE_SIZE))
            .mr(px(Metrics::TAB_CLOSE_END_MARGIN))
            .flex_none()
            .rounded(px(4.0))
            .border_1()
            .border_color(theme.primary.opacity(0.0))
            .cursor_pointer()
            .hover(move |style| style.bg(theme.muted))
            .active(|style| style.opacity(0.76))
            .focus(move |style| style.border_color(theme.primary))
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                this.close_tab(&close_path, cx);
            }))
            .child(
                icon("icons/x.svg", close_icon_color, 12.0)
                    .group_hover(group, move |style| style.text_color(theme.muted_foreground)),
            );
        close = match focus.get(index) {
            Some(focus) => close.track_focus(&focus.close),
            None => close.tab_index(0).focusable(),
        };
        tab = tab
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
                    .child(
                        div()
                            .min_w_0()
                            .flex_grow()
                            .truncate()
                            .when(deleted, |label| label.line_through())
                            .child(filename),
                    ),
            )
            .child(close);
        tabs = tabs.child(tab);
    }
    tabs = tabs.child(titlebar_drag_area("titlebar-drag"));

    div()
        .id("tab-bar")
        .debug_selector(|| "tab-bar".into())
        .flex()
        .items_center()
        .h(px(layout.tab_bar_height))
        .flex_none()
        .border_b_1()
        .border_color(theme.border_subtle)
        .bg(theme.background)
        .child(titlebar_leading(layout.titlebar_leading, theme, cx))
        .child(tabs)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(2.0))
                .h_full()
                .px(px(6.0))
                .flex_none()
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
                .children(app.companion.toggle_button(theme, cx)),
        )
        .into_any_element()
}

/// The titlebar row of the empty window: no tabs or breadcrumb, only the sidebar toggle
/// (when the sidebar is hidden) and the command palette.
pub fn render_empty_toolbar(
    theme: Theme,
    layout: &ShellLayout,
    companion_button: Option<AnyElement>,
    cx: &Context<MdowApp>,
) -> AnyElement {
    div()
        .id("empty-toolbar")
        .debug_selector(|| "empty-toolbar".into())
        .flex()
        .items_center()
        .h(px(layout.tab_bar_height))
        .flex_none()
        .child(titlebar_leading(layout.titlebar_leading, theme, cx))
        .child(titlebar_drag_area("titlebar-drag"))
        .child(
            div()
                .flex()
                .items_center()
                .h_full()
                .gap(px(2.0))
                .px(px(6.0))
                .flex_none()
                .child(icon_button(
                    "toggle-palette",
                    "icons/command.svg",
                    theme,
                    cx.listener(|this, _, window, cx| {
                        this.click_toggle_overlay(OverlayKind::Palette, window, cx);
                    }),
                ))
                .children(companion_button),
        )
        .into_any_element()
}

pub fn render_breadcrumb(theme: Theme, tab: &DocumentTab, cx: &Context<MdowApp>) -> AnyElement {
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
        .debug_selector(|| "breadcrumb".into())
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
            22.0,
            12.0,
            theme,
            cx.listener(|this, _, _, cx| this.click_toggle_wide_mode(cx)),
        ))
        .into_any_element()
}

fn banner(theme: Theme, tint: gpui::Hsla) -> Div {
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
        .border_color(tint.opacity(0.35))
        .bg(tint.opacity(0.08))
        .font_family(Metrics::FONT_SANS)
        .text_size(px(11.0))
        .text_color(theme.foreground)
}

pub fn render_error_banner(theme: Theme, error: &UserFacingError) -> AnyElement {
    banner(theme, theme.destructive)
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
            banner(theme, theme.destructive)
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

fn banner_button(
    id: &'static str,
    label: &'static str,
    theme: Theme,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    text_button(id, label, theme, on_click)
        .border_color(theme.border)
        .bg(theme.card)
        .text_size(px(11.0))
        .text_color(theme.foreground)
}

/// The open file vanished from disk. The last good content stays readable underneath.
pub fn render_deleted_banner(theme: Theme, path: &Path, cx: &Context<MdowApp>) -> AnyElement {
    let close_path = path.to_owned();
    let folder = path.parent().map(Path::to_owned);
    div()
        .id("deleted-banner")
        .debug_selector(|| "deleted-banner".into())
        .child(
            banner(theme, theme.accent)
                .child(icon("icons/alert-circle.svg", theme.accent, 14.0))
                .child(
                    div()
                        .flex_none()
                        .font_weight(FontWeight::MEDIUM)
                        .child("This file was deleted"),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_grow()
                        .truncate()
                        .font_family(Metrics::FONT_MONO)
                        .text_size(px(10.5))
                        .text_color(theme.muted_foreground)
                        .child(path.to_string_lossy().into_owned()),
                )
                .child(banner_button(
                    "deleted-show-in-finder",
                    "Show in Finder",
                    theme,
                    move |_, _, _| {
                        if let Some(folder) = folder.as_ref() {
                            let _ = open::that(folder);
                        }
                    },
                ))
                .child(banner_button(
                    "deleted-close-tab",
                    "Close Tab",
                    theme,
                    cx.listener(move |this, _, _, cx| this.close_tab(&close_path, cx)),
                )),
        )
        .into_any_element()
}

pub fn render_update_banner(
    theme: Theme,
    update: &UpdateUi,
    cx: &Context<MdowApp>,
) -> Option<AnyElement> {
    let copy = update.banner_copy()?;
    let action = update.action_label();
    let mut row = div()
        .id("update-banner")
        .debug_selector(|| "update-banner".into())
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(12.0))
        .py(px(6.0))
        .flex_none()
        .border_t_1()
        .border_color(theme.border_subtle)
        .bg(theme.muted.opacity(0.45))
        .font_family(Metrics::FONT_SANS)
        .text_size(px(11.0))
        .text_color(theme.muted_foreground)
        .child(div().min_w_0().flex_grow().child(copy));
    if let Some(label) = action {
        let event_download = update.can_download();
        let manual_download = update.needs_manual_download();
        row = row.child(
            div()
                .id("update-banner-action")
                .tab_index(0)
                .focusable()
                .flex_none()
                .debug_selector(|| "update-banner-action".into())
                .px(px(8.0))
                .h(px(22.0))
                .flex()
                .items_center()
                .rounded(px(5.0))
                .border_1()
                .border_color(theme.border)
                .bg(theme.card)
                .text_color(theme.foreground)
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    if manual_download {
                        let _ = open::that(crate::sparkle::RELEASES_URL);
                    } else if event_download {
                        this.download_update(cx);
                    } else {
                        this.install_update(cx);
                    }
                }))
                .child(label),
        );
    }
    Some(
        row.child(compact_icon_button(
            "update-banner-dismiss",
            "icons/x.svg",
            22.0,
            12.0,
            theme,
            cx.listener(|this, _, _, cx| this.dismiss_update_banner(cx)),
        ))
        .into_any_element(),
    )
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

    #[test]
    fn tab_close_shows_for_active_or_keyboard_focused_tabs_only() {
        assert!(tab_close_visible(true, false, false));
        assert!(tab_close_visible(false, true, false));
        assert!(tab_close_visible(false, false, true));
        assert!(!tab_close_visible(false, false, false));
    }

    #[test]
    fn recent_labels_split_name_and_parent_folder() {
        assert_eq!(
            recent_labels(Path::new("/Users/zain/notes/guides/reading.md")),
            ("reading.md".into(), "guides".into())
        );
        assert_eq!(
            recent_labels(Path::new("README.md")),
            ("README.md".into(), String::new())
        );
    }
}
