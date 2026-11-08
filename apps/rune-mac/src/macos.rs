//! The bits of macOS Rune leaves to the app: the light/dark override, Finder and editor
//! hand-offs, and documents opened from Finder, the Dock or `open`.

use crate::prefs::Appearance;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn reveal_in_finder(path: &Path) {
    let _ = Command::new("/usr/bin/open").arg("-R").arg(path).spawn();
}

/// Opens the file in the default text editor.
pub fn open_in_editor(path: &Path) {
    let _ = Command::new("/usr/bin/open").arg("-t").arg(path).spawn();
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
        reveal_in_finder(path);
        return;
    }
    let _ = Command::new("/usr/bin/open").arg(path).spawn();
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
pub fn speak(text: &str) -> Option<std::process::Child> {
    use std::io::Write;
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

/// Whether macOS keeps windows when an app quits: System Settings › Desktop & Dock › "Close
/// windows when quitting an application", off.
pub fn keeps_windows() -> bool {
    #[cfg(target_os = "macos")]
    {
        let defaults = objc2_foundation::NSUserDefaults::standardUserDefaults();
        defaults.boolForKey(&objc2_foundation::NSString::from_str("NSQuitAlwaysKeepsWindows"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}
