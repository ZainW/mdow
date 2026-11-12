//! Quick Open (⌘K): headings of the open document, files in the folder and recents, and
//! commands, ranked by fuzzy match. Up and Down move, Return opens, Escape closes.

use crate::app::{App, Cmd, OpenIn, QuickOpen};
use crate::files;
use crate::icons;
use crate::theme::{self, Palette};
use rune::{El, Icon, Key, KeyEvent, Tx, col, icon, row, text};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum Action {
    Heading(u64, usize),
    File(PathBuf),
    Command(Cmd),
}

#[derive(Clone, Debug)]
pub struct Item {
    pub title: String,
    pub detail: String,
    pub glyph: &'static Icon,
    pub hint: &'static str,
    pub action: Action,
}

/// A command's title, shortcut and what it runs.
type Command = (&'static str, &'static str, fn() -> Cmd);

const COMMANDS: &[Command] = &[
    ("Open…", "⌘O", || Cmd::OpenDialog),
    ("Open Folder…", "⇧⌘O", || Cmd::OpenFolderDialog),
    ("New Tab", "⌘T", || Cmd::NewTab),
    ("New Window", "⌘N", || Cmd::NewWindow),
    ("Find in Document", "⌘F", || Cmd::Find),
    ("Toggle Sidebar", "⌃⌘S", || Cmd::ToggleSidebar),
    ("Show Outline", "⌃⌘1", || Cmd::Sidebar(crate::prefs::SidebarMode::Outline)),
    ("Show Files", "⌃⌘2", || Cmd::Sidebar(crate::prefs::SidebarMode::Files)),
    ("Show Recents", "⌃⌘3", || Cmd::Sidebar(crate::prefs::SidebarMode::Recents)),
    ("Toggle Companion", "⌥⌘0", || Cmd::ToggleCompanion),
    ("Split View", "⌘\\", || Cmd::ToggleSplit),
    ("Zoom In", "⌘+", || Cmd::ZoomIn),
    ("Zoom Out", "⌘-", || Cmd::ZoomOut),
    ("Actual Size", "⌘0", || Cmd::ActualSize),
    ("Toggle Light/Dark", "", || Cmd::ToggleAppearance),
    ("Reload", "⌘R", || Cmd::Reload),
    ("Show in Finder", "⌥⌘R", || Cmd::ShowInFinder),
    ("Open in Editor", "⇧⌘E", || Cmd::OpenInEditor),
    ("Copy Path", "⌥⌘C", || Cmd::CopyPath),
    ("Clear Recents", "", || Cmd::ClearRecents),
    ("Settings…", "⌘,", || Cmd::Settings),
    ("Keyboard Shortcuts", "⌘/", || Cmd::Shortcuts),
];

/// The ranked results for the current query.
pub fn items(s: &App, query: &str) -> Vec<Item> {
    let q = query.trim();
    let mut scored: Vec<(f64, usize, Item)> = Vec::new();
    let mut push = |score: f64, item: Item, order: usize| {
        if q.is_empty() || score > 0.0 {
            scored.push((score, order, item));
        }
    };
    let pane = s.tab().pane();
    if let Some(doc) = pane.doc.as_ref() {
        for (i, h) in doc.doc.headings.iter().enumerate() {
            let score = files::fuzzy_score(q, &h.text) * 1.2;
            push(
                score,
                Item {
                    title: h.text.clone(),
                    detail: format!("H{}", h.level),
                    glyph: &icons::HASH,
                    hint: "",
                    action: Action::Heading(pane.id, h.row),
                },
                i,
            );
        }
    }
    let mut seen = std::collections::HashSet::new();
    let mut paths: Vec<PathBuf> = Vec::new();
    if let Some(scan) = s.tab().scan.as_ref() {
        files::flatten_files(&scan.tree, &mut paths);
    }
    paths.extend(s.prefs.recents.iter().filter(|p| p.is_file()).cloned());
    for (i, path) in paths.into_iter().enumerate() {
        if !seen.insert(path.clone()) {
            continue;
        }
        let name = files::display_name(&path);
        let full = path.display().to_string();
        let score = (files::fuzzy_score(q, &name) * 1.5).max(files::fuzzy_score(q, &full));
        push(
            score,
            Item {
                title: name,
                detail: files::display_dir(&path),
                glyph: &icons::DOC,
                hint: "",
                action: Action::File(path),
            },
            1000 + i,
        );
    }
    for (i, (title, hint, cmd)) in COMMANDS.iter().enumerate() {
        let title = crate::platform::label(title);
        let score = files::fuzzy_score(q, title);
        push(
            score,
            Item {
                title: title.to_string(),
                detail: String::new(),
                glyph: &icons::COMMAND,
                hint: crate::platform::label(hint),
                action: Action::Command(cmd()),
            },
            100_000 + i,
        );
    }
    if q.is_empty() {
        // With nothing typed: the outline first, then files, then commands, as listed.
        scored.sort_by_key(|(_, order, _)| *order);
    } else {
        scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal).then(a.1.cmp(&b.1))
        });
    }
    scored.into_iter().take(60).map(|(_, _, item)| item).collect()
}

fn choose(s: &mut App, tx: &mut Tx) {
    let query = s.quick_open.as_ref().map(|q| q.query.text.clone()).unwrap_or_default();
    let selected = s.quick_open.as_ref().map_or(0, |q| q.selected);
    let list = items(s, &query);
    let Some(item) = list.get(selected).cloned() else { return };
    s.quick_open = None;
    match item.action {
        Action::Heading(pane, row) => s.scroll_to_row(pane, row, tx),
        Action::File(path) => {
            s.open_path(&path, if tx.mods().cmd { OpenIn::NewTab } else { OpenIn::Here });
            tx.set_title(s.window_title());
        }
        Action::Command(cmd) => s.run(cmd, tx),
    }
}

pub fn view(s: &App, q: &QuickOpen, p: &Palette, k: &rune_kit::Kit) -> El {
    let list = items(s, &q.query.text);
    let selected = q.selected.min(list.len().saturating_sub(1));
    let field = rune_kit::TextInput::new(&q.query)
        .placeholder("Search headings, files and commands")
        .label("Quick Open")
        .leading_icon(&icons::SEARCH)
        .plain()
        .text_style(theme::ui(15.0, 400.0))
        .on_edit(|s: &mut App| &mut s.quick_open.as_mut().expect("quick open is open").query)
        .on_change(|s: &mut App, _tx: &mut Tx| {
            if let Some(q) = s.quick_open.as_mut() {
                q.selected = 0;
            }
        })
        .on_key(|s: &mut App, ev: &KeyEvent, tx: &mut Tx| {
            let n = {
                let query = s.quick_open.as_ref().map(|q| q.query.text.clone()).unwrap_or_default();
                items(s, &query).len()
            };
            let Some(q) = s.quick_open.as_mut() else { return false };
            match &ev.key {
                Key::Down => {
                    q.selected = if n == 0 { 0 } else { (q.selected + 1) % n };
                    tx.scroll_into_view(format!("qo-{}", q.selected));
                    true
                }
                Key::Up => {
                    q.selected = if n == 0 { 0 } else { (q.selected + n - 1) % n };
                    tx.scroll_into_view(format!("qo-{}", q.selected));
                    true
                }
                Key::Enter => {
                    choose(s, tx);
                    true
                }
                Key::Escape => {
                    s.quick_open = None;
                    true
                }
                _ => false,
            }
        })
        .build(k)
        .key("quick-open-field");

    let mut rows = col().w_full().px(6.0).pb(6.0);
    if list.is_empty() {
        rows = rows.child(
            text("No results").font(theme::ui(13.0, 400.0)).color(p.tertiary).px(12.0).py(10.0),
        );
    }
    for (i, item) in list.iter().enumerate() {
        let on = i == selected;
        let mut r = row()
            .key(format!("qo-{i}"))
            .w_full()
            .h(36.0)
            .gap(10.0)
            .center_y()
            .px(10.0)
            .radius(7.0)
            .role("option")
            .label(item.title.clone())
            .checked(on)
            .fill(if on { p.accent_soft } else { rune::Color::TRANSPARENT })
            .hover(p.hover)
            .child(icon(item.glyph, 15.0).color(if on { p.accent } else { p.tertiary }))
            .child(
                text(item.title.clone())
                    .font(theme::ui(13.5, 500.0))
                    .color(p.text)
                    .truncate()
                    .min_w(0.0)
                    .shrink(),
            )
            .child(
                text(item.detail.clone())
                    .font(theme::ui(11.5, 400.0))
                    .color(p.tertiary)
                    .truncate()
                    .min_w(0.0)
                    .grow(1.0),
            );
        if !item.hint.is_empty() {
            r = r.child(text(item.hint).font(theme::ui(11.5, 500.0)).color(p.tertiary));
        }
        rows = rows.child(r.on_press_tx(move |s: &mut App, tx: &mut Tx| {
            if let Some(q) = s.quick_open.as_mut() {
                q.selected = i;
            }
            choose(s, tx);
        }));
    }
    let panel = col()
        .key("quick-open")
        .w(600.0)
        .radius(12.0)
        .fill(p.elevated)
        .border(1.0, p.line)
        .shadow(18.0, 48.0, p.shadow)
        .clip_rounded(12.0)
        .child(
            row()
                .w_full()
                .px(10.0)
                .py(6.0)
                .border_sides([0.0, 0.0, 1.0, 0.0], p.line)
                .child(field.grow(1.0)),
        )
        .child(
            col()
                .key("quick-open-list")
                .w_full()
                .h((list.len().max(1) as f32 * 36.0 + 12.0).min(384.0))
                .scroll(rune::ScrollSpec { stick_bottom: false })
                .child(rows),
        )
        .transition(rune::Transition::opacity());
    col()
        .key("quick-open-backdrop")
        .absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .pt(72.0)
        .modal(true, |s: &mut App| s.quick_open = None)
        .child(row().w_full().justify_center().child(panel))
}
