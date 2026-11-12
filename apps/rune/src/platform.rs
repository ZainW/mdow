//! What Rune leaves to the app on each system: the light/dark override, hand-offs to the file
//! manager and the editor, documents opened from outside, and how shortcuts are written.
//!
//! On macOS that is AppKit: Finder, `open`, the menu bar's submenus and the recent documents
//! list. On Linux it is the desktop's own tools (`xdg-open`, the file manager's D-Bus name, and
//! Omarchy's helpers where they exist), and Ctrl where the Mac has ⌘.

use crate::prefs::Appearance;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const MAC: bool = cfg!(target_os = "macos");

/// Runs a helper and forgets it: nothing to read back, and a failure leaves things as they were.
fn run(program: &str, args: &[&std::ffi::OsStr]) -> bool {
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|mut child| {
            // Reap it, so helpers that exit at once don't linger as zombies.
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        })
        .is_ok()
}

/// A `file://` URI for a path, with everything outside the unreserved set percent-encoded, so
/// spaces, `#`, `%` and commas (which `dbus-send` splits lists on) survive.
fn file_uri(path: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    let mut uri = String::from("file://");
    for &byte in path.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.' | b'~') {
            uri.push(byte as char);
        } else {
            uri.push_str(&format!("%{byte:02X}"));
        }
    }
    uri
}

/// The desktop entry that opens a type by default, like `org.gnome.TextEditor.desktop`.
fn default_app(mime: &str) -> Option<String> {
    let output = Command::new("xdg-mime")
        .args(["query", "default", mime])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!name.is_empty()).then_some(name)
}

/// Shows the file selected in Finder, or in the desktop's file manager.
pub fn reveal_in_files(path: &Path) {
    if MAC {
        run("/usr/bin/open", &["-R".as_ref(), path.as_os_str()]);
        return;
    }
    let path = path.to_path_buf();
    // Off the window's thread: the file manager may have to start before it answers.
    std::thread::spawn(move || {
        // File managers answer on this name and select the file; without one, open its folder.
        let shown = Command::new("dbus-send")
            .args([
                "--session",
                "--print-reply",
                "--reply-timeout=5000",
                "--dest=org.freedesktop.FileManager1",
                "/org/freedesktop/FileManager1",
                "org.freedesktop.FileManager1.ShowItems",
                &format!("array:string:{}", file_uri(&path)),
                "string:",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        if !shown && let Some(dir) = path.parent() {
            run("xdg-open", &[dir.as_os_str()]);
        }
    });
}

/// Opens the file in the default text editor.
pub fn open_in_editor(path: &Path) {
    if MAC {
        run("/usr/bin/open", &["-t".as_ref(), path.as_os_str()]);
        return;
    }
    // Omarchy knows the editor.
    if run("omarchy-launch-editor", &[path.as_os_str()]) {
        return;
    }
    let path = path.to_path_buf();
    std::thread::spawn(move || {
        let ours = |app: &str| app.starts_with("com.zain.mdow");
        // Elsewhere, the app for plain text is the editor. The app for markdown may well be
        // Mdow itself, and opening the file there again isn't editing it.
        let editor = default_app("text/plain").filter(|app| !ours(app));
        if let Some(app) = editor
            && run("gtk-launch", &[app.trim_end_matches(".desktop").as_ref(), path.as_os_str()])
        {
            return;
        }
        if !default_app("text/markdown").is_some_and(|app| ours(&app)) {
            run("xdg-open", &[path.as_os_str()]);
        }
    });
}

/// Opens a linked file in its app, but only shows apps, folders, scripts and other executables in
/// Finder: a link in a document should never run something.
pub fn open_with_default_app(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    const RUNNABLE: [&str; 12] = [
        "app", "command", "sh", "zsh", "bash", "tool", "pkg", "mpkg", "terminal", "scpt",
        "workflow", "action",
    ];
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    let executable =
        std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0);
    if path.is_dir() || RUNNABLE.contains(&ext.as_str()) || executable {
        reveal_in_files(path);
        return;
    }
    run(if MAC { "/usr/bin/open" } else { "xdg-open" }, &[path.as_os_str()]);
}

/// Keeps Vulkan from loading NVIDIA's driver on a machine with no NVIDIA GPU. Call first in
/// `main`, before anything starts a thread.
///
/// The Vulkan loader opens every installed driver to ask what it can drive. NVIDIA's takes
/// about half a second to answer "nothing" where its packages are installed without one of its
/// GPUs (an eGPU that isn't plugged in, a card that was swapped out), and that half second is
/// spent before the window's first frame. A driver for hardware that isn't there can't be the
/// one a window draws with, so leaving it out changes nothing but the wait.
pub fn skip_absent_gpu_drivers() {
    const NVIDIA: &str = "0x10de";
    if !cfg!(target_os = "linux") {
        return;
    }
    // Someone who chose their drivers knows better.
    let chosen = ["VK_LOADER_DRIVERS_DISABLE", "VK_LOADER_DRIVERS_SELECT", "VK_ICD_FILENAMES"]
        .iter()
        .chain(&["VK_DRIVER_FILES"])
        .any(|var| std::env::var_os(var).is_some());
    if chosen {
        return;
    }
    let Ok(devices) = std::fs::read_dir("/sys/bus/pci/devices") else { return };
    let mut gpus = 0;
    for device in devices.flatten() {
        let read = |name: &str| std::fs::read_to_string(device.path().join(name));
        // Class 0x03: display controllers.
        if !read("class").is_ok_and(|class| class.trim().starts_with("0x03")) {
            continue;
        }
        gpus += 1;
        if read("vendor").is_ok_and(|vendor| vendor.trim() == NVIDIA) {
            return;
        }
    }
    // With no GPU listed at all (a container, an ARM board), this is the wrong place to look.
    if gpus > 0 {
        // SAFETY: called before any thread exists, so nothing reads the environment meanwhile.
        unsafe { std::env::set_var("VK_LOADER_DRIVERS_DISABLE", "*nvidia*") };
    }
}

/// Where settings are kept: Application Support on macOS, the XDG config folder elsewhere.
pub fn settings_file() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    if MAC {
        return home.join("Library/Application Support/Mdow Rune/settings.json");
    }
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute())
        .unwrap_or_else(|| home.join(".config"))
        .join("mdow/settings.json")
}

/// A label as this system writes it. The app's labels are the Mac's: off the Mac, shortcuts
/// are spelled out with Ctrl for ⌘ ("⇧⌘O" becomes "Ctrl+Shift+O"), the few that need both ⌘
/// and ⌃ take the Linux app's keys, and Finder is the file manager.
pub fn label(mac: &'static str) -> &'static str {
    if MAC {
        return mac;
    }
    use std::cell::RefCell;
    use std::collections::HashMap;
    // Labels are only made on the window's thread, a fixed set of them, each translated once.
    thread_local! {
        static SEEN: RefCell<HashMap<&'static str, &'static str>> = RefCell::new(HashMap::new());
    }
    SEEN.with_borrow_mut(|seen| {
        *seen.entry(mac).or_insert_with(|| Box::leak(translate(mac).into_boxed_str()))
    })
}

fn translate(mac: &str) -> String {
    let mut out = String::with_capacity(mac.len() + 8);
    let mut chars = mac.chars().peekable();
    while let Some(c) = chars.next() {
        if !matches!(c, '⌘' | '⌃' | '⌥' | '⇧') {
            out.push(c);
            continue;
        }
        let (mut cmd, mut ctrl, mut alt, mut shift) = (false, false, false, false);
        let mut next = Some(c);
        while let Some(m) = next {
            match m {
                '⌘' => cmd = true,
                '⌃' => ctrl = true,
                '⌥' => alt = true,
                '⇧' => shift = true,
                _ => break,
            }
            next = chars.next_if(|n| matches!(n, '⌘' | '⌃' | '⌥' | '⇧'));
        }
        let key = chars.next();
        // ⌃⌘ has no counterpart where ⌘ is Ctrl: these are the Linux app's keys for them.
        if cmd && ctrl {
            match key {
                Some('S') => out.push_str("Ctrl+B"),
                Some(k) => {
                    out.push_str("Ctrl+Alt+");
                    out.push(k);
                }
                None => {}
            }
            continue;
        }
        if cmd || ctrl {
            out.push_str("Ctrl+");
        }
        if alt {
            out.push_str("Alt+");
        }
        if shift {
            out.push_str("Shift+");
        }
        match key {
            Some('↑') => out.push_str("Up"),
            Some('↓') => out.push_str("Down"),
            Some('↩') => out.push_str("Enter"),
            Some('⌫') => out.push_str("Backspace"),
            Some('⇥') => out.push_str("Tab"),
            Some(k) => out.push(k),
            None => {}
        }
    }
    out.replace("Finder", "Files")
}

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use objc2::MainThreadMarker;
    use objc2::runtime::{AnyClass, AnyObject, Imp, Sel};
    use objc2::sel;
    use objc2_app_kit::{
        NSAppearance, NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSApplication,
    };
    use objc2_foundation::{NSArray, NSURL};
    use std::sync::Mutex;

    static APPLIED: Mutex<Option<Appearance>> = Mutex::new(None);

    /// Sets the app's appearance, which Rune then reports to every window as its theme.
    pub fn set_appearance(appearance: Appearance) {
        if rune::is_headless() {
            return;
        }
        let mut applied = APPLIED.lock().unwrap();
        if *applied == Some(appearance) {
            return;
        }
        let Some(mtm) = MainThreadMarker::new() else { return };
        *applied = Some(appearance);
        let app = NSApplication::sharedApplication(mtm);
        let named = match appearance {
            Appearance::System => None,
            Appearance::Light => NSAppearance::appearanceNamed(unsafe { NSAppearanceNameAqua }),
            Appearance::Dark => NSAppearance::appearanceNamed(unsafe { NSAppearanceNameDarkAqua }),
        };
        app.setAppearance(named.as_deref());
    }

    unsafe extern "C-unwind" fn open_urls(
        _this: *mut AnyObject,
        _cmd: Sel,
        _app: *mut AnyObject,
        urls: *mut NSArray<NSURL>,
    ) {
        let Some(urls) = (unsafe { urls.as_ref() }) else { return };
        let paths: Vec<PathBuf> =
            urls.iter().filter_map(|u| u.path().map(|p| PathBuf::from(p.to_string()))).collect();
        if !paths.is_empty() {
            if crate::windows::target() == crate::windows::MAIN {
                crate::menu::show_main_if_hidden();
            }
            crate::windows::send_to_front(crate::app::Cmd::Open(paths));
        }
    }

    /// AppKit hands files opened from Finder, the Dock or `open` to its delegate's
    /// `application:openURLs:`. winit's delegate has none, so this gives every object one
    /// (only the app's delegate is ever asked), before the event loop starts.
    pub fn install_open_handler() {
        let Some(cls) = AnyClass::get(c"NSObject") else { return };
        type OpenUrls =
            unsafe extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject, *mut NSArray<NSURL>);
        let imp: Imp = unsafe { std::mem::transmute::<OpenUrls, Imp>(open_urls as OpenUrls) };
        unsafe {
            objc2::ffi::class_addMethod(
                cls as *const AnyClass as *mut AnyClass,
                sel!(application:openURLs:),
                imp,
                c"v@:@@".as_ptr(),
            );
        }
    }
}

#[cfg(target_os = "macos")]
pub use imp::{install_open_handler, set_appearance};

#[cfg(not(target_os = "macos"))]
pub fn set_appearance(_: Appearance) {}
#[cfg(not(target_os = "macos"))]
pub fn install_open_handler() {}

/// Reads `text` aloud with the system voice, as Edit › Speech does; kill the child to stop.
/// macOS only: there's no voice every Linux desktop has.
pub fn speak(text: &str) -> Option<std::process::Child> {
    use std::io::Write;
    if !MAC {
        return None;
    }
    let mut child =
        Command::new("/usr/bin/say").stdin(std::process::Stdio::piped()).spawn().ok()?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(text.as_bytes());
    }
    Some(child)
}

/// What a native submenu item does: an index into [`menu_command`].
#[cfg(target_os = "macos")]
mod menus {
    use crate::app::Cmd;
    use crate::prefs::ReadingWidth;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, NSObject, Sel};
    use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
    use objc2_app_kit::{NSApplication, NSEventModifierFlags, NSMenu, NSMenuItem};
    use objc2_foundation::{NSObjectProtocol, NSString};
    use std::cell::RefCell;
    use std::path::PathBuf;

    const RECENT: isize = 1000;
    const CLEAR_RECENTS: isize = 999;

    fn command(tag: isize) -> Option<Cmd> {
        Some(match tag {
            t if (RECENT..RECENT + 100).contains(&t) => Cmd::OpenRecent((t - RECENT) as usize),
            CLEAR_RECENTS => Cmd::ClearRecents,
            1 => Cmd::Find,
            2 => Cmd::FindNext,
            3 => Cmd::FindPrevious,
            4 => Cmd::UseSelectionForFind,
            5 => Cmd::JumpToSelection,
            10 => Cmd::StartSpeaking,
            11 => Cmd::StopSpeaking,
            20 => Cmd::ReadingWidth(ReadingWidth::Narrow),
            21 => Cmd::ReadingWidth(ReadingWidth::Standard),
            22 => Cmd::ReadingWidth(ReadingWidth::Wide),
            23 => Cmd::ReadingWidth(ReadingWidth::Full),
            _ => return None,
        })
    }

    define_class!(
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "MdowMenuTarget"]
        struct Target;

        unsafe impl NSObjectProtocol for Target {}

        impl Target {
            #[unsafe(method(pick:))]
            fn pick(&self, sender: &NSMenuItem) {
                if let Some(cmd) = command(sender.tag()) {
                    if crate::windows::target() == crate::windows::MAIN {
                        crate::menu::show_main_if_hidden();
                    }
                    crate::windows::send_to_front(cmd);
                }
            }
        }
    );

    thread_local! {
        static TARGET: RefCell<Option<Retained<Target>>> = const { RefCell::new(None) };
        static RECENTS: RefCell<Option<Retained<NSMenu>>> = const { RefCell::new(None) };
        static WIDTHS: RefCell<Vec<Retained<NSMenuItem>>> = const { RefCell::new(Vec::new()) };
    }

    fn item(
        mtm: MainThreadMarker,
        title: &str,
        tag: isize,
        key: &str,
        mods: NSEventModifierFlags,
    ) -> Retained<NSMenuItem> {
        let target = TARGET.with_borrow(|t| t.clone());
        unsafe {
            let item = NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &NSString::from_str(title),
                Some(sel!(pick:)),
                &NSString::from_str(key),
            );
            item.setKeyEquivalentModifierMask(mods);
            item.setTag(tag);
            if let Some(t) = &target {
                let obj: &AnyObject = t;
                item.setTarget(Some(obj));
            }
            item
        }
    }

    fn submenu(
        mtm: MainThreadMarker,
        title: &str,
        items: Vec<Retained<NSMenuItem>>,
    ) -> (Retained<NSMenuItem>, Retained<NSMenu>) {
        let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str(title));
        for i in &items {
            menu.addItem(i);
        }
        let holder = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &NSString::from_str(title),
                None,
                &NSString::from_str(""),
            )
        };
        holder.setSubmenu(Some(&menu));
        (holder, menu)
    }

    fn top(app: &NSApplication, title: &str) -> Option<Retained<NSMenu>> {
        let root = app.mainMenu()?;
        (0..root.numberOfItems())
            .filter_map(|i| root.itemAtIndex(i))
            .find(|i| i.title().to_string() == title)?
            .submenu()
    }

    fn index_of(menu: &NSMenu, title: &str) -> Option<isize> {
        (0..menu.numberOfItems())
            .find(|i| menu.itemAtIndex(*i).is_some_and(|it| it.title().to_string() == title))
    }

    /// Adds the submenus Rune's menus can't hold: File › Open Recent, Edit › Find and Speech,
    /// View › Text Width. Runs once, after Rune has made its menus.
    pub fn install() -> bool {
        let Some(mtm) = MainThreadMarker::new() else { return false };
        if TARGET.with_borrow(|t| t.is_some()) {
            return true;
        }
        let app = NSApplication::sharedApplication(mtm);
        let (Some(file), Some(edit), Some(view)) =
            (top(&app, "File"), top(&app, "Edit"), top(&app, "View"))
        else {
            return false;
        };
        let target: Retained<Target> = unsafe { msg_send![Target::alloc(mtm), init] };
        TARGET.with_borrow_mut(|t| *t = Some(target));
        let cmd = NSEventModifierFlags::Command;
        let none = NSEventModifierFlags::empty();

        let (recent, recent_menu) = submenu(mtm, "Open Recent", Vec::new());
        let at = index_of(&file, "Open Folder…").map_or(file.numberOfItems(), |i| i + 1);
        file.insertItem_atIndex(&recent, at);
        RECENTS.with_borrow_mut(|r| *r = Some(recent_menu));

        let (find, _) = submenu(
            mtm,
            "Find",
            vec![
                item(mtm, "Find…", 1, "f", cmd),
                item(mtm, "Find Next", 2, "g", cmd),
                item(mtm, "Find Previous", 3, "g", cmd | NSEventModifierFlags::Shift),
                item(mtm, "Use Selection for Find", 4, "e", cmd),
                item(mtm, "Jump to Selection", 5, "j", cmd),
            ],
        );
        let (speech, _) = submenu(
            mtm,
            "Speech",
            vec![
                item(mtm, "Start Speaking", 10, "", none),
                item(mtm, "Stop Speaking", 11, "", none),
            ],
        );
        edit.addItem(&NSMenuItem::separatorItem(mtm));
        edit.addItem(&find);
        edit.addItem(&speech);

        let widths = vec![
            item(mtm, "Narrow", 20, "", none),
            item(mtm, "Standard", 21, "", none),
            item(mtm, "Wide", 22, "", none),
            item(mtm, "Full Width", 23, "", none),
        ];
        WIDTHS.with_borrow_mut(|w| *w = widths.clone());
        let (width, _) = submenu(mtm, "Text Width", widths);
        let at = index_of(&view, "Zoom Out").map_or(view.numberOfItems(), |i| i + 1);
        view.insertItem_atIndex(&width, at);
        let _: Option<Sel> = None;
        true
    }

    /// Lists `recents` in File › Open Recent and ticks the text width in use.
    pub fn refresh(recents: &[PathBuf], width: ReadingWidth) {
        let Some(mtm) = MainThreadMarker::new() else { return };
        RECENTS.with_borrow(|menu| {
            let Some(menu) = menu else { return };
            menu.removeAllItems();
            for (i, path) in recents.iter().take(20).enumerate() {
                let it = item(
                    mtm,
                    &crate::files::display_name(path),
                    RECENT + i as isize,
                    "",
                    NSEventModifierFlags::empty(),
                );
                it.setToolTip(Some(&NSString::from_str(&path.display().to_string())));
                menu.addItem(&it);
            }
            if !recents.is_empty() {
                menu.addItem(&NSMenuItem::separatorItem(mtm));
            }
            let clear = item(mtm, "Clear Menu", CLEAR_RECENTS, "", NSEventModifierFlags::empty());
            clear.setEnabled(!recents.is_empty());
            menu.addItem(&clear);
        });
        let on = ReadingWidth::ALL.iter().position(|w| *w == width);
        WIDTHS.with_borrow(|items| {
            for (i, it) in items.iter().enumerate() {
                it.setState(if Some(i) == on { 1 } else { 0 });
            }
        });
    }

    /// Adds `path` to the system's recent documents (the Dock menu and Recent Items).
    pub fn note_recent(path: &std::path::Path) {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let controller = objc2_app_kit::NSDocumentController::sharedDocumentController(mtm);
        let url =
            objc2_foundation::NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
        controller.noteNewRecentDocumentURL(&url);
    }
}

#[cfg(target_os = "macos")]
pub use menus::{install as install_submenus, note_recent, refresh as refresh_submenus};

// Elsewhere there's no menu bar or recent documents list; headless runs and tests go without.
#[cfg(not(target_os = "macos"))]
pub fn install_submenus() -> bool {
    false
}
#[cfg(not(target_os = "macos"))]
pub fn refresh_submenus(_: &[std::path::PathBuf], _: crate::prefs::ReadingWidth) {}
#[cfg(not(target_os = "macos"))]
pub fn note_recent(_: &Path) {}

/// Whether macOS keeps windows when an app quits: System Settings › Desktop & Dock › "Close
/// windows when quitting an application", off.
pub fn keeps_windows() -> bool {
    #[cfg(target_os = "macos")]
    {
        let defaults = objc2_foundation::NSUserDefaults::standardUserDefaults();
        defaults.boolForKey(&objc2_foundation::NSString::from_str("NSQuitAlwaysKeepsWindows"))
    }
    // Linux desktops have no such setting; the last session's tabs come back, as in the
    // Linux app before this one.
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcuts_are_spelled_out_off_the_mac() {
        assert_eq!(translate("⌘O"), "Ctrl+O");
        assert_eq!(translate("⇧⌘O"), "Ctrl+Shift+O");
        assert_eq!(translate("⌥⌘↑"), "Ctrl+Alt+Up");
        assert_eq!(translate("⌃⇥"), "Ctrl+Tab");
        assert_eq!(translate("⌘1–⌘9"), "Ctrl+1–Ctrl+9");
        assert_eq!(translate("Back (⌘[)"), "Back (Ctrl+[)");
        assert_eq!(translate("Show in Finder"), "Show in Files");
        assert_eq!(translate(""), "");
    }

    #[test]
    fn file_uris_escape_what_would_break_them() {
        assert_eq!(file_uri(Path::new("/home/u/a.md")), "file:///home/u/a.md");
        assert_eq!(
            file_uri(Path::new("/n/a, b #1 100%.md")),
            "file:///n/a%2C%20b%20%231%20100%25.md"
        );
        assert_eq!(file_uri(Path::new("/n/é.md")), "file:///n/%C3%A9.md");
    }

    #[test]
    fn chords_with_command_and_control_take_the_linux_keys() {
        assert_eq!(translate("⌃⌘S"), "Ctrl+B");
        assert_eq!(translate("⌃⌘2"), "Ctrl+Alt+2");
    }
}
