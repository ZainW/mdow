//! Mdow for Linux: the Mdow markdown viewer built natively on GTK 4 and libadwaita.

mod companion;
mod dialogs;
mod doc;
mod files;
mod fonts;
mod highlight;
mod media;
mod omarchy;
mod perf;
mod prefs;
mod print;
mod reader;
mod render;
mod segmented;
mod sidebar;
mod theme;
mod welcome;
mod window;

use adw::prelude::*;
use gtk::{gio, glib};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use window::Win;

pub const APP_ID: &str = "com.zain.mdow.Linux";

thread_local! {
    static PREFS: RefCell<Option<Rc<RefCell<prefs::Prefs>>>> = const { RefCell::new(None) };
}

/// Names an icon-only control for screen readers. Tooltips carry their shortcut in brackets, which
/// isn't part of the name.
pub fn set_accessible_name(widget: &impl IsA<gtk::Accessible>, tooltip: &str) {
    let name = tooltip.split(" (").next().unwrap_or(tooltip);
    widget.update_property(&[gtk::accessible::Property::Label(name)]);
}

fn prefs_path() -> PathBuf {
    match std::env::var_os("MDOW_STATE") {
        Some(path) => PathBuf::from(path),
        None => prefs::Prefs::path(),
    }
}

/// Put the app icon (for uninstalled runs) and Mdow's own symbolic icons where the icon theme
/// can find them.
fn install_runtime_icons() {
    let dir = glib::user_cache_dir().join("mdow-linux").join("icons");
    let scalable = dir.join("hicolor").join("scalable");
    let apps = scalable.join("apps");
    if std::fs::create_dir_all(&apps).is_ok() {
        let _ = std::fs::write(apps.join(format!("{APP_ID}.svg")), media::LOGO_SVG);
    }
    let actions = scalable.join("actions");
    let symbolic: [(&str, &[u8]); 3] = [
        (
            "mdow-companion-symbolic.svg",
            include_bytes!("../data/icons/mdow-companion-symbolic.svg"),
        ),
        ("mdow-wide-symbolic.svg", include_bytes!("../data/icons/mdow-wide-symbolic.svg")),
        ("mdow-narrow-symbolic.svg", include_bytes!("../data/icons/mdow-narrow-symbolic.svg")),
    ];
    if std::fs::create_dir_all(&actions).is_ok() {
        for (name, bytes) in symbolic {
            let _ = std::fs::write(actions.join(name), bytes);
        }
    }
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::IconTheme::for_display(&display).add_search_path(dir);
    }
}

/// The preferences every window shares. A new install on Omarchy uses the system's font for code.
fn shared_prefs() -> Rc<RefCell<prefs::Prefs>> {
    if let Some(prefs) = PREFS.with_borrow(Clone::clone) {
        return prefs;
    }
    let path = prefs_path();
    let mut loaded = prefs::Prefs::load_from(&path);
    if !path.exists() && window::on_omarchy() {
        loaded.code_font = prefs::CodeFont::SystemMono;
    }
    let prefs = Rc::new(RefCell::new(loaded));
    PREFS.with_borrow_mut(|p| *p = Some(prefs.clone()));
    prefs
}

/// The window to open things in: the last focused one, or the first, with its session restored.
fn main_window(app: &adw::Application) -> Win {
    let windows = Win::all();
    let focused = app.active_window();
    if let Some(win) = windows.iter().find(|w| focused.as_ref() == Some(w.window().upcast_ref())) {
        return win.clone();
    }
    if let Some(win) = windows.into_iter().next() {
        return win;
    }
    let win = Win::new(app, shared_prefs(), prefs_path());
    perf::step("window built");
    win.restore_session();
    perf::step("session restored");
    if let Ok(target) = std::env::var("MDOW_SCREENSHOT") {
        schedule_screenshot(&win, target);
    }
    if let Some(scenarios) = perf::bench() {
        win.run_bench(scenarios);
    }
    win
}

/// Debug aid: render the window offscreen to a PNG after it settles, then quit.
fn schedule_screenshot(win: &Win, target: String) {
    let delay =
        std::env::var("MDOW_SCREENSHOT_DELAY").ok().and_then(|v| v.parse().ok()).unwrap_or(1500);
    let window = win.window().clone();
    let prepare = win.clone();
    glib::timeout_add_local_once(std::time::Duration::from_millis(delay / 2), move || {
        if let Ok(target) = std::env::var("MDOW_SCREENSHOT_SCROLL") {
            prepare.debug_scroll(&target);
        }
        if let Ok(query) = std::env::var("MDOW_SCREENSHOT_FIND") {
            prepare.debug_find(&query);
        }
        if std::env::var_os("MDOW_SCREENSHOT_CHEAT").is_some() {
            prepare.debug_cheat_sheet();
        }
        if let Ok(actions) = std::env::var("MDOW_SCREENSHOT_ACTION") {
            for action in actions.split(';').filter(|a| !a.is_empty()) {
                let (name, arg) = match action.split_once('=') {
                    Some((n, a)) => (n, Some(a.to_variant())),
                    None => (action, None),
                };
                let _ = WidgetExt::activate_action(prepare.window(), name, arg.as_ref());
            }
        }
    });
    glib::timeout_add_local_once(std::time::Duration::from_millis(delay), move || {
        let paintable = gtk::WidgetPaintable::new(Some(&window));
        let (w, h) = (window.width() as f64, window.height() as f64);
        let snapshot = gtk::Snapshot::new();
        paintable.snapshot(&snapshot, w, h);
        let node = snapshot.to_node();
        let renderer = window.renderer();
        match (node, renderer) {
            (Some(node), Some(renderer)) => {
                let texture = renderer.render_texture(&node, None);
                if let Err(err) = texture.save_to_png(&target) {
                    eprintln!("mdow: screenshot failed: {err}");
                }
            }
            _ => eprintln!("mdow: nothing to screenshot"),
        }
        if let Some(app) = window.application() {
            app.quit();
        }
    });
}

fn main() -> glib::ExitCode {
    if std::env::args().any(|a| a == "--version") {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return glib::ExitCode::SUCCESS;
    }
    perf::init();
    fonts::register();
    perf::step("fonts registered");
    let mut flags = gio::ApplicationFlags::HANDLES_OPEN;
    if std::env::var_os("MDOW_SCREENSHOT").is_some() || perf::bench().is_some() {
        flags |= gio::ApplicationFlags::NON_UNIQUE;
    }
    let app = adw::Application::builder().application_id(APP_ID).flags(flags).build();
    app.connect_startup(|app| {
        perf::step("GTK started");
        window::init_omarchy();
        perf::step("Omarchy theme read");
        if std::env::var_os("MDOW_SCREENSHOT").is_some()
            && let Some(settings) = gtk::Settings::default()
        {
            // Headless captures have no client acknowledging frames, so animations never finish.
            settings.set_gtk_enable_animations(false);
        }
        highlight::preload();
        install_runtime_icons();
        perf::step("icons installed");
        window::set_accels(app);
        let quit = gio::SimpleAction::new("quit", None);
        let weak = app.downgrade();
        quit.connect_activate(move |_, _| {
            for win in Win::all() {
                win.save_now();
            }
            if let Some(app) = weak.upgrade() {
                app.quit();
            }
        });
        app.add_action(&quit);
    });
    app.connect_shutdown(|_| companion::service::Runtime::shared().stop_now());
    app.connect_activate(|app| {
        main_window(app).window().present();
        perf::step("window presented");
    });
    app.connect_open(|app, files, _| {
        let win = main_window(app);
        let paths: Vec<PathBuf> = files.iter().filter_map(|f| f.path()).collect();
        for (i, path) in paths.iter().enumerate() {
            win.open_path(path, None, i + 1 == paths.len());
        }
        win.window().present();
        perf::step("window presented");
    });
    app.run()
}
