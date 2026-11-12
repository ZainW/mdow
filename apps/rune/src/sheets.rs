//! Settings (⌘,) and Keyboard Shortcuts (⌘/): windows of their own in Mdow's style, as in the
//! Swift app (`window_view`), or sheets over the window in headless runs, which have one window.

use crate::app::{App, Sheet};
use crate::icons;
use crate::prefs::{self, Appearance, BodyFont, CodeFont, LineSpacing, ReadingWidth};
use crate::theme::{self, Palette};
use crate::ui::chrome_button;
use rune::{El, Tx, col, el, row, text};

pub fn view(
    s: &App,
    sheet: Sheet,
    p: &Palette,
    k: &rune_kit::Kit,
    window_height: f32,
    above: &mut Vec<El>,
) -> El {
    let height = (window_height - 160.0).clamp(240.0, 600.0);
    let (title, body) = match sheet {
        Sheet::Settings => ("Settings", settings(s, p, k, above)),
        Sheet::Shortcuts => ("Keyboard Shortcuts", shortcuts(p)),
    };
    let panel = col()
        .key("sheet")
        .w(560.0)
        .radius(14.0)
        .fill(p.elevated)
        .border(1.0, p.line)
        .shadow(20.0, 60.0, p.shadow)
        .clip_rounded(14.0)
        .child(
            row()
                .w_full()
                .h(48.0)
                .center_y()
                .px(20.0)
                .border_sides([0.0, 0.0, 1.0, 0.0], p.line)
                .child(text(title).font(theme::ui(15.0, 600.0)).color(p.text).grow(1.0))
                .child(
                    chrome_button("sheet-close", &icons::CLOSE, "Close", p, true, None)
                        .on_press(|s: &mut App| s.sheet = None),
                ),
        )
        .child(
            col()
                .key("sheet-body")
                .w_full()
                .h(height)
                .scroll(rune::ScrollSpec { stick_bottom: false })
                .child(body),
        )
        .transition(rune::Transition::opacity());
    col()
        .key("sheet-backdrop")
        .absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .center()
        .fill(rune::Color::BLACK.alpha(if p.dark { 0.35 } else { 0.12 }))
        .modal(true, |s: &mut App| s.sheet = None)
        .child(panel)
}

/// Settings or Keyboard Shortcuts filling a window of their own.
pub fn window_view(
    s: &App,
    sheet: Sheet,
    p: &Palette,
    k: &rune_kit::Kit,
    above: &mut Vec<El>,
) -> El {
    let (title, body) = match sheet {
        Sheet::Settings => ("Settings", settings(s, p, k, above)),
        Sheet::Shortcuts => ("Keyboard Shortcuts", shortcuts(p)),
    };
    col()
        .key("panel-window")
        .w_full()
        .h_full()
        .fill(p.elevated)
        .child(
            row()
                .w_full()
                .h(theme::HEADER_HEIGHT)
                .center()
                .window_drag()
                .border_sides([0.0, 0.0, 1.0, 0.0], p.line)
                .child(text(title).font(theme::ui(13.0, 600.0)).color(p.text)),
        )
        .child(
            col()
                .key("sheet-body")
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .scroll(rune::ScrollSpec { stick_bottom: false })
                .child(body),
        )
}

fn group(title: &str, p: &Palette, rows: Vec<El>) -> El {
    let mut box_ = col().w_full().radius(10.0).fill(p.paper).border(1.0, p.line);
    let n = rows.len();
    for (i, r) in rows.into_iter().enumerate() {
        let r = r.px(14.0).py(10.0);
        box_ = box_.child(if i + 1 < n { r.border_sides([0.0, 0.0, 1.0, 0.0], p.line) } else { r });
    }
    col()
        .w_full()
        .gap(8.0)
        .child(
            text(title.to_uppercase())
                .font(rune::TextStyle { tracking: 0.5, ..theme::ui(10.5, 600.0) })
                .color(p.tertiary)
                .px(4.0),
        )
        .child(box_)
}

fn setting(label: &str, detail: Option<&str>, control: El, p: &Palette) -> El {
    let mut left = col()
        .grow(1.0)
        .min_w(0.0)
        .gap(2.0)
        .child(text(label.to_string()).font(theme::ui(13.0, 500.0)).color(p.text));
    if let Some(d) = detail {
        left =
            left.child(text(d.to_string()).font(theme::ui(11.5, 400.0)).color(p.tertiary).wrap());
    }
    row().w_full().gap(16.0).center_y().child(left).child(control)
}

/// A native pop-up menu button listing `options`.
fn picker(
    key: &'static str,
    current: &str,
    options: Vec<(String, String)>,
    p: &Palette,
    pick: fn(&mut App, &str),
) -> El {
    let shown =
        options.iter().find(|(id, _)| id == current).map(|(_, t)| t.clone()).unwrap_or_default();
    let current = current.to_string();
    row()
        .key(key)
        .h(28.0)
        .gap(6.0)
        .center_y()
        .px(10.0)
        .radius(6.0)
        .fill(p.well)
        .border(1.0, p.line)
        .hover(p.hover)
        .role("combobox")
        .label(shown.clone())
        .child(text(shown).font(theme::ui(12.5, 500.0)).color(p.text))
        .child(rune::icon(&icons::CHEVRON_DOWN, 12.0).color(p.tertiary))
        .on_press_tx(move |_s: &mut App, tx: &mut Tx| {
            let menu = rune::PopupMenu::picker(options.clone(), &current);
            tx.popup_menu(menu, key, move |s: &mut App, picked: Option<&str>, _tx: &mut Tx| {
                if let Some(id) = picked {
                    pick(s, id);
                }
            });
        })
}

fn segmented<T: Copy + PartialEq + 'static>(
    all: &'static [T],
    current: T,
    name: fn(T) -> &'static str,
    k: &rune_kit::Kit,
    set: fn(&mut prefs::Prefs, T),
) -> El {
    let labels: Vec<&str> = all.iter().map(|v| name(*v)).collect();
    let selected = all.iter().position(|v| *v == current);
    rune_kit::SegmentedControl::new(&labels, selected)
        .fit()
        .on_pick(move |s: &mut App, i: usize, _tx: &mut Tx| {
            if let Some(v) = all.get(i).copied() {
                s.prefs = prefs::update(|pr| set(pr, v));
            }
        })
        .build(k)
}

fn settings(s: &App, p: &Palette, k: &rune_kit::Kit, above: &mut Vec<El>) -> El {
    let pr = &s.prefs;
    let appearance =
        segmented(&Appearance::ALL, pr.appearance, Appearance::name, k, |pr, v| pr.appearance = v);
    let body_font = picker(
        "body-font",
        BodyFont::ALL.iter().position(|f| *f == pr.body_font).unwrap_or(0).to_string().as_str(),
        BodyFont::ALL
            .iter()
            .enumerate()
            .map(|(i, f)| (i.to_string(), f.name().to_string()))
            .collect(),
        p,
        |s, id| {
            if let Some(f) = id.parse::<usize>().ok().and_then(|i| BodyFont::ALL.get(i).copied()) {
                s.prefs = prefs::update(|pr| pr.body_font = f);
            }
        },
    );
    let code_font = picker(
        "code-font",
        CodeFont::ALL.iter().position(|f| *f == pr.code_font).unwrap_or(0).to_string().as_str(),
        CodeFont::ALL
            .iter()
            .enumerate()
            .map(|(i, f)| (i.to_string(), f.name().to_string()))
            .collect(),
        p,
        |s, id| {
            if let Some(f) = id.parse::<usize>().ok().and_then(|i| CodeFont::ALL.get(i).copied()) {
                s.prefs = prefs::update(|pr| pr.code_font = f);
            }
        },
    );
    let spacing = segmented(&LineSpacing::ALL, pr.line_spacing, LineSpacing::name, k, |pr, v| {
        pr.line_spacing = v
    });
    let width = segmented(&ReadingWidth::ALL, pr.reading_width, ReadingWidth::name, k, |pr, v| {
        pr.reading_width = v
    });
    let size = row()
        .gap(4.0)
        .center_y()
        .child(
            chrome_button(
                "text-smaller",
                &icons::CHEVRON_DOWN,
                "Smaller",
                p,
                pr.text_size > 11.0,
                None,
            )
            .on_press(|s: &mut App| {
                s.prefs = prefs::update(|pr| pr.text_size = (pr.text_size - 1.0).max(11.0));
            }),
        )
        .child(
            text(format!("{} pt", pr.text_size.round() as i32))
                .font(rune::TextStyle { features: "'tnum' 1", ..theme::ui(12.5, 500.0) })
                .color(p.text)
                .w(48.0)
                .align(rune::Align::Center),
        )
        .child(
            chrome_button(
                "text-larger",
                &icons::CHEVRON_UP,
                "Larger",
                p,
                pr.text_size < 28.0,
                None,
            )
            .on_press(|s: &mut App| {
                s.prefs = prefs::update(|pr| pr.text_size = (pr.text_size + 1.0).min(28.0));
            }),
        );
    let zoom = row()
        .gap(4.0)
        .center_y()
        .child(
            chrome_button("zoom-out", &icons::CHEVRON_DOWN, "Zoom Out", p, pr.can_zoom_out(), None)
                .on_press(|s: &mut App| s.prefs = prefs::update(|pr| pr.zoom_out())),
        )
        .child(
            text(format!("{}%", (pr.zoom * 100.0).round() as i32))
                .font(rune::TextStyle { features: "'tnum' 1", ..theme::ui(12.5, 500.0) })
                .color(p.text)
                .w(48.0)
                .align(rune::Align::Center),
        )
        .child(
            chrome_button("zoom-in", &icons::CHEVRON_UP, "Zoom In", p, pr.can_zoom_in(), None)
                .on_press(|s: &mut App| s.prefs = prefs::update(|pr| pr.zoom_in())),
        );
    let tabs = rune_kit::Switch::new(pr.open_in_tabs)
        .label("Open documents in tabs")
        .hide_label()
        .on_toggle(|s: &mut App| s.prefs = prefs::update(|pr| pr.open_in_tabs = !pr.open_in_tabs))
        .build(k)
        .key("open-in-tabs");
    let reload = rune_kit::Switch::new(pr.reload_on_change)
        .label("Reload when the file changes")
        .hide_label()
        .on_toggle(|s: &mut App| {
            s.prefs = prefs::update(|pr| pr.reload_on_change = !pr.reload_on_change)
        })
        .build(k)
        .key("reload-on-change");
    let darken = rune_kit::Switch::new(pr.darken_html)
        .label("Darken HTML pages")
        .hide_label()
        .on_toggle(|s: &mut App| s.prefs = prefs::update(|pr| pr.darken_html = !pr.darken_html))
        .build(k)
        .key("darken-html");
    let reset = rune_kit::Button::new("Reset Reading Settings")
        .variant(rune_kit::ButtonVariant::Ghost)
        .on_press(|s: &mut App| s.prefs = prefs::update(|pr| pr.reset_reading()))
        .build(k)
        .key("reset-reading");

    col()
        .w_full()
        .gap(18.0)
        .p_all(20.0)
        .child(group("Appearance", p, vec![setting("Theme", None, appearance, p)]))
        .child(group(
            "Reading",
            p,
            vec![
                setting("Text font", None, body_font, p),
                setting("Code font", None, code_font, p),
                setting("Text size", None, size, p),
                setting("Zoom", Some("⌘+ and ⌘- change it from anywhere."), zoom, p),
                setting("Line spacing", None, spacing, p),
                setting("Text width", None, width, p),
            ],
        ))
        .child(group(
            "Behaviour",
            p,
            vec![
                setting(
                    "Open documents in tabs",
                    Some("Otherwise a document replaces the one in front."),
                    tabs,
                    p,
                ),
                setting(
                    "Reload when the file changes",
                    Some("Keeps your place while you edit elsewhere."),
                    reload,
                    p,
                ),
                setting(
                    "Darken HTML pages",
                    Some("Pages without a dark look of their own are inverted in dark mode."),
                    darken,
                    p,
                ),
            ],
        ))
        .child(crate::companion::settings_group_layered(s, p, k, above))
        .child(row().w_full().justify_end().child(reset))
}

/// Every command with a shortcut, grouped as the menus are.
pub const SHORTCUTS: &[(&str, &[(&str, &str)])] = &[
    (
        "File",
        &[
            ("New Window", "⌘N"),
            ("New Tab", "⌘T"),
            ("Open…", "⌘O"),
            ("Open Folder…", "⇧⌘O"),
            ("Close Tab", "⌘W"),
            ("Close Window", "⇧⌘W"),
            ("Reload", "⌘R"),
            ("Show in Finder", "⌥⌘R"),
            ("Open in Editor", "⇧⌘E"),
            ("Copy Path", "⌥⌘C"),
        ],
    ),
    (
        "Find",
        &[
            ("Find…", "⌘F"),
            ("Find Next", "⌘G"),
            ("Find Previous", "⇧⌘G"),
            ("Use Selection for Find", "⌘E"),
            ("Quick Open", "⌘K"),
        ],
    ),
    (
        "View",
        &[
            ("Show Sidebar", "⌃⌘S"),
            ("Outline", "⌃⌘1"),
            ("Files", "⌃⌘2"),
            ("Recents", "⌃⌘3"),
            ("Show Companion", "⌥⌘0"),
            ("Split View", "⌘\\"),
            ("Actual Size", "⌘0"),
            ("Zoom In", "⌘+"),
            ("Zoom Out", "⌘-"),
            ("Enter Full Screen", "⌃⌘F"),
        ],
    ),
    (
        "Go",
        &[("Back", "⌘["), ("Forward", "⌘]"), ("Previous Heading", "⌥⌘↑"), ("Next Heading", "⌥⌘↓")],
    ),
    (
        "Window",
        &[
            ("Show Previous Tab", "⇧⌘["),
            ("Show Next Tab", "⇧⌘]"),
            ("Next Tab", "⌃⇥"),
            ("Tab 1–8, Last Tab", "⌘1–⌘9"),
        ],
    ),
    ("App", &[("Settings…", "⌘,"), ("Keyboard Shortcuts", "⌘/"), ("Quit", "⌘Q")]),
];

/// Shortcuts for things only the Mac build does.
const MAC_ONLY_KEYS: &[&str] = &["⌃⌘F"];

fn shortcuts(p: &Palette) -> El {
    let mut c = col().w_full().gap(18.0).p_all(20.0);
    for (title, items) in SHORTCUTS {
        let rows: Vec<El> = items
            .iter()
            .filter(|(_, keys)| crate::platform::MAC || !MAC_ONLY_KEYS.contains(keys))
            .map(|(label, keys)| {
                let (label, keys) = (crate::platform::label(label), crate::platform::label(keys));
                row()
                    .w_full()
                    .center_y()
                    .child(
                        text(label.to_string())
                            .font(theme::ui(13.0, 400.0))
                            .color(p.text)
                            .grow(1.0),
                    )
                    .child(
                        text(keys.to_string())
                            .font(theme::ui(12.0, 500.0))
                            .color(p.secondary)
                            .px(7.0)
                            .py(2.0)
                            .radius(5.0)
                            .fill(p.well)
                            .border(1.0, p.line),
                    )
            })
            .collect();
        c = c.child(group(title, p, rows));
    }
    c.child(el().h(4.0))
}

trait Pad {
    fn p_all(self, v: f32) -> Self;
}
impl Pad for El {
    fn p_all(self, v: f32) -> Self {
        self.px(v).py(v)
    }
}
