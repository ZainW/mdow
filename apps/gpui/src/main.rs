use anyhow::Context;
use gpui::{
    App, AppContext, Application, Bounds, KeyBinding, TitlebarOptions, WindowBounds, WindowHandle,
    WindowOptions, point, px, size,
};
use mdow_gpui::{
    actions::{
        About, BringAllToFront, CloseTab, Dismiss, FindNext, FindPrevious, Hide, HideOthers,
        Minimize, NewWindow, NextTab, OpenFile, OpenFolder, OpenWebsite, PreviousTab, Quit,
        SelectLastTab, SelectTab1, SelectTab2, SelectTab3, SelectTab4, SelectTab5, SelectTab6,
        SelectTab7, SelectTab8, ShowAll, SidebarFolder, SidebarOutline, SidebarRecents, ToggleFind,
        ToggleFullScreen, TogglePalette, ToggleSettings, ToggleShortcuts, ToggleSidebar,
        ToggleSplitView, ToggleWideMode, ZoomIn, ZoomOut, ZoomReset,
    },
    app::MdowApp,
    assets::{BUNDLED_FONTS, MdowAssets, discover_asset_root, validate_required_assets},
    menus::{WEBSITE_URL, app_menus},
    overlay,
    persist::{Restored, SessionRole, StateStore},
    theme::TrafficLights,
    ui::field,
};
use std::{borrow::Cow, ffi::OsString, path::PathBuf};

enum WindowSeed {
    RestoreSession,
    Blank,
    Smoke,
    RestoreSessionThenOpen(Option<PathBuf>),
}

struct LaunchArgs {
    verify_assets: bool,
    smoke_test: bool,
    document_path: Option<PathBuf>,
}

fn launch_args<T>(args: impl IntoIterator<Item = T>) -> LaunchArgs
where
    T: Into<OsString>,
{
    let mut verify_assets = false;
    let mut smoke_test = false;
    let mut document_path = None;

    for argument in args.into_iter().skip(1).map(Into::into) {
        if argument == "--verify-assets" {
            verify_assets = true;
        } else if argument == "--smoke-test" {
            smoke_test = true;
        } else if document_path.is_none() && !argument.to_string_lossy().starts_with('-') {
            document_path = Some(PathBuf::from(argument));
        }
    }

    LaunchArgs {
        verify_assets,
        smoke_test,
        document_path,
    }
}

fn default_window_title() -> &'static str {
    mdow_gpui::app::DEFAULT_WINDOW_TITLE
}

fn key_bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("cmd-n", NewWindow, None),
        KeyBinding::new("cmd-o", OpenFile, None),
        KeyBinding::new("cmd-shift-o", OpenFolder, None),
        KeyBinding::new("cmd-b", ToggleSidebar, None),
        KeyBinding::new("cmd-w", CloseTab, None),
        KeyBinding::new("cmd-shift-w", ToggleWideMode, None),
        KeyBinding::new("cmd-\\", ToggleSplitView, None),
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-f", ToggleFind, None),
        KeyBinding::new("cmd-k", TogglePalette, None),
        KeyBinding::new("cmd-shift-p", TogglePalette, None),
        KeyBinding::new("cmd-,", ToggleSettings, None),
        KeyBinding::new("cmd-/", ToggleShortcuts, None),
        KeyBinding::new("escape", Dismiss, None),
        KeyBinding::new("cmd-g", FindNext, None),
        KeyBinding::new("cmd-shift-g", FindPrevious, None),
        KeyBinding::new("cmd-=", ZoomIn, None),
        KeyBinding::new("cmd--", ZoomOut, None),
        KeyBinding::new("cmd-0", ZoomReset, None),
        KeyBinding::new("ctrl-1", SidebarRecents, None),
        KeyBinding::new("ctrl-2", SidebarFolder, None),
        KeyBinding::new("ctrl-3", SidebarOutline, None),
        KeyBinding::new("cmd-alt-right", NextTab, None),
        KeyBinding::new("cmd-alt-left", PreviousTab, None),
        KeyBinding::new("ctrl-tab", NextTab, None),
        KeyBinding::new("ctrl-shift-tab", PreviousTab, None),
        KeyBinding::new("cmd-1", SelectTab1, None),
        KeyBinding::new("cmd-2", SelectTab2, None),
        KeyBinding::new("cmd-3", SelectTab3, None),
        KeyBinding::new("cmd-4", SelectTab4, None),
        KeyBinding::new("cmd-5", SelectTab5, None),
        KeyBinding::new("cmd-6", SelectTab6, None),
        KeyBinding::new("cmd-7", SelectTab7, None),
        KeyBinding::new("cmd-8", SelectTab8, None),
        KeyBinding::new("cmd-9", SelectLastTab, None),
        KeyBinding::new("cmd-h", Hide, None),
        KeyBinding::new("alt-cmd-h", HideOthers, None),
        KeyBinding::new("cmd-m", Minimize, None),
        KeyBinding::new("ctrl-cmd-f", ToggleFullScreen, None),
        // Outside a Field, Copy and Select All reach the root and act on the reader's selection.
        KeyBinding::new("cmd-a", field::SelectAll, None),
        KeyBinding::new("cmd-c", field::Copy, None),
        KeyBinding::new("left", field::MoveLeft, Some("Field")),
        KeyBinding::new("right", field::MoveRight, Some("Field")),
        KeyBinding::new("shift-left", field::SelectLeft, Some("Field")),
        KeyBinding::new("shift-right", field::SelectRight, Some("Field")),
        KeyBinding::new("cmd-a", field::SelectAll, Some("Field")),
        KeyBinding::new("home", field::Home, Some("Field")),
        KeyBinding::new("end", field::End, Some("Field")),
        KeyBinding::new("backspace", field::Backspace, Some("Field")),
        KeyBinding::new("delete", field::Delete, Some("Field")),
        KeyBinding::new("cmd-v", field::Paste, Some("Field")),
        KeyBinding::new("cmd-c", field::Copy, Some("Field")),
        KeyBinding::new("cmd-x", field::Cut, Some("Field")),
        KeyBinding::new("enter", field::Submit, Some("Field")),
        KeyBinding::new("shift-enter", field::SubmitBackward, Some("Field")),
        KeyBinding::new("escape", field::Cancel, Some("Field")),
        KeyBinding::new("down", overlay::SelectNext, Some("Palette")),
        KeyBinding::new("up", overlay::SelectPrev, Some("Palette")),
    ]
}

fn main() -> anyhow::Result<()> {
    let launch_args = launch_args(std::env::args_os());
    let asset_root = discover_asset_root(
        std::env::current_exe().context("locating Mdow Native executable")?,
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"),
    )?;
    validate_required_assets(&asset_root)?;

    if launch_args.verify_assets {
        println!("{}", asset_root.display());
        return Ok(());
    }

    let fonts = BUNDLED_FONTS
        .iter()
        .map(|asset| {
            std::fs::read(asset_root.join(asset))
                .with_context(|| format!("reading required asset {asset}"))
                .map(Cow::Owned)
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let launch_path = launch_args.document_path;
    let application = Application::new().with_assets(MdowAssets::new(asset_root));
    let (open_sender, open_receiver) = async_channel::unbounded();
    application.on_open_urls(move |urls| {
        let _ = open_sender.try_send(urls);
    });
    application.on_reopen(|cx| {
        if cx.windows().is_empty() {
            open_main_window(WindowSeed::RestoreSession, cx);
        }
        cx.activate(true);
    });
    application.run(move |cx: &mut App| {
        cx.text_system()
            .add_fonts(fonts)
            .expect("register required Mdow fonts");

        cx.bind_keys(key_bindings());
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.on_action(|_: &NewWindow, cx| {
            open_main_window(WindowSeed::Blank, cx);
        });
        cx.on_action(|_: &Hide, cx| cx.hide());
        cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
        cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
        cx.on_action(|_: &About, _| native::show_about_panel());
        cx.on_action(|_: &BringAllToFront, cx| {
            native::arrange_in_front();
            cx.activate(true);
        });
        cx.on_action(|_: &OpenWebsite, _| {
            let _ = open::that(WEBSITE_URL);
        });
        cx.set_menus(app_menus(&[]));

        let seed = if launch_args.smoke_test {
            WindowSeed::Smoke
        } else {
            WindowSeed::RestoreSessionThenOpen(launch_path.clone())
        };
        mdow_gpui::perf::start(launch_path.clone(), cx);
        let primary = open_main_window(seed, cx);
        cx.spawn(async move |cx| {
            while let Ok(urls) = open_receiver.recv().await {
                let _ = cx.update(|cx| open_file_urls(urls, cx));
            }
        })
        .detach();
        if launch_args.smoke_test {
            cx.spawn(async move |cx| {
                gpui::Timer::after(std::time::Duration::from_secs(3)).await;
                primary
                    .update(cx, |_, _, _| ())
                    .expect("smoke window remains alive");
                println!("MDOW_SMOKE_OK");
                let _ = cx.update(|cx| cx.quit());
            })
            .detach();
        }
        cx.activate(true);
    });
    Ok(())
}

mod native {
    #[cfg(target_os = "macos")]
    unsafe extern "C" {
        fn mdow_show_about_panel();
        fn mdow_arrange_in_front();
    }

    pub fn show_about_panel() {
        #[cfg(target_os = "macos")]
        unsafe {
            mdow_show_about_panel()
        };
    }

    pub fn arrange_in_front() {
        #[cfg(target_os = "macos")]
        unsafe {
            mdow_arrange_in_front()
        };
    }
}

fn local_file_paths(urls: Vec<String>) -> Vec<PathBuf> {
    urls.into_iter()
        .filter_map(|value| url::Url::parse(&value).ok()?.to_file_path().ok())
        .collect()
}

fn open_file_urls(urls: Vec<String>, cx: &mut App) {
    let paths = local_file_paths(urls);
    if paths.is_empty() {
        return;
    }
    let window = cx
        .active_window()
        .and_then(|window| window.downcast::<MdowApp>())
        .or_else(|| {
            cx.windows()
                .into_iter()
                .find_map(|window| window.downcast::<MdowApp>())
        })
        .unwrap_or_else(|| open_main_window(WindowSeed::RestoreSession, cx));
    let _ = window.update(cx, |app, window, cx| {
        app.open_paths(paths, cx);
        window.activate_window();
    });
    cx.activate(true);
}

fn open_main_window(seed: WindowSeed, cx: &mut App) -> WindowHandle<MdowApp> {
    let store = if matches!(seed, WindowSeed::Smoke) {
        StateStore::in_memory()
    } else {
        StateStore::open_default()
    };
    let Restored { prefs, session } = store.load();
    let (restore, launch_path, role) = match seed {
        WindowSeed::RestoreSession => (true, None, SessionRole::Owner),
        WindowSeed::Smoke => (false, None, SessionRole::Transient),
        WindowSeed::Blank => (false, None, SessionRole::Transient),
        WindowSeed::RestoreSessionThenOpen(path) => (true, path, SessionRole::Owner),
    };
    let bounds = session
        .window
        .filter(|_| restore)
        .map(|saved| {
            Bounds::new(
                point(px(saved.x), px(saved.y)),
                size(px(saved.width), px(saved.height)),
            )
        })
        .filter(|bounds| bounds.size.width > px(200.0) && bounds.size.height > px(200.0))
        .unwrap_or_else(|| Bounds::centered(None, size(px(1120.0), px(760.0)), cx));
    cx.open_window(
        WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some(default_window_title().into()),
                appears_transparent: true,
                traffic_light_position: Some(TrafficLights::position()),
            }),
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        move |window, cx| {
            cx.new(|cx| {
                let mut app = MdowApp::boot(prefs, store, role, window, cx);
                if restore {
                    app.restore_session(session, cx);
                }
                if let Some(path) = launch_path.as_deref() {
                    app.open_path(path, cx);
                }
                app
            })
        },
    )
    .expect("open Mdow window")
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::Keystroke;
    use std::ffi::OsString;

    #[test]
    fn finder_urls_decode_spaces_unicode_and_ignore_non_file_schemes() {
        assert_eq!(
            local_file_paths(vec![
                "file:///tmp/hello%20world.md".into(),
                "file:///tmp/caf%C3%A9.mdx".into(),
                "https://example.com/test.md".into(),
                "invalid".into(),
            ]),
            vec![
                PathBuf::from("/tmp/hello world.md"),
                PathBuf::from("/tmp/café.mdx")
            ]
        );
    }

    #[test]
    fn launch_path_is_the_first_non_flag_argument_only() {
        let args = launch_args([
            OsString::from("mdow"),
            OsString::from("--verify"),
            OsString::from("first.md"),
            OsString::from("second.md"),
        ]);

        assert_eq!(args.document_path, Some(PathBuf::from("first.md")));
        assert_eq!(
            launch_args([OsString::from("mdow"), OsString::from("--verify")]).document_path,
            None
        );
    }

    #[test]
    fn verify_assets_flag_is_not_treated_as_a_document_path() {
        let args = launch_args(["MdowNative", "--verify-assets"]);

        assert!(args.verify_assets);
        assert_eq!(args.document_path, None);
    }

    #[test]
    fn default_window_title_names_the_native_app() {
        assert_eq!(default_window_title(), "Mdow Native");
    }

    #[test]
    fn tab_navigation_keys_match_the_electron_shortcuts() {
        let bindings = key_bindings();
        let action_for = |keys: &str| {
            bindings
                .iter()
                .find(|binding| {
                    let wanted = Keystroke::parse(keys).unwrap();
                    matches!(binding.keystrokes(), [only]
                        if only.inner().key == wanted.key
                            && only.inner().modifiers == wanted.modifiers)
                })
                .unwrap_or_else(|| panic!("{keys} should be bound"))
                .action()
                .as_any()
        };

        assert!(action_for("cmd-alt-right").is::<NextTab>());
        assert!(action_for("cmd-alt-left").is::<PreviousTab>());
        assert!(action_for("ctrl-tab").is::<NextTab>());
        assert!(action_for("ctrl-shift-tab").is::<PreviousTab>());
        assert!(action_for("cmd-1").is::<SelectTab1>());
        assert!(action_for("cmd-8").is::<SelectTab8>());
        assert!(action_for("cmd-9").is::<SelectLastTab>());
        assert!(action_for("cmd-h").is::<Hide>());
        assert!(action_for("alt-cmd-h").is::<HideOthers>());
        assert!(action_for("cmd-m").is::<Minimize>());
        assert!(action_for("ctrl-cmd-f").is::<ToggleFullScreen>());
    }
}
