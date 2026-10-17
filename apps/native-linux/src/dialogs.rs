//! Command palette, Settings, Keyboard Shortcuts and the hold-Ctrl cheat sheet — the desktop's
//! `CommandPalette.tsx`, `SettingsDialog.tsx`, `ShortcutsDialog.tsx` and `CheatSheetOverlay.tsx`,
//! built from libadwaita dialogs and toggle groups.

use crate::files;
use crate::prefs::{CodeFont, ContentFont, InterfaceScale, Prefs, ReadingWidth, ThemeMode};
use crate::segmented::Segmented;
use adw::prelude::*;
use gtk::{gdk, glib};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

// ── Command palette ──────────────────────────────────────────────────────────

struct PaletteAction {
    label: &'static str,
    hint: &'static str,
    keywords: &'static str,
    icon: &'static str,
    action: &'static str,
}

const ACTIONS: &[PaletteAction] = &[
    PaletteAction {
        label: "Open File",
        hint: "Choose a markdown or HTML document",
        keywords: "document markdown html",
        icon: "document-open-symbolic",
        action: "win.open-file",
    },
    PaletteAction {
        label: "Open Folder",
        hint: "Browse a folder of documents",
        keywords: "workspace directory sidebar",
        icon: "folder-open-symbolic",
        action: "win.open-folder",
    },
    PaletteAction {
        label: "Toggle Sidebar",
        hint: "Show or hide navigation",
        keywords: "recents folder outline",
        icon: "sidebar-show-symbolic",
        action: "win.toggle-sidebar",
    },
    PaletteAction {
        label: "Find in Document",
        hint: "Search the active document",
        keywords: "search matches",
        icon: "edit-find-symbolic",
        action: "win.find",
    },
    PaletteAction {
        label: "Toggle Wide Mode",
        hint: "Switch reading width",
        keywords: "full width reading layout",
        icon: "mdow-wide-symbolic",
        action: "win.wide-mode",
    },
    PaletteAction {
        label: "Toggle Split View",
        hint: "Read side by side",
        keywords: "side by side pane columns",
        icon: "view-dual-symbolic",
        action: "win.split-view",
    },
    PaletteAction {
        label: "Settings",
        hint: "Theme, fonts, layout",
        keywords: "preferences appearance",
        icon: "emblem-system-symbolic",
        action: "win.settings",
    },
    PaletteAction {
        label: "Keyboard Shortcuts",
        hint: "View available shortcuts",
        keywords: "help keys",
        icon: "input-keyboard-symbolic",
        action: "win.shortcuts",
    },
];

fn palette_row(icon: &str, label: &str, detail: &str, target: String) -> gtk::ListBoxRow {
    let b = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    b.append(&gtk::Image::from_icon_name(icon));
    let l = gtk::Label::new(Some(label));
    l.set_xalign(0.0);
    l.set_ellipsize(gtk::pango::EllipsizeMode::End);
    b.append(&l);
    let d = gtk::Label::new(Some(detail));
    d.add_css_class("mdow-palette-hint");
    d.set_hexpand(true);
    d.set_xalign(1.0);
    d.set_ellipsize(gtk::pango::EllipsizeMode::Start);
    b.append(&d);
    let row = gtk::ListBoxRow::new();
    row.set_child(Some(&b));
    row.set_widget_name(&target);
    row
}

fn heading_row(text: &str) -> gtk::ListBoxRow {
    let l = gtk::Label::new(Some(text));
    l.add_css_class("mdow-palette-heading");
    l.set_xalign(0.0);
    let row = gtk::ListBoxRow::new();
    row.set_child(Some(&l));
    row.set_selectable(false);
    row.set_activatable(false);
    row.set_can_focus(false);
    row
}

fn select_step(list: &gtk::ListBox, step: i32) {
    let rows: Vec<gtk::ListBoxRow> = {
        let mut out = Vec::new();
        let mut child = list.first_child();
        while let Some(widget) = child {
            if let Ok(row) = widget.clone().downcast::<gtk::ListBoxRow>()
                && row.is_selectable()
            {
                out.push(row);
            }
            child = widget.next_sibling();
        }
        out
    };
    if rows.is_empty() {
        return;
    }
    let current = list.selected_row().and_then(|s| rows.iter().position(|r| *r == s));
    let next = match current {
        Some(i) => (i as i32 + step).clamp(0, rows.len() as i32 - 1) as usize,
        None => 0,
    };
    list.select_row(Some(&rows[next]));
    rows[next].grab_focus();
}

/// `files`: `(path, name)` for the open folder, then recents outside it.
pub fn present_palette(parent: &impl IsA<gtk::Widget>, files: Vec<(String, String)>) {
    let parent: gtk::Widget = parent.clone().upcast();
    let dialog = adw::Dialog::new();
    dialog.set_content_width(512);
    dialog.set_title("Command Palette");
    dialog.add_css_class("mdow-palette");

    let entry = gtk::SearchEntry::new();
    entry.set_placeholder_text(Some("Search files and commands…"));
    entry.add_css_class("mdow-palette-entry");
    entry.set_search_delay(0);
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.set_activate_on_single_click(true);
    let empty = gtk::Label::new(Some("No matching files or commands"));
    empty.add_css_class("mdow-palette-empty");
    list.set_placeholder(Some(&empty));
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_height(true)
        .max_content_height(360)
        .child(&list)
        .build();
    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    footer.add_css_class("mdow-palette-footer");
    footer.set_halign(gtk::Align::Fill);
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    footer.append(&spacer);
    for (key, label) in [("↵", "run/open"), ("esc", "dismiss")] {
        let item = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        let k = gtk::Label::new(Some(key));
        k.add_css_class("mdow-kbd");
        item.append(&k);
        item.append(&gtk::Label::new(Some(label)));
        footer.append(&item);
    }
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.append(&entry);
    content.append(&scroll);
    content.append(&footer);
    dialog.set_child(Some(&content));

    let files = Rc::new(files);
    let fill = {
        let list = list.clone();
        let files = files.clone();
        move |query: &str| {
            list.remove_all();
            let action_items: Vec<(String, String)> = ACTIONS
                .iter()
                .map(|a| (format!("{} {} {}", a.label, a.hint, a.keywords), a.label.to_string()))
                .collect();
            let hits = files::fuzzy_search(query, &action_items, ACTIONS.len());
            let actions: Vec<&PaletteAction> = ACTIONS
                .iter()
                .filter(|a| query.trim().is_empty() || hits.iter().any(|h| h.1 == a.label))
                .collect();
            if !actions.is_empty() {
                list.append(&heading_row("Actions"));
                for a in actions {
                    list.append(&palette_row(a.icon, a.label, a.hint, format!("a:{}", a.action)));
                }
            }
            let results = files::fuzzy_search(query, &files, 50);
            if !results.is_empty() {
                list.append(&heading_row("Files"));
                for (path, name) in results {
                    let dir = files::display_dir(Path::new(path));
                    list.append(&palette_row(
                        "text-x-generic-symbolic",
                        name,
                        &dir,
                        format!("f:{path}"),
                    ));
                }
            }
            let mut child = list.first_child();
            while let Some(widget) = child {
                if let Some(row) = widget.downcast_ref::<gtk::ListBoxRow>()
                    && row.is_selectable()
                {
                    list.select_row(Some(row));
                    break;
                }
                child = widget.next_sibling();
            }
        }
    };
    fill("");
    let fill = Rc::new(fill);

    let run = {
        let dialog = dialog.downgrade();
        let parent = parent.clone();
        move |row: &gtk::ListBoxRow| {
            let target = row.widget_name().to_string();
            if let Some(dialog) = dialog.upgrade() {
                dialog.close();
            }
            if let Some(action) = target.strip_prefix("a:") {
                let _ = parent.activate_action(action, None);
            } else if let Some(path) = target.strip_prefix("f:") {
                let _ = parent.activate_action("win.open-path", Some(&path.to_variant()));
            }
        }
    };
    let run = Rc::new(run);
    {
        let run = run.clone();
        list.connect_row_activated(move |_, row| run(row));
    }
    {
        let fill = fill.clone();
        entry.connect_search_changed(move |entry| fill(&entry.text()));
    }
    {
        let list = list.clone();
        let run = run.clone();
        entry.connect_activate(move |_| {
            if let Some(row) = list.selected_row() {
                run(&row);
            }
        });
    }
    let keys = gtk::EventControllerKey::new();
    {
        let list = list.clone();
        let entry_weak = entry.downgrade();
        keys.connect_key_pressed(move |_, key, _, _| {
            let step = match key {
                gdk::Key::Down | gdk::Key::KP_Down => 1,
                gdk::Key::Up | gdk::Key::KP_Up => -1,
                gdk::Key::Page_Down => 8,
                gdk::Key::Page_Up => -8,
                _ => return glib::Propagation::Proceed,
            };
            select_step(&list, step);
            if let Some(entry) = entry_weak.upgrade()
                && !entry.has_focus()
            {
                entry.grab_focus();
                entry.set_position(-1);
            }
            glib::Propagation::Stop
        });
    }
    entry.add_controller(keys);
    entry.set_key_capture_widget(Some(&content));
    dialog.set_focus(Some(&entry));
    dialog.present(Some(&parent));
}

// ── Settings ─────────────────────────────────────────────────────────────────

fn field(label: &str, child: &impl IsA<gtk::Widget>) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let l = gtk::Label::new(Some(label));
    l.add_css_class("mdow-field-label");
    l.set_xalign(0.0);
    b.append(&l);
    b.append(child);
    b
}

fn segmented<T: Copy + 'static>(
    options: &[(T, &str, &str, Option<&str>)],
    current: &str,
    on_change: impl Fn(&str) + 'static,
) -> Segmented {
    let group = Segmented::new("mdow-segmented");
    for (_, id, label, icon) in options {
        match icon {
            Some(icon) => group.add_icon_label(id, icon, label),
            None => group.add_label(id, label),
        }
    }
    group.set_active_name(current);
    group.connect_changed(on_change);
    group
}

struct Tile {
    button: gtk::ToggleButton,
    id: &'static str,
}

fn tiles(
    items: &[(&'static str, &'static str, String)],
    glyph: &str,
    mono: bool,
    current: &str,
    on_change: impl Fn(&str) + 'static,
) -> (gtk::Grid, Rc<Vec<Tile>>) {
    let on_change: Rc<dyn Fn(&str)> = Rc::new(on_change);
    let grid = gtk::Grid::new();
    grid.set_column_spacing(6);
    grid.set_column_homogeneous(true);
    let mut out = Vec::new();
    let mut first: Option<gtk::ToggleButton> = None;
    for (i, (id, label, family)) in items.iter().enumerate() {
        let button = gtk::ToggleButton::new();
        button.add_css_class("mdow-tile");
        let overlay = gtk::Overlay::new();
        let b = gtk::Box::new(gtk::Orientation::Vertical, 6);
        b.set_halign(gtk::Align::Center);
        let g = gtk::Label::new(None);
        g.set_markup(&format!(
            "<span font_family=\"{}\">{}</span>",
            glib::markup_escape_text(family),
            glib::markup_escape_text(glyph)
        ));
        g.add_css_class("mdow-tile-glyph");
        if mono {
            g.add_css_class("mdow-mono-glyph");
        }
        b.append(&g);
        let l = gtk::Label::new(Some(label));
        l.add_css_class("mdow-tile-label");
        b.append(&l);
        overlay.set_child(Some(&b));
        let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        dot.add_css_class("mdow-tile-dot");
        dot.set_halign(gtk::Align::End);
        dot.set_valign(gtk::Align::Start);
        dot.set_margin_top(0);
        dot.set_margin_end(0);
        dot.set_can_target(false);
        overlay.add_overlay(&dot);
        button.set_child(Some(&overlay));
        match &first {
            Some(f) => button.set_group(Some(f)),
            None => first = Some(button.clone()),
        }
        button.set_active(*id == current);
        dot.set_visible(*id == current);
        let id_owned = *id;
        let dot_weak = dot.downgrade();
        let cb = on_change.clone();
        button.connect_toggled(move |button| {
            if let Some(dot) = dot_weak.upgrade() {
                dot.set_visible(button.is_active());
            }
            if button.is_active() {
                cb(id_owned);
            }
        });
        grid.attach(&button, i as i32, 0, 1, 1);
        out.push(Tile { button, id });
    }
    (grid, Rc::new(out))
}

pub fn present_settings(parent: &impl IsA<gtk::Widget>, prefs: &Prefs) {
    let parent: gtk::Widget = parent.clone().upcast();
    let dialog = adw::Dialog::new();
    dialog.set_title("Settings");
    dialog.set_content_width(448);
    dialog.add_css_class("mdow-settings");

    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&adw::WindowTitle::new("Settings", "Tune how markdown reads.")));
    toolbar.add_top_bar(&header);

    let body = gtk::Box::new(gtk::Orientation::Vertical, 20);
    body.set_margin_start(24);
    body.set_margin_end(24);
    body.set_margin_top(8);
    body.set_margin_bottom(24);

    // Decorative preview in the chosen fonts.
    let preview = gtk::Box::new(gtk::Orientation::Vertical, 6);
    preview.add_css_class("mdow-preview");
    let preview_title = gtk::Label::new(None);
    preview_title.add_css_class("mdow-preview-title");
    preview_title.set_xalign(0.0);
    let preview_text = gtk::Label::new(None);
    preview_text.add_css_class("mdow-preview-text");
    preview_text.set_xalign(0.0);
    preview_text.set_wrap(true);
    preview.append(&preview_title);
    preview.append(&preview_text);
    body.append(&preview);
    let fonts = Rc::new(RefCell::new((prefs.content_font, prefs.code_font)));
    let render_preview = {
        let preview_title = preview_title.clone();
        let preview_text = preview_text.clone();
        let fonts = fonts.clone();
        move || {
            let (content, code) = *fonts.borrow();
            let family = content_pango_family(content);
            preview_title.set_markup(&format!(
                "<span font_family=\"{family}\" letter_spacing=\"-200\">The quiet morning</span>"
            ));
            preview_text.set_markup(&format!(
                "<span font_family=\"{family}\" line_height=\"1.65\">Words on the page settle into their rhythm, and <span font_family=\"{}\" size=\"90%\">ligatures</span> too.</span>",
                code.pango_family()
            ));
        }
    };
    render_preview();
    let render_preview = Rc::new(render_preview);

    let activate = {
        let parent = parent.clone();
        move |action: &str, value: &str| {
            let _ = parent.activate_action(action, Some(&value.to_variant()));
        }
    };
    let activate = Rc::new(activate);

    let theme_options: Vec<(ThemeMode, &str, &str, Option<&str>)> = vec![
        (ThemeMode::System, "system", "System", Some("computer-symbolic")),
        (ThemeMode::Light, "light", "Light", Some("weather-clear-symbolic")),
        (ThemeMode::Dark, "dark", "Dark", Some("weather-clear-night-symbolic")),
    ];
    let a = activate.clone();
    let theme = segmented(&theme_options, prefs.theme.id(), move |v| a("win.theme", v));
    body.append(&field("Theme", theme.widget()));

    let scale_options: Vec<(InterfaceScale, &str, &str, Option<&str>)> =
        InterfaceScale::ALL.iter().map(|s| (*s, s.id(), s.label(), None)).collect();
    let a = activate.clone();
    let scale =
        segmented(&scale_options, prefs.interface_scale.id(), move |v| a("win.interface-scale", v));
    body.append(&field("Interface scale", scale.widget()));

    let width_options: Vec<(ReadingWidth, &str, &str, Option<&str>)> =
        ReadingWidth::ALL.iter().map(|w| (*w, w.id(), w.label(), None)).collect();
    let a = activate.clone();
    let width =
        segmented(&width_options, prefs.reading_width.id(), move |v| a("win.reading-width", v));
    body.append(&field("Reading width", width.widget()));

    let content_items: Vec<(&'static str, &'static str, String)> = ContentFont::ALL
        .iter()
        .map(|f| (f.id(), f.label(), content_pango_family(*f).to_string()))
        .collect();
    let (a, f, rp) = (activate.clone(), fonts.clone(), render_preview.clone());
    let (content_grid, content_tiles) =
        tiles(&content_items, "Aa", false, prefs.content_font.id(), move |v| {
            if let Some(font) = ContentFont::from_id(v) {
                f.borrow_mut().0 = font;
                rp();
            }
            a("win.content-font", v);
        });
    body.append(&field("Content font", &content_grid));

    let code_items: Vec<(&'static str, &'static str, String)> =
        CodeFont::ALL.iter().map(|f| (f.id(), f.label(), f.pango_family().to_string())).collect();
    let (a, f, rp) = (activate.clone(), fonts.clone(), render_preview.clone());
    let (code_grid, code_tiles) =
        tiles(&code_items, "() => {}", true, prefs.code_font.id(), move |v| {
            if let Some(font) = CodeFont::from_id(v) {
                f.borrow_mut().1 = font;
                rp();
            }
            a("win.code-font", v);
        });
    body.append(&field("Code font", &code_grid));

    let rule = gtk::Separator::new(gtk::Orientation::Horizontal);
    rule.add_css_class("mdow-rule");
    body.append(&rule);
    let reset = gtk::Button::with_label("Reset to defaults");
    reset.add_css_class("mdow-small");
    reset.add_css_class("mdow-outline");
    reset.set_halign(gtk::Align::End);
    {
        let parent = parent.clone();
        let defaults = Prefs::default();
        let (theme, scale, width) = (theme.clone(), scale.clone(), width.clone());
        let (content_tiles, code_tiles) = (content_tiles.clone(), code_tiles.clone());
        reset.connect_clicked(move |_| {
            let _ = parent.activate_action("win.reset-appearance", None);
            theme.set_active_name(defaults.theme.id());
            scale.set_active_name(defaults.interface_scale.id());
            width.set_active_name(defaults.reading_width.id());
            for t in content_tiles.iter() {
                if t.id == defaults.content_font.id() {
                    t.button.set_active(true);
                }
            }
            for t in code_tiles.iter() {
                if t.id == defaults.code_font.id() {
                    t.button.set_active(true);
                }
            }
        });
    }
    body.append(&reset);

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_height(true)
        .child(&body)
        .build();
    toolbar.set_content(Some(&scroll));
    dialog.set_child(Some(&toolbar));
    dialog.present(Some(&parent));
}

pub fn content_pango_family(font: ContentFont) -> &'static str {
    match font {
        ContentFont::Inter => "Inter Variable, Inter, sans-serif",
        ContentFont::Charter => "Charter, Bitstream Charter, Georgia, serif",
        ContentFont::SystemSans => "sans-serif",
        ContentFont::Georgia => "Georgia, Times New Roman, serif",
    }
}

// ── Shortcuts ────────────────────────────────────────────────────────────────

pub struct ShortcutSection {
    pub heading: &'static str,
    pub items: Vec<(&'static str, Vec<&'static str>)>,
}

/// One source of truth for the dialog and the cheat sheet, as `cheat-sheet.ts` on desktop.
pub fn shortcut_columns() -> Vec<Vec<ShortcutSection>> {
    vec![
        vec![ShortcutSection {
            heading: "Navigation",
            items: vec![
                ("Command palette", vec!["Ctrl", "K"]),
                ("Find in document", vec!["Ctrl", "F"]),
                ("Switch tab", vec!["Ctrl", "1–9"]),
                ("Next tab", vec!["Ctrl", "Alt", "→"]),
                ("Previous tab", vec!["Ctrl", "Alt", "←"]),
                ("Close tab", vec!["Ctrl", "W"]),
            ],
        }],
        vec![ShortcutSection {
            heading: "View",
            items: vec![
                ("Toggle sidebar", vec!["Ctrl", "B"]),
                ("Zoom in", vec!["Ctrl", "+"]),
                ("Zoom out", vec!["Ctrl", "−"]),
                ("Reset zoom", vec!["Ctrl", "0"]),
                ("Full screen", vec!["F11"]),
                ("All shortcuts", vec!["Ctrl", "/"]),
            ],
        }],
        vec![
            ShortcutSection {
                heading: "Files",
                items: vec![
                    ("Open file", vec!["Ctrl", "O"]),
                    ("Open folder", vec!["Ctrl", "⇧", "O"]),
                ],
            },
            ShortcutSection { heading: "App", items: vec![("Settings", vec!["Ctrl", ","])] },
        ],
    ]
}

fn keycaps(keys: &[&str]) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Horizontal, 3);
    for key in keys {
        let l = gtk::Label::new(Some(key));
        l.add_css_class("mdow-keycap");
        b.append(&l);
    }
    b
}

fn shortcut_row(label: &str, keys: &[&str]) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.add_css_class("mdow-shortcut-row");
    let l = gtk::Label::new(Some(label));
    l.add_css_class("mdow-shortcut-label");
    l.set_xalign(0.0);
    l.set_hexpand(true);
    row.append(&l);
    row.append(&keycaps(keys));
    row
}

pub fn present_shortcuts(parent: &impl IsA<gtk::Widget>) {
    let dialog = adw::Dialog::new();
    dialog.set_title("Keyboard Shortcuts");
    dialog.set_content_width(384);
    dialog.add_css_class("mdow-shortcuts");
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
    body.set_margin_bottom(16);
    let sections: Vec<ShortcutSection> = shortcut_columns().into_iter().flatten().collect();
    for (i, section) in sections.iter().enumerate() {
        if i > 0 {
            body.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        }
        let h = gtk::Label::new(Some(&section.heading.to_uppercase()));
        h.add_css_class("mdow-shortcut-heading");
        h.set_xalign(0.0);
        body.append(&h);
        for (label, keys) in &section.items {
            body.append(&shortcut_row(label, keys));
        }
    }
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_height(true)
        .child(&body)
        .build();
    toolbar.set_content(Some(&scroll));
    dialog.set_child(Some(&toolbar));
    dialog.present(Some(parent));
}

/// The panel shown while Ctrl is held.
pub fn cheat_sheet() -> gtk::Box {
    let panel = gtk::Box::new(gtk::Orientation::Horizontal, 32);
    panel.add_css_class("mdow-cheatsheet");
    panel.set_halign(gtk::Align::Center);
    panel.set_valign(gtk::Align::Center);
    panel.set_can_target(false);
    for column in shortcut_columns() {
        let col = gtk::Box::new(gtk::Orientation::Vertical, 14);
        for section in column {
            let s = gtk::Box::new(gtk::Orientation::Vertical, 6);
            let h = gtk::Label::new(Some(&section.heading.to_uppercase()));
            h.add_css_class("mdow-shortcut-heading");
            h.set_xalign(0.0);
            s.append(&h);
            for (label, keys) in &section.items {
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 16);
                let l = gtk::Label::new(Some(label));
                l.add_css_class("mdow-shortcut-label");
                l.set_xalign(0.0);
                l.set_hexpand(true);
                row.append(&l);
                row.append(&keycaps(keys));
                s.append(&row);
            }
            col.append(&s);
        }
        panel.append(&col);
    }
    panel
}

pub fn present_about(parent: &impl IsA<gtk::Widget>) {
    let about = adw::AboutDialog::builder()
        .application_name("Mdow")
        .application_icon(crate::APP_ID)
        .developer_name("Zain Wania")
        .version(env!("CARGO_PKG_VERSION"))
        .comments("A quiet markdown viewer, native to Linux.")
        .website("https://mdow.wania.app")
        .license_type(gtk::License::MitX11)
        .copyright("© 2026 Zain")
        .build();
    about.present(Some(parent));
}
