//! The menu bar, laid out as the Swift app's: File, Edit (with Find), View, Go, Window and
//! Help, plus Settings in the app menu. Submenus Rune's menus can't hold (Open Recent, Find,
//! Speech, Text Width) are added in AppKit by `platform::install_submenus`. Rune adds the
//! standard Edit and Window items (Copy, Select All, Minimize).
//!
//! Each item queues its command for the window in front (see `windows`).

use crate::app::{App, Cmd};
use crate::prefs::SidebarMode;
use crate::windows;
use rune::{Key, KeyEvent, MenuItem, Shortcut, Tx};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

static INSTALLED: AtomicBool = AtomicBool::new(false);
static HANDLE: OnceLock<rune::AppHandle<App>> = OnceLock::new();

pub fn installed() -> bool {
    INSTALLED.load(Ordering::Relaxed)
}

/// Brings back the main window when a command or an opened file lands in it while it's closed
/// (closing hides it; the app keeps running, as Mac document apps do).
pub fn show_main_if_hidden() {
    if let Some(h) = HANDLE.get()
        && !h.is_visible()
    {
        h.open();
    }
}

fn route(s: &mut App, tx: &mut Tx, cmd: Cmd) {
    let target = windows::target();
    if target == windows::MAIN && !matches!(cmd, Cmd::NewWindow | Cmd::Settings | Cmd::Shortcuts) {
        show_main_if_hidden();
    }
    if target == s.key || !windows::keys().contains(&target) {
        s.run(cmd, tx);
    } else {
        windows::send(&target, cmd);
    }
}

fn key(chord: &str) -> Shortcut {
    chord.parse().unwrap_or_else(|_| panic!("bad chord {chord}"))
}

fn item(
    menu: &str,
    title: &str,
    chord: Option<&str>,
    action: fn(&mut App, &mut Tx),
) -> MenuItem<App> {
    let mut i = MenuItem::new(title, action).menu(menu).id(format!("{menu}/{title}"));
    if let Some(c) = chord {
        i = i.key_equivalent(key(c));
    }
    i
}

macro_rules! tab {
    ($($n:literal),*) => {
        [$(|s: &mut App, tx: &mut Tx| route(s, tx, Cmd::SelectTab($n))),*]
    };
}

const TAB_SLOTS: [fn(&mut App, &mut Tx); 9] = tab!(0, 1, 2, 3, 4, 5, 6, 7, 8);

pub fn items() -> Vec<MenuItem<App>> {
    let mut v = vec![
        MenuItem::new("Settings…", |s, tx| route(s, tx, Cmd::Settings))
            .id("settings")
            .key_equivalent(key("cmd+comma")),
        // File
        item("File", "New Window", Some("cmd+n"), |s, tx| route(s, tx, Cmd::NewWindow)),
        item("File", "New Tab", Some("cmd+t"), |s, tx| route(s, tx, Cmd::NewTab)),
        item("File", "Open…", Some("cmd+o"), |s, tx| route(s, tx, Cmd::OpenDialog)),
        item("File", "Open Folder…", Some("cmd+shift+o"), |s, tx| {
            route(s, tx, Cmd::OpenFolderDialog)
        }),
        MenuItem::separator().menu("File"),
        item("File", "Close Tab", Some("cmd+w"), |s, tx| route(s, tx, Cmd::CloseTab)),
        item("File", "Close Window", Some("cmd+shift+w"), |s, tx| route(s, tx, Cmd::CloseWindow)),
        item("File", "Reload", Some("cmd+r"), |s, tx| route(s, tx, Cmd::Reload)),
        MenuItem::separator().menu("File"),
        item("File", "Show in Finder", Some("cmd+alt+r"), |s, tx| route(s, tx, Cmd::ShowInFinder)),
        item("File", "Open in Editor", Some("cmd+shift+e"), |s, tx| {
            route(s, tx, Cmd::OpenInEditor)
        }),
        item("File", "Copy Path", Some("cmd+alt+c"), |s, tx| route(s, tx, Cmd::CopyPath)),
        MenuItem::separator().menu("File"),
        item("File", "Export as PDF…", None, |s, tx| route(s, tx, Cmd::ExportPdf)),
        item("File", "Page Setup…", Some("cmd+shift+p"), |s, tx| route(s, tx, Cmd::PageSetup)),
        item("File", "Print…", Some("cmd+p"), |s, tx| route(s, tx, Cmd::Print)),
        // Edit: Rune adds Undo, Cut, Copy, Paste and Select All; `platform::install_submenus`
        // adds Find and Speech.
        // View
        item("View", "Show Sidebar", Some("cmd+ctrl+s"), |s, tx| route(s, tx, Cmd::ToggleSidebar)),
        item("View", "Outline", Some("cmd+ctrl+1"), |s, tx| {
            route(s, tx, Cmd::Sidebar(SidebarMode::Outline))
        }),
        item("View", "Files", Some("cmd+ctrl+2"), |s, tx| {
            route(s, tx, Cmd::Sidebar(SidebarMode::Files))
        }),
        item("View", "Recents", Some("cmd+ctrl+3"), |s, tx| {
            route(s, tx, Cmd::Sidebar(SidebarMode::Recents))
        }),
        item("View", "Show Companion", Some("cmd+alt+0"), |s, tx| {
            route(s, tx, Cmd::ToggleCompanion)
        }),
        MenuItem::separator().menu("View"),
        item("View", "Split View", Some("cmd+backslash"), |s, tx| route(s, tx, Cmd::ToggleSplit)),
        MenuItem::separator().menu("View"),
        item("View", "Actual Size", Some("cmd+0"), |s, tx| route(s, tx, Cmd::ActualSize)),
        item("View", "Zoom In", Some("cmd+plus"), |s, tx| route(s, tx, Cmd::ZoomIn)),
        item("View", "Zoom Out", Some("cmd+minus"), |s, tx| route(s, tx, Cmd::ZoomOut)),
        MenuItem::separator().menu("View"),
        item("View", "Toggle Light and Dark", None, |s, tx| route(s, tx, Cmd::ToggleAppearance)),
        item("View", "Enter Full Screen", Some("cmd+ctrl+f"), |s, tx| {
            route(s, tx, Cmd::FullScreen)
        }),
        // Go
        item("Go", "Back", Some("cmd+["), |s, tx| route(s, tx, Cmd::Back)),
        item("Go", "Forward", Some("cmd+]"), |s, tx| route(s, tx, Cmd::Forward)),
        MenuItem::separator().menu("Go"),
        item("Go", "Previous Heading", Some("cmd+alt+up"), |s, tx| {
            route(s, tx, Cmd::PreviousHeading)
        }),
        item("Go", "Next Heading", Some("cmd+alt+down"), |s, tx| route(s, tx, Cmd::NextHeading)),
        MenuItem::separator().menu("Go"),
        item("Go", "Quick Open…", Some("cmd+k"), |s, tx| route(s, tx, Cmd::QuickOpen)),
        // Window
        MenuItem::separator().menu("Window"),
        item("Window", "Show Previous Tab", Some("cmd+shift+["), |s, tx| {
            route(s, tx, Cmd::PreviousTab)
        }),
        item("Window", "Show Next Tab", Some("cmd+shift+]"), |s, tx| route(s, tx, Cmd::NextTab)),
        item("Window", "Next Tab", Some("ctrl+tab"), |s, tx| route(s, tx, Cmd::NextTab)),
        item("Window", "Previous Tab", Some("ctrl+shift+tab"), |s, tx| {
            route(s, tx, Cmd::PreviousTab)
        }),
        // Help
        item("Help", "Keyboard Shortcuts", Some("cmd+slash"), |s, tx| route(s, tx, Cmd::Shortcuts)),
    ];
    for (i, action) in TAB_SLOTS.iter().enumerate() {
        let title = if i == 8 { "Last Tab".to_string() } else { format!("Tab {}", i + 1) };
        v.push(
            MenuItem::new(title.clone(), *action)
                .menu("Window")
                .id(format!("tab-{i}"))
                .key_equivalent(key(&format!("cmd+{}", i + 1)))
                .keys_first(),
        );
    }
    v
}

pub fn install(app: rune::App<App>) -> rune::App<App> {
    if rune::is_headless() || !cfg!(target_os = "macos") {
        return app;
    }
    let mut app = app.standard_menus();
    for item in items() {
        app = app.menu_item(item);
    }
    let _ = HANDLE.set(app.handle());
    INSTALLED.store(true, Ordering::Relaxed);
    app
}

/// The modifiers held besides Command. Off the Mac, Command is the Control key, so Rune
/// reports both for one press; there Control says nothing more.
pub fn chord_mods(ev: &KeyEvent) -> rune::Mods {
    let mut m = ev.mods;
    if !crate::platform::MAC && m.cmd {
        m.ctrl = false;
    }
    m
}

/// The menu's shortcuts where there's no native menu bar: on Linux, and in headless scripts
/// and tests. The chords are the menu's, with the Linux app's keys for the ones that need both
/// Command and Control on the Mac.
pub fn fallback_key(s: &mut App, ev: &KeyEvent, tx: &mut Tx) -> bool {
    let m = chord_mods(ev);
    let linux = !crate::platform::MAC;
    let cmd = match &ev.key {
        Key::Tab if ev.mods.ctrl => {
            if m.shift {
                Cmd::PreviousTab
            } else {
                Cmd::NextTab
            }
        }
        Key::Function(5) if linux => Cmd::Reload,
        Key::Up if m.cmd && m.alt => Cmd::PreviousHeading,
        Key::Down if m.cmd && m.alt => Cmd::NextHeading,
        Key::Left if linux && m.alt && !m.cmd => Cmd::Back,
        Key::Right if linux && m.alt && !m.cmd => Cmd::Forward,
        Key::Command(c) => match (*c, m.shift, m.alt, m.ctrl) {
            ('n', false, false, false) => Cmd::NewWindow,
            ('w', true, false, false) => Cmd::CloseWindow,
            ('e', true, false, false) => Cmd::OpenInEditor,
            ('e', false, false, false) => Cmd::UseSelectionForFind,
            ('r', false, true, false) => Cmd::ShowInFinder,
            ('c', false, true, false) => Cmd::CopyPath,
            ('b', false, false, false) if linux => Cmd::ToggleSidebar,
            ('q', false, false, false) if linux => Cmd::Quit,
            (d @ '1'..='3', false, true, false) if linux => Cmd::Sidebar(match d {
                '1' => SidebarMode::Outline,
                '2' => SidebarMode::Files,
                _ => SidebarMode::Recents,
            }),
            ('t', false, false, false) => Cmd::NewTab,
            ('w', false, false, false) => Cmd::CloseTab,
            ('o', false, false, false) => Cmd::OpenDialog,
            ('o', true, _, _) => Cmd::OpenFolderDialog,
            ('r', false, false, false) => Cmd::Reload,
            ('f', false, false, false) => Cmd::Find,
            ('g', false, false, false) => Cmd::FindNext,
            ('g', true, _, _) => Cmd::FindPrevious,
            ('k', false, false, false) => Cmd::QuickOpen,
            ('s', false, false, true) => Cmd::ToggleSidebar,
            ('\\', false, false, false) => Cmd::ToggleSplit,
            ('0', false, false, false) => Cmd::ActualSize,
            ('0', false, true, false) => Cmd::ToggleCompanion,
            ('=' | '+', _, false, false) => Cmd::ZoomIn,
            ('-', false, false, false) => Cmd::ZoomOut,
            ('[', false, false, false) => Cmd::Back,
            (']', false, false, false) => Cmd::Forward,
            ('[', true, _, _) | ('{', _, _, _) => Cmd::PreviousTab,
            (']', true, _, _) | ('}', _, _, _) => Cmd::NextTab,
            (',', false, false, false) => Cmd::Settings,
            ('/', false, false, false) => Cmd::Shortcuts,
            (d @ '1'..='9', false, false, false) => Cmd::SelectTab(d as usize - '1' as usize),
            (d @ '1'..='3', false, false, true) => Cmd::Sidebar(match d {
                '1' => SidebarMode::Outline,
                '2' => SidebarMode::Files,
                _ => SidebarMode::Recents,
            }),
            _ => return false,
        },
        _ => return false,
    };
    s.run(cmd, tx);
    true
}
