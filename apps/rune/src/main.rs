//! Mdow for Mac on Rune: the Swift app's reader, chrome and behaviour, drawn by Rune's GPU
//! renderer instead of AppKit and TextKit.
//!
//! `cargo run -- README.md docs/` opens a window. Rune's headless tools work too:
//! `cargo run -- --snap out.png --size 1200x800 README.md` renders the window without one.

mod actions;
mod app;
mod companion;
mod doc;
mod export;
mod files;
mod highlight;
mod icons;
mod media;
mod menu;
mod omarchy;
mod perf;
mod platform;
mod prefs;
mod quick_open;
mod reader;
mod sheets;
mod sidebar;
mod theme;
mod ui;
mod watch;
mod webview;
mod welcome;
mod windows;

use app::App;
use rune::{AppSpec, Chrome, Titlebar};
use std::path::PathBuf;

/// Documents and folders named on the command line (Rune's own `--` flags are skipped).
fn launch_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut skip_next = false;
    for arg in rune::extra_args() {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg.starts_with("--") {
            // Rune's flags that take a value.
            skip_next = !arg.contains('=')
                && matches!(
                    arg.as_str(),
                    "--snap"
                        | "--scene"
                        | "--at"
                        | "--size"
                        | "--scale"
                        | "--theme"
                        | "--script"
                        | "--frames"
                        | "--start"
                        | "--out"
                        | "--video"
                        | "--fps"
                        | "--duration"
                        | "--tree"
                        | "--trace"
                        | "--window"
                        | "--every"
                        | "--cols"
                        | "--strip"
                        | "--crop"
                        | "--desktop"
                        | "--keys"
                        | "--keys-theme"
                );
            continue;
        }
        if arg.starts_with("-Apple") || arg.starts_with("-NS") {
            skip_next = true;
            continue;
        }
        let path = PathBuf::from(&arg);
        let path = if path.is_absolute() {
            path
        } else {
            std::env::current_dir().unwrap_or_default().join(path)
        };
        if path.exists() {
            out.push(path);
        }
    }
    out
}

fn init_main() -> App {
    let mut paths = launch_paths();
    if paths.is_empty() && !rune::is_headless() && platform::keeps_windows() {
        // Reopen last session's tabs, when "Close windows when quitting an application" is off.
        paths = prefs::get()
            .session
            .into_iter()
            .next()
            .unwrap_or_default()
            .into_iter()
            .filter(|p| p.exists())
            .collect();
    }
    App::new(windows::MAIN.to_string(), paths)
}

fn init_window() -> App {
    let (key, paths) = windows::take_pending().unwrap_or_else(|| (windows::new_key(), Vec::new()));
    App::new(key, paths)
}

fn spec(init: fn() -> App) -> AppSpec<App> {
    AppSpec {
        fonts: theme::FONTS,
        families: (theme::UI_FAMILY, theme::MONO_FAMILY),
        min_size: Some((640.0, 420.0)),
        setup: Some(|s, waker| s.setup(waker)),
        poll: Some(|s, now, tx| s.poll(now, tx)),
        on_key: Some(ui::on_key),
        scenes: SCENES,
        ..AppSpec::new(
            "Mdow",
            rune_mac::saved_size(windows::MAIN).unwrap_or((1180.0, 800.0)),
            init,
            ui::view,
        )
    }
}

type Scene = (&'static str, fn(&mut App));

const SCENES: &[Scene] = &[
    ("dark", |s| s.prefs = prefs::update(|p| p.appearance = prefs::Appearance::Dark)),
    ("light", |s| s.prefs = prefs::update(|p| p.appearance = prefs::Appearance::Light)),
    ("quick-open", |s| {
        let mut q = rune::TextEdit::new("");
        q.focused = true;
        s.quick_open = Some(app::QuickOpen { query: q, selected: 0 });
    }),
    ("settings", |s| s.sheet = Some(app::Sheet::Settings)),
    ("shortcuts", |s| s.sheet = Some(app::Sheet::Shortcuts)),
    ("files", |s| {
        s.prefs = prefs::update(|p| {
            p.sidebar_open = true;
            p.sidebar_mode = prefs::SidebarMode::Files;
        });
        if let Some(dir) = s.tab().pane().path().and_then(|p| p.parent()).map(|p| p.to_path_buf()) {
            s.set_folder(&dir);
        }
    }),
    ("recents", |s| {
        s.prefs = prefs::update(|p| {
            p.sidebar_open = true;
            p.sidebar_mode = prefs::SidebarMode::Recents;
        })
    }),
    ("no-sidebar", |s| s.prefs = prefs::update(|p| p.sidebar_open = false)),
    ("split", |s| {
        let tab = s.tab_mut();
        let path = tab.pane().path().map(|p| p.to_path_buf());
        if let Some(path) = path {
            s.open_split_of(&path);
        }
    }),
    ("companion", |s| s.companion.open = true),
    ("companion-demo", |s| s.companion.demo()),
];

/// The Mac's window buttons sit in the header. Linux keeps the frame its desktop draws.
const CHROME: Chrome = Chrome {
    full_size_content: true,
    traffic_lights: if platform::MAC { Some((16.0, 13.0)) } else { None },
    drag_regions: true,
};

thread_local! {
    static PANELS: std::cell::RefCell<std::collections::HashMap<&'static str, rune::WindowHandle>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    static NEXT_PANEL: std::cell::Cell<Option<app::Sheet>> = const { std::cell::Cell::new(None) };
}

fn init_panel() -> App {
    let kind = NEXT_PANEL.with(|n| n.take()).unwrap_or(app::Sheet::Settings);
    let mut app = App::new(panel_key(kind).to_string(), Vec::new());
    app.panel = Some(kind);
    app
}

fn panel_key(kind: app::Sheet) -> &'static str {
    match kind {
        app::Sheet::Settings => "settings",
        app::Sheet::Shortcuts => "shortcuts",
    }
}

/// Shows the Settings or Keyboard Shortcuts window, making it the first time.
pub fn open_panel(kind: app::Sheet) {
    let key = panel_key(kind);
    if let Some(handle) = PANELS.with_borrow(|p| p.get(key).cloned()) {
        handle.open();
        return;
    }
    NEXT_PANEL.with(|n| n.set(Some(kind)));
    let size = match kind {
        app::Sheet::Settings => (560.0, 680.0),
        app::Sheet::Shortcuts => (480.0, 640.0),
    };
    let mut spec = spec(init_panel);
    spec.size = size;
    spec.min_size = Some((420.0, 360.0));
    // Closing hides it; it comes back as it was.
    let app = rune::App::new(spec).keep_running();
    let spec = rune::WindowSpec::from_app(key, app)
        .chrome(CHROME)
        .titlebar(Titlebar::new(theme::HEADER_HEIGHT));
    let handle = rune::open_window(spec);
    PANELS.with_borrow_mut(|p| p.insert(key, handle));
}

/// Opens another window (⌘N), with `paths` in it.
pub fn open_new_window(paths: Vec<PathBuf>) {
    let key = windows::new_key();
    windows::push_pending(key.clone(), paths);
    // Closing it ends it for good: commands stop going to it.
    let attach_key = key.clone();
    let app = rune::App::new(spec(init_window))
        .on_window(move |w| webview::attach(&attach_key, w))
        .on_close(|s: &mut App, _tx| {
            s.companion.shutdown();
            crate::windows::unregister(&s.key);
        });
    let spec = rune::WindowSpec::from_app(key, app)
        .chrome(CHROME)
        .titlebar(Titlebar::new(theme::HEADER_HEIGHT));
    let _ = rune::open_window(spec);
}

fn main() {
    platform::skip_absent_gpu_drivers();
    rune_mac::launch();
    rune::prewarm_gpu();
    platform::install_open_handler();
    omarchy::start();
    rune::fast_exit(companion::stop_server);
    let app = rune::App::new(spec(init_main))
        .on_window(|w| webview::attach(windows::MAIN, w))
        .on_frame_drawn(|_rt| {
            perf::frame_drawn();
            perf::bench_frame();
        })
        .chrome(CHROME)
        .titlebar(Titlebar::new(theme::HEADER_HEIGHT));
    let app = if platform::MAC {
        // A Mac app outlives its windows: this one hides, and the next (from the Dock) starts
        // empty.
        app.keep_running().on_close(|s: &mut App, _tx| {
            s.save_session();
            s.reset();
        })
    } else {
        // Elsewhere closing the window ends the app.
        app.on_close(|s: &mut App, _tx| {
            s.save_session();
            s.companion.shutdown();
        })
    };
    let app = menu::install(app);
    app.run();
}

#[cfg(test)]
mod tests;
