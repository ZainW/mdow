//! The window: a header with the window buttons, tabs and actions; the sidebar; and the document
//! on a rounded card of paper, with find floating over it.

use crate::app::{App, Cmd, DRAWN_DARK, FindState, Sheet};
use crate::icons;
use crate::reader;
use crate::theme::{self, CARD_RADIUS, GUTTER, HEADER_HEIGHT, Palette, TRAFFIC_LIGHTS_WIDTH};
use rune::{Color, Cx, El, Icon, Key, KeyEvent, Tx, col, el, icon, row, text};
use rune_kit::shell::{TabItem, TabOrientation, TabReorder, TabStrip};

pub fn view(s: &App, cx: &mut Cx<App>) -> El {
    let dark = if rune::is_headless() {
        match s.prefs.appearance {
            crate::prefs::Appearance::Dark => true,
            crate::prefs::Appearance::Light => false,
            crate::prefs::Appearance::System => cx.system_dark(),
        }
    } else {
        cx.system_dark()
    };
    DRAWN_DARK.with(|d| d.set(dark));
    let p = Palette::get(dark);
    let k = p.kit();
    let mut layer: Vec<El> = Vec::new();
    if let Some(kind) = s.panel {
        let body = crate::sheets::window_view(s, kind, &p, &k, &mut layer);
        return rune_kit::root(
            body.on_window_focus(|s: &mut App, focused, _tx: &mut Tx| {
                s.focused = focused;
                crate::windows::set_focused(&s.key, focused);
            }),
            layer,
        );
    }

    let header = header(s, cx, &p, &k, &mut layer);
    let mut body = row().key("body").w_full().grow(1.0).min_h(0.0);
    if s.prefs.sidebar_open {
        body = body.child(crate::sidebar::view(s, &p, &k)).child(sidebar_handle());
    }
    body = body.child(card(s, cx, &p, &k));
    if s.companion.open {
        body = body.child(crate::companion::panel(s, &p, &k));
    }

    let mut root = col()
        .key("window")
        .w_full()
        .h_full()
        .fill(p.chrome)
        .child(header)
        .child(body)
        .on_window_focus(|s: &mut App, focused, _tx: &mut Tx| {
            s.focused = focused;
            crate::windows::set_focused(&s.key, focused);
        })
        .on_drop_files(|s: &mut App, files: &[std::path::PathBuf], tx: &mut Tx| {
            s.open_paths(files, tx);
        });

    if let Some(t) = &s.toast {
        root = root.child(
            row()
                .key("toast")
                .absolute(0.0, 0.0)
                .w_full()
                .h_full()
                .justify_center()
                .items_end()
                .pb(28.0)
                .inert()
                .child(
                    text(t.text.clone())
                        .font(theme::ui(12.5, 500.0))
                        .color(p.text)
                        .px(14.0)
                        .py(8.0)
                        .radius(8.0)
                        .fill(p.elevated)
                        .border(1.0, p.line)
                        .shadow(6.0, 20.0, p.shadow)
                        .transition(rune::Transition::opacity()),
                ),
        );
    }
    if let Some(q) = &s.quick_open {
        layer.push(crate::quick_open::view(s, q, &p, &k));
    }
    if let Some(sheet) = s.sheet {
        let height = cx.layout_size("window").map_or(800.0, |z| z[1]);
        let mut above = Vec::new();
        layer.push(crate::sheets::view(s, sheet, &p, &k, height, &mut above));
        layer.extend(above);
    }
    rune_kit::root(root, layer)
}

/// A flat icon button in the header or sidebar.
pub fn chrome_button(
    key: &str,
    glyph: &'static Icon,
    label: &str,
    p: &Palette,
    enabled: bool,
    on: Option<bool>,
) -> El {
    let color = if !enabled {
        p.tertiary.alpha(0.6)
    } else if on == Some(true) {
        p.text
    } else {
        p.secondary
    };
    let mut b = row()
        .key(key.to_string())
        .size(28.0, 28.0)
        .center()
        .radius(6.0)
        .role("button")
        .label(label.to_string())
        .help(label.to_string())
        .child(icon(glyph, 17.0).color(color));
    if on == Some(true) {
        b = b.fill(p.selected);
    }
    if enabled {
        b = b.hover(p.hover).focusable(true);
    } else {
        b = b.disabled(true);
    }
    if let Some(on) = on {
        b = b.checked(on);
    }
    b
}

fn header(s: &App, cx: &mut Cx<App>, p: &Palette, k: &rune_kit::Kit, layer: &mut Vec<El>) -> El {
    let leading = row()
        .gap(2.0)
        .center_y()
        .pl(TRAFFIC_LIGHTS_WIDTH)
        .child(
            chrome_button(
                "sidebar-toggle",
                &icons::SIDEBAR,
                "Show Sidebar (⌃⌘S)",
                p,
                true,
                Some(s.prefs.sidebar_open),
            )
            .on_press_tx(|s: &mut App, tx: &mut Tx| s.run(Cmd::ToggleSidebar, tx)),
        )
        .child(
            chrome_button("back", &icons::BACK, "Back (⌘[)", p, s.can_go(false), None)
                .on_press_tx(|s: &mut App, tx: &mut Tx| s.run(Cmd::Back, tx)),
        )
        .child(
            chrome_button("forward", &icons::FORWARD, "Forward (⌘])", p, s.can_go(true), None)
                .on_press_tx(|s: &mut App, tx: &mut Tx| s.run(Cmd::Forward, tx)),
        );

    // A tab whose document has a companion edit waiting for review gets a dot.
    let pending = s.companion.pending_review_paths();
    let titles: Vec<(String, String, bool)> = s
        .tabs
        .iter()
        .map(|t| {
            let waiting = t
                .pane()
                .path()
                .is_some_and(|p| pending.iter().any(|q| std::path::Path::new(q) == p));
            (t.id.to_string(), t.title(), waiting)
        })
        .collect();
    let active = s.tab().id.to_string();
    let items = titles.iter().map(|(id, title, waiting)| {
        TabItem::new(id.as_str(), title.as_str())
            .icon(&icons::DOC)
            .status(waiting.then_some(rune_kit::shell::TabItemStatus::Attention))
    });
    let strip = TabStrip::new("tabs", TabOrientation::Horizontal)
        .tabs(items)
        .active(Some(active.as_str()))
        .viewport_width(cx.layout_size("tabs").map(|z| z[0]))
        .drag_state(&s.tab_drag, |s: &mut App| &mut s.tab_drag)
        .close_icon(&icons::CLOSE)
        .fill(p.chrome)
        .trailing_action(
            chrome_button("new-tab", &icons::PLUS, "New Tab (⌘T)", p, true, None)
                .on_press_tx(|s: &mut App, tx: &mut Tx| s.run(Cmd::NewTab, tx)),
        )
        .on_select(|s: &mut App, id: &str, tx: &mut Tx| {
            if let Some(i) = s.tabs.iter().position(|t| t.id.to_string() == id) {
                s.select_tab(i, tx);
            }
        })
        .on_close(|s: &mut App, id: &str, tx: &mut Tx| {
            if let Some(i) = s.tabs.iter().position(|t| t.id.to_string() == id) {
                s.close_tab(i, tx);
            }
        })
        .on_reorder(|s: &mut App, r: TabReorder, _tx: &mut Tx| {
            if r.from < s.tabs.len() && r.to < s.tabs.len() {
                let active_id = s.tab().id;
                let t = s.tabs.remove(r.from);
                s.tabs.insert(r.to, t);
                s.active = s.tabs.iter().position(|t| t.id == active_id).unwrap_or(0);
            }
        })
        .build_layered(k, layer);

    let trailing = row()
        .gap(2.0)
        .center_y()
        .pr(10.0)
        .child(
            chrome_button(
                "quick-open",
                &icons::SEARCH,
                "Quick Open (⌘K)",
                p,
                true,
                Some(s.quick_open.is_some()),
            )
            .on_press_tx(|s: &mut App, tx: &mut Tx| s.run(Cmd::QuickOpen, tx)),
        )
        .child(
            chrome_button(
                "companion",
                &icons::SPARKLE,
                "Companion (⌥⌘0)",
                p,
                true,
                Some(s.companion.open),
            )
            .on_press_tx(|s: &mut App, tx: &mut Tx| s.run(Cmd::ToggleCompanion, tx)),
        )
        .child(
            chrome_button("more", &icons::MORE, "More", p, s.tab().pane().doc.is_some(), None)
                .on_drawn_layout_tx(|s: &mut App, r: [f32; 4], tx: &mut Tx| {
                    s.more_at = [r[0] + r[2] / 2.0, r[1] + r[3]];
                    tx.live_only();
                })
                .on_press_tx(|s: &mut App, tx: &mut Tx| {
                    let split =
                        if s.tab().panes.len() > 1 { "Close Split View" } else { "Split View" };
                    let menu = rune::PopupMenu::new()
                        .item(
                            rune::PopupItem::new("editor", "Open in Editor")
                                .symbol("square.and.pencil"),
                        )
                        .item(rune::PopupItem::new("finder", "Show in Finder").symbol("folder"))
                        .item(rune::PopupItem::new("copy", "Copy Path").symbol("doc.on.clipboard"))
                        .item(rune::PopupItem::new("share", "Share…").symbol("square.and.arrow.up"))
                        .separator()
                        .item(rune::PopupItem::new("split", split).symbol("rectangle.split.2x1"))
                        .item(rune::PopupItem::new("reload", "Reload").symbol("arrow.clockwise"))
                        .separator()
                        .item(rune::PopupItem::new("pdf", "Export as PDF…").symbol("doc.richtext"))
                        .item(rune::PopupItem::new("print", "Print…").symbol("printer"));
                    tx.popup_menu(
                        menu,
                        "more",
                        |s: &mut App, picked: Option<&str>, tx: &mut Tx| {
                            let cmd = match picked {
                                Some("finder") => Cmd::ShowInFinder,
                                Some("editor") => Cmd::OpenInEditor,
                                Some("copy") => Cmd::CopyPath,
                                Some("reload") => Cmd::Reload,
                                Some("split") => Cmd::ToggleSplit,
                                Some("share") => Cmd::Share,
                                Some("pdf") => Cmd::ExportPdf,
                                Some("print") => Cmd::Print,
                                _ => return,
                            };
                            s.run(cmd, tx);
                        },
                    );
                }),
        );

    row()
        .key("header")
        .w_full()
        .h(HEADER_HEIGHT)
        .center_y()
        .gap(8.0)
        .window_drag()
        .child(leading)
        .child(row().key("tabs").grow(1.0).min_w(0.0).h_full().center_y().child(strip))
        .child(trailing)
}

fn card(s: &App, cx: &mut Cx<App>, p: &Palette, k: &rune_kit::Kit) -> El {
    let tab = s.tab();
    let width = cx.layout_size("card").map(|z| z[0]).unwrap_or(900.0);
    let panes = tab.panes.len().max(1);
    let split = tab.split.clamp(0.2, 0.8);
    let mut inner = row().w_full().h_full().min_h(0.0);
    for (i, pane) in tab.panes.iter().enumerate() {
        if i > 0 {
            inner = inner.child(split_handle(p, width));
        }
        let share = if panes == 1 {
            1.0
        } else if i == 0 {
            split
        } else {
            1.0 - split
        };
        let pane_width = (width - (panes as f32 - 1.0) * 7.0) * share;
        let focused = i == tab.focus && panes > 1;
        let id = pane.id;
        let content = match &pane.doc {
            None => crate::welcome::view(s, p, k),
            Some(d) if d.error.is_some() => reader::error(d.error.unwrap(), &d.path, p),
            // WebKit draws the page in a native view laid over this one.
            Some(d) if d.html => {
                let dark = p.dark;
                col()
                    .key(format!("web-{id}"))
                    .w_full()
                    .grow(1.0)
                    .min_h(0.0)
                    .fill(p.paper)
                    .on_drawn_layout_tx(move |s: &mut App, r: [f32; 4], tx: &mut Tx| {
                        s.web_rects.insert(id, (r, dark));
                        s.place_web(id);
                        tx.live_only();
                    })
            }
            Some(d) => reader::document(s, id, d, pane_width, p),
        };
        let mut c = col()
            .key(format!("pane-{id}"))
            .grow(share)
            .w(0.0)
            .min_w(0.0)
            .h_full()
            .min_h(0.0)
            .on_press(move |s: &mut App| {
                let tab = s.tab_mut();
                if let Some(i) = tab.panes.iter().position(|p| p.id == id) {
                    tab.focus = i;
                }
            })
            .child(content);
        if focused {
            c = c.child(el().absolute(0.0, 0.0).w_full().h(2.0).fill(p.accent.alpha(0.5)).inert());
        }
        inner = inner.child(c);
    }
    let mut card = col()
        .key("card")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .radius(CARD_RADIUS)
        .clip_rounded(CARD_RADIUS)
        .fill(p.paper)
        .border(1.0, p.line)
        .shadow(1.0, 3.0, p.shadow.alpha(p.shadow.a * 0.5))
        .child(inner);
    if let Some(f) = &s.find {
        card = card.child(find_bar(f, s.front_is_html(), p, k));
    }
    col()
        .grow(1.0)
        .w(0.0)
        .min_w(0.0)
        .h_full()
        .min_h(0.0)
        .pl(if s.prefs.sidebar_open { 0.0 } else { GUTTER })
        .pr(GUTTER)
        .pb(GUTTER)
        .child(card)
}

fn find_bar(f: &FindState, html: bool, p: &Palette, k: &rune_kit::Kit) -> El {
    // WebKit says only whether a page has the text, not how often.
    let count = if html {
        match f.web_found {
            Some(false) if !f.query.text.is_empty() => "No matches".into(),
            _ => String::new(),
        }
    } else {
        match (f.matches.len(), f.current) {
            (0, _) if f.query.text.is_empty() => String::new(),
            (0, _) => "No matches".into(),
            (n, Some(c)) => format!("{} of {n}", c + 1),
            (n, None) => format!("{n} matches"),
        }
    };
    let field = rune_kit::TextInput::new(&f.query)
        .placeholder("Find")
        .label("Find in document")
        .leading_icon(&icons::SEARCH)
        .width(Some(220.0))
        .on_edit(|s: &mut App| &mut s.find.as_mut().expect("find is open").query)
        .on_change(|s: &mut App, tx: &mut Tx| s.find_changed(tx))
        .on_key(|s: &mut App, ev: &KeyEvent, tx: &mut Tx| match &ev.key {
            Key::Enter => {
                s.find_step(!ev.mods.shift, tx);
                true
            }
            Key::Escape => {
                s.find = None;
                true
            }
            _ => false,
        })
        .build(k)
        .key("find-field");
    row().key("find-bar").absolute(0.0, 10.0).w_full().justify_end().pr(14.0).child(
        row()
            .gap(6.0)
            .center_y()
            .padding(6.0)
            .radius(10.0)
            .fill(p.elevated)
            .border(1.0, p.line)
            .shadow(8.0, 24.0, p.shadow)
            .child(field)
            .child(
                text(count)
                    .font(TextStyle_tnum(theme::ui(11.5, 500.0)))
                    .color(p.tertiary)
                    .min_w(64.0)
                    .align(rune::Align::End),
            )
            .child(
                chrome_button(
                    "find-prev",
                    &icons::CHEVRON_UP,
                    "Previous (⇧⌘G)",
                    p,
                    html || !f.matches.is_empty(),
                    None,
                )
                .on_press_tx(|s: &mut App, tx: &mut Tx| s.find_step(false, tx)),
            )
            .child(
                chrome_button(
                    "find-next",
                    &icons::CHEVRON_DOWN,
                    "Next (⌘G)",
                    p,
                    html || !f.matches.is_empty(),
                    None,
                )
                .on_press_tx(|s: &mut App, tx: &mut Tx| s.find_step(true, tx)),
            )
            .child(
                chrome_button("find-close", &icons::CLOSE, "Close (Esc)", p, true, None)
                    .on_press(|s: &mut App| s.find = None),
            )
            .transition(rune::Transition::opacity()),
    )
}

#[allow(non_snake_case)]
fn TextStyle_tnum(style: rune::TextStyle) -> rune::TextStyle {
    rune::TextStyle { features: "'tnum' 1", ..style }
}

/// Keys the menus don't cover: Escape closes whatever floats, and ⌘1–9 pick tabs when the
/// native menu isn't there (headless runs).
pub fn on_key(s: &mut App, ev: &KeyEvent, tx: &mut Tx) -> bool {
    let plain = !ev.mods.shift && !ev.mods.alt && !ev.mods.ctrl;
    match &ev.key {
        Key::Command('a') if plain && s.quick_open.is_none() && s.sheet.is_none() => {
            s.select_all(tx)
        }
        Key::Command('c') if plain && s.select_all.is_some() => s.copy_all(tx),
        Key::Escape => {
            if s.quick_open.is_some() {
                s.quick_open = None;
            } else if s.sheet.is_some() {
                s.sheet = None;
            } else if s.find.is_some() {
                s.find = None;
            } else {
                return false;
            }
            true
        }
        _ if s.companion.open && crate::companion::window_key(s, ev, tx) => true,
        _ if !crate::menu::installed() => crate::menu::fallback_key(s, ev, tx),
        _ => false,
    }
}

#[allow(dead_code)]
fn unused(_: Color, _: Sheet) {}

/// The sidebar's trailing edge: drag to resize it, double-click to reset it.
fn sidebar_handle() -> El {
    el().key("sidebar-resize")
        .w(8.0)
        .h_full()
        .cursor(rune::Cursor::ResizeColumn)
        .label("Resize sidebar")
        .on_drag(|s: &mut App, d: &mut rune::Drag, _tx: &mut Tx| {
            if d.phase == rune::Phase::Down {
                d.data = s.prefs.sidebar_width;
            }
            let width = (d.data + d.dx).clamp(180.0, 420.0).round();
            if d.phase == rune::Phase::Up {
                s.prefs = crate::prefs::update(|p| p.sidebar_width = width);
            } else {
                s.prefs.sidebar_width = width;
            }
        })
        .on_double_press(|s: &mut App| s.prefs = crate::prefs::update(|p| p.sidebar_width = 240.0))
}

/// The line between split panes: drag to share the width differently, double-click to even it.
fn split_handle(p: &Palette, width: f32) -> El {
    row()
        .key("split-resize")
        .w(7.0)
        .h_full()
        .justify_center()
        .cursor(rune::Cursor::ResizeColumn)
        .label("Resize panes")
        .child(el().w(1.0).h_full().fill(p.line))
        .on_drag(move |s: &mut App, d: &mut rune::Drag, _tx: &mut Tx| {
            if d.phase == rune::Phase::Down {
                d.data = s.tab().split;
            }
            s.tab_mut().split = (d.data + d.dx / width.max(1.0)).clamp(0.2, 0.8);
        })
        .on_double_press(|s: &mut App| s.tab_mut().split = 0.5)
}
