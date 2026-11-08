//! The sidebar: the Outline (it follows the reading position), the Files of the tab's folder
//! with a filter, or Recents, with Settings and a light/dark switch at the bottom.

use crate::app::{App, Cmd, OpenIn};
use crate::files::{self, TreeNode};
use crate::icons;
use crate::prefs::{self, SidebarMode};
use crate::theme::{self, Palette};
use crate::ui::chrome_button;
use rune::{El, GrowthSpec, ScrollSpec, Tx, col, el, icon, row, text};
use std::path::{Path, PathBuf};

pub fn view(s: &App, p: &Palette, k: &rune_kit::Kit) -> El {
    let mode = s.prefs.sidebar_mode;
    let mut tabs = row().gap(2.0).px(10.0).pb(6.0);
    for (m, glyph) in [
        (SidebarMode::Outline, &icons::OUTLINE),
        (SidebarMode::Files, &icons::FOLDER),
        (SidebarMode::Recents, &icons::CLOCK),
    ] {
        let on = m == mode;
        tabs = tabs.child(
            row()
                .key(format!("sidebar-{}", m.name().to_lowercase()))
                .h(26.0)
                .grow(1.0)
                .center()
                .gap(5.0)
                .radius(6.0)
                .role("tab")
                .label(m.name())
                .checked(on)
                .fill(if on { p.selected } else { rune::Color::TRANSPARENT })
                .hover(p.hover)
                .child(icon(glyph, 14.0).color(if on { p.text } else { p.secondary }))
                .child(
                    text(m.name())
                        .font(theme::ui(11.5, if on { 600.0 } else { 500.0 }))
                        .color(if on { p.text } else { p.secondary }),
                )
                .on_press(move |s: &mut App| {
                    s.prefs = prefs::update(|pr| pr.sidebar_mode = m);
                }),
        );
    }
    let content = match mode {
        SidebarMode::Outline => outline(s, p),
        SidebarMode::Files => files_view(s, p, k),
        SidebarMode::Recents => recents(s, p),
    };
    let dark = p.dark;
    let footer = row()
        .w_full()
        .px(10.0)
        .py(8.0)
        .gap(4.0)
        .center_y()
        .border_sides([1.0, 0.0, 0.0, 0.0], p.line)
        .child(
            chrome_button("settings", &icons::GEAR, "Settings (⌘,)", p, true, None)
                .on_press_tx(|s: &mut App, tx: &mut Tx| s.run(Cmd::Settings, tx)),
        )
        .child(el().grow(1.0))
        .child(
            chrome_button(
                "appearance",
                if dark { &icons::SUN } else { &icons::MOON },
                if dark { "Light Mode" } else { "Dark Mode" },
                p,
                true,
                None,
            )
            .on_press_tx(|s: &mut App, tx: &mut Tx| s.run(Cmd::ToggleAppearance, tx)),
        );
    col()
        .key("sidebar")
        .w(s.prefs.sidebar_width)
        .h_full()
        .min_h(0.0)
        .pt(2.0)
        .child(tabs)
        .child(content)
        .child(footer)
}

fn section_title(label: &str, p: &Palette) -> El {
    text(label.to_uppercase())
        .font(rune::TextStyle { tracking: 0.5, ..theme::ui(10.5, 600.0) })
        .color(p.tertiary)
        .px(18.0)
        .pt(8.0)
        .pb(4.0)
}

fn empty(label: &str, p: &Palette) -> El {
    col()
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .px(18.0)
        .pt(14.0)
        .child(text(label.to_string()).font(theme::ui(12.0, 400.0)).color(p.tertiary).wrap())
}

fn sidebar_row(
    key: String,
    label: String,
    depth: usize,
    selected: bool,
    glyph: Option<&'static rune::Icon>,
    p: &Palette,
) -> El {
    let mut r = row()
        .key(key)
        .w_full()
        .h(26.0)
        .center_y()
        .gap(6.0)
        .pl(10.0 + depth as f32 * 12.0)
        .pr(8.0)
        .radius(6.0)
        .role("treeitem")
        .label(label.clone())
        .fill(if selected { p.selected } else { rune::Color::TRANSPARENT })
        .hover(p.hover);
    if let Some(g) = glyph {
        r = r.child(icon(g, 14.0).color(p.tertiary));
    }
    r.child(
        text(label)
            .font(theme::ui(12.5, if selected { 600.0 } else { 400.0 }))
            .color(if selected { p.text } else { p.secondary })
            .truncate()
            .grow(1.0)
            .min_w(0.0),
    )
}

fn outline(s: &App, p: &Palette) -> El {
    let pane = s.tab().pane();
    let Some(doc) = pane.doc.as_ref() else {
        return empty("Open a document to see its outline.", p);
    };
    if doc.html {
        return empty("HTML pages have no outline.", p);
    }
    let heads = &doc.doc.headings;
    if heads.is_empty() {
        return empty("This document has no headings.", p);
    }
    let current = pane.current_heading();
    let min_level = heads.iter().map(|h| h.level).min().unwrap_or(1);
    let pane_id = pane.id;
    let rows = col()
        .key("outline-rows")
        .w_full()
        .px(8.0)
        .growth(GrowthSpec::default())
        .virtual_rows(
            heads.len(),
            26.0,
            move |_s: &App, i| format!("outline-{i}").into(),
            move |s: &App, i| {
                let Some(doc) = s.pane(pane_id).and_then(|p| p.doc.as_ref()) else { return col() };
                let Some(h) = doc.doc.headings.get(i) else { return col() };
                let p = Palette::get(s.dark());
                let depth = (h.level - min_level) as usize;
                let row_index = h.row;
                let link = format!(
                    "{}#{}",
                    doc.path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    h.slug
                );
                sidebar_row(
                    format!("outline-{i}"),
                    h.text.clone(),
                    depth,
                    current == Some(i),
                    None,
                    &p,
                )
                .on_press_tx(move |s: &mut App, tx: &mut Tx| {
                    s.scroll_to_row(pane_id, row_index, tx)
                })
                .on_secondary_press(move |_s: &mut App, press, tx: &mut Tx| {
                    let link = link.clone();
                    let menu = rune::PopupMenu::new()
                        .item(rune::PopupItem::new("link", "Copy Link to Heading"));
                    tx.popup_menu(
                        menu,
                        press,
                        move |_s: &mut App, picked: Option<&str>, tx: &mut Tx| {
                            if picked == Some("link") {
                                tx.copy(link.clone());
                            }
                        },
                    );
                })
            },
        )
        .virtual_rows_version((doc.version, current, p.dark));
    col()
        .key("outline")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .scroll(ScrollSpec { stick_bottom: false })
        .child(rows)
}

/// The tree's rows as drawn: (node, depth), with closed folders' children left out.
fn flatten<'a>(
    nodes: &'a [TreeNode],
    expanded: &std::collections::HashSet<PathBuf>,
    filtering: bool,
    depth: usize,
    out: &mut Vec<(&'a TreeNode, usize)>,
) {
    for n in nodes {
        out.push((n, depth));
        if let Some(children) = &n.children
            && (filtering || expanded.contains(&n.path))
        {
            flatten(children, expanded, filtering, depth + 1, out);
        }
    }
}

fn files_view(s: &App, p: &Palette, k: &rune_kit::Kit) -> El {
    let tab = s.tab();
    let Some(folder) = tab.folder.as_ref() else {
        return col()
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .gap(10.0)
            .child(
                text("Open a folder to browse its documents.")
                    .font(theme::ui(12.0, 400.0))
                    .color(p.tertiary)
                    .wrap()
                    .px(18.0)
                    .pt(14.0),
            )
            .child(
                rune_kit::Button::new("Open Folder…")
                    .on_press(|s: &mut App| crate::windows::send(&s.key, Cmd::OpenFolderDialog))
                    .build(k)
                    .key("files-open-folder")
                    .mx(18.0),
            );
    };
    let Some(scan) = tab.scan.as_ref() else { return empty("Reading the folder…", p) };
    let query = s.files_filter.text.trim().to_string();
    let filtered;
    let nodes: &[TreeNode] = if query.is_empty() {
        &scan.tree
    } else {
        filtered = files::filter_tree(&scan.tree, folder, &query);
        &filtered
    };
    let filter = rune_kit::TextInput::new(&s.files_filter)
        .placeholder("Filter")
        .label("Filter files")
        .leading_icon(&icons::SEARCH)
        .on_edit(|s: &mut App| &mut s.files_filter)
        .build(k)
        .key("files-filter");
    let mut flat = Vec::new();
    flatten(nodes, &tab.expanded, !query.is_empty(), 0, &mut flat);
    let current = tab.pane().path().map(Path::to_path_buf);
    let rows: Vec<(PathBuf, String, usize, bool)> =
        flat.iter().map(|(n, d)| (n.path.clone(), n.name.clone(), *d, n.is_dir())).collect();
    let count = rows.len();
    let version =
        (rows.len(), query.clone(), tab.expanded.len(), current.clone(), p.dark, scan.tree.len());
    let list = col()
        .key("files-rows")
        .w_full()
        .px(8.0)
        .growth(GrowthSpec::default())
        .virtual_rows(
            count,
            26.0,
            {
                let rows = rows.clone();
                move |_s: &App, i| format!("file-{}", rows[i].0.display()).into()
            },
            move |s: &App, i| {
                let (path, name, depth, dir) = rows[i].clone();
                let menu_path = path.clone();
                let p = Palette::get(s.dark());
                let open = dir && s.tab().expanded.contains(&path);
                let selected = current.as_deref() == Some(path.as_path());
                let glyph = if dir {
                    if open { &icons::CHEVRON_DOWN } else { &icons::CHEVRON_RIGHT }
                } else {
                    &icons::DOC
                };
                sidebar_row(
                    format!("file-{}", path.display()),
                    name,
                    depth,
                    selected,
                    Some(glyph),
                    &p,
                )
                .on_secondary_press(move |s: &mut App, press, tx: &mut Tx| {
                    s.file_menu(menu_path.clone(), false, press, tx)
                })
                .on_press_tx(move |s: &mut App, tx: &mut Tx| {
                    if dir {
                        let tab = s.tab_mut();
                        if !tab.expanded.remove(&path) {
                            tab.expanded.insert(path.clone());
                        }
                    } else {
                        let id = s.tab().pane().id;
                        if tx.mods().cmd {
                            s.open_path(&path, OpenIn::NewTab);
                        } else {
                            s.navigate(id, &path, None, true);
                        }
                        tx.set_title(s.window_title());
                    }
                })
            },
        )
        .virtual_rows_version(version);
    let name = files::display_name(folder);
    let mut c = col()
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .child(row().px(10.0).pb(6.0).child(filter.grow(1.0)))
        .child(section_title(&name, p));
    if count == 0 {
        c = c.child(empty(
            if query.is_empty() { "No documents in this folder." } else { "No matching files." },
            p,
        ));
    } else {
        c = c.child(
            col()
                .key("files")
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .scroll(ScrollSpec { stick_bottom: false })
                .child(list),
        );
    }
    if scan.truncated {
        c = c.child(
            text(format!("Showing the first {} files.", files::MAX_FILES))
                .font(theme::ui(11.0, 400.0))
                .color(p.tertiary)
                .px(18.0)
                .py(4.0),
        );
    }
    c
}

fn recents(s: &App, p: &Palette) -> El {
    let recents = &s.prefs.recents;
    if recents.is_empty() {
        return empty("Documents and folders you open show up here.", p);
    }
    let mut c = col().w_full().px(8.0);
    for (i, path) in recents.iter().enumerate() {
        let dir = path.is_dir();
        let label = files::display_name(path);
        let detail = files::display_dir(path);
        let path = path.clone();
        c = c.child(
            col()
                .key(format!("recent-{i}"))
                .w_full()
                .px(10.0)
                .py(5.0)
                .radius(6.0)
                .hover(p.hover)
                .role("button")
                .label(label.clone())
                .help(path.display().to_string())
                .child(
                    row()
                        .gap(6.0)
                        .center_y()
                        .child(
                            icon(if dir { &icons::FOLDER } else { &icons::DOC }, 13.0)
                                .color(p.tertiary),
                        )
                        .child(
                            text(label)
                                .font(theme::ui(12.5, 500.0))
                                .color(p.text)
                                .truncate()
                                .grow(1.0)
                                .min_w(0.0),
                        ),
                )
                .child(
                    text(detail).font(theme::ui(11.0, 400.0)).color(p.tertiary).truncate().pl(19.0),
                )
                .on_secondary_press({
                    let path = path.clone();
                    move |s: &mut App, press, tx: &mut Tx| {
                        s.file_menu(path.clone(), true, press, tx)
                    }
                })
                .on_press_tx(move |s: &mut App, tx: &mut Tx| {
                    s.open_paths(std::slice::from_ref(&path), tx)
                }),
        );
    }
    col()
        .key("recents")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .scroll(ScrollSpec { stick_bottom: false })
        .child(c)
}
