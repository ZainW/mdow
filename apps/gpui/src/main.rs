use anyhow::Context;
use gpui::{
    App, AppContext, Application, Bounds, KeyBinding, Menu, MenuItem, SystemMenuType,
    TitlebarOptions, WindowBounds, WindowHandle, WindowOptions, point, px, size,
};
use mdow_gpui::{
    actions::{
        CloseTab, Dismiss, FindNext, FindPrevious, NewWindow, OpenFile, OpenFolder, Quit,
        SidebarFolder, SidebarOutline, SidebarRecents, ToggleFind, TogglePalette, ToggleSettings,
        ToggleShortcuts, ToggleSidebar, ToggleWideMode, ZoomIn, ZoomOut, ZoomReset,
    },
    app::MdowApp,
    assets::{BUNDLED_FONTS, MdowAssets, discover_asset_root, validate_required_assets},
    overlay,
    persist::{Restored, SessionRole, StateStore},
    theme::TrafficLights,
    ui::field,
};
use std::{borrow::Cow, ffi::OsString, path::PathBuf};

enum WindowSeed {
    RestoreSession,
    Blank,
    RestoreSessionThenOpen(Option<PathBuf>),
}

struct LaunchArgs {
    verify_assets: bool,
    document_path: Option<PathBuf>,
}

fn launch_args<T>(args: impl IntoIterator<Item = T>) -> LaunchArgs
where
    T: Into<OsString>,
{
    let mut verify_assets = false;
    let mut document_path = None;

    for argument in args.into_iter().skip(1).map(Into::into) {
        if argument == "--verify-assets" {
            verify_assets = true;
        } else if document_path.is_none() && !argument.to_string_lossy().starts_with('-') {
            document_path = Some(PathBuf::from(argument));
        }
    }

    LaunchArgs {
        verify_assets,
        document_path,
    }
}

fn default_window_title() -> &'static str {
    "Mdow Native"
}

fn app_menus() -> Vec<Menu> {
    vec![
        Menu {
            name: "Mdow Native".into(),
            items: vec![
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action("Quit Mdow Native", Quit),
            ],
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("New Window", NewWindow),
                MenuItem::action("Open File…", OpenFile),
                MenuItem::action("Open Folder…", OpenFolder),
                MenuItem::separator(),
                MenuItem::action("Close Tab", CloseTab),
            ],
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::action("Find…", ToggleFind),
                MenuItem::action("Find Next", FindNext),
                MenuItem::action("Find Previous", FindPrevious),
            ],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Toggle Sidebar", ToggleSidebar),
                MenuItem::action("Recents", SidebarRecents),
                MenuItem::action("Folder", SidebarFolder),
                MenuItem::action("Outline", SidebarOutline),
                MenuItem::separator(),
                MenuItem::action("Toggle Wide Mode", ToggleWideMode),
                MenuItem::action("Zoom In", ZoomIn),
                MenuItem::action("Zoom Out", ZoomOut),
                MenuItem::action("Actual Size", ZoomReset),
                MenuItem::separator(),
                MenuItem::action("Command Palette", TogglePalette),
                MenuItem::action("Settings…", ToggleSettings),
                MenuItem::action("Keyboard Shortcuts", ToggleShortcuts),
            ],
        },
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

        cx.bind_keys([
            KeyBinding::new("cmd-n", NewWindow, None),
            KeyBinding::new("cmd-o", OpenFile, None),
            KeyBinding::new("cmd-shift-o", OpenFolder, None),
            KeyBinding::new("cmd-b", ToggleSidebar, None),
            KeyBinding::new("cmd-w", CloseTab, None),
            KeyBinding::new("cmd-shift-w", ToggleWideMode, None),
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
        ]);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.on_action(|_: &NewWindow, cx| {
            open_main_window(WindowSeed::Blank, cx);
        });
        cx.set_menus(app_menus());

        let _primary = open_main_window(WindowSeed::RestoreSessionThenOpen(launch_path), cx);
        cx.activate(true);
    });
    Ok(())
}

fn open_main_window(seed: WindowSeed, cx: &mut App) -> WindowHandle<MdowApp> {
    let store = StateStore::open_default();
    let Restored { prefs, session } = store.load();
    let (restore, launch_path, role) = match seed {
        WindowSeed::RestoreSession => (true, None, SessionRole::Owner),
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
    use gpui::OwnedMenuItem;
    use std::ffi::OsString;

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
    fn native_menus_keep_quit_once_and_file_actions_in_a_distinct_file_menu() {
        let menus = app_menus().into_iter().map(Menu::owned).collect::<Vec<_>>();
        let names = menus
            .iter()
            .map(|menu| menu.name.as_ref())
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["Mdow Native", "File", "Edit", "View"]);
        assert!(matches!(
            &menus[0].items[0],
            OwnedMenuItem::SystemMenu(menu) if menu.name.as_ref() == "Services"
        ));
        assert!(matches!(menus[0].items[1], OwnedMenuItem::Separator));
        assert!(matches!(
            &menus[0].items[2],
            OwnedMenuItem::Action { name, action, .. }
                if name == "Quit Mdow Native" && action.as_any().is::<Quit>()
        ));

        let file_actions = menus[1]
            .items
            .iter()
            .filter_map(|item| match item {
                OwnedMenuItem::Action { name, action, .. } => {
                    Some((name.as_str(), action.as_ref()))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(file_actions.len(), 4);
        assert_eq!(file_actions[0].0, "New Window");
        assert!(file_actions[0].1.as_any().is::<NewWindow>());
        assert_eq!(file_actions[1].0, "Open File…");
        assert!(file_actions[1].1.as_any().is::<OpenFile>());
        assert_eq!(file_actions[2].0, "Open Folder…");
        assert!(file_actions[2].1.as_any().is::<OpenFolder>());
        assert_eq!(file_actions[3].0, "Close Tab");
        assert!(file_actions[3].1.as_any().is::<CloseTab>());

        let quit_count = menus
            .iter()
            .flat_map(|menu| &menu.items)
            .filter(|item| {
                matches!(
                    item,
                    OwnedMenuItem::Action { action, .. } if action.as_any().is::<Quit>()
                )
            })
            .count();
        assert_eq!(quit_count, 1);
        assert_eq!(default_window_title(), "Mdow Native");
    }
}
