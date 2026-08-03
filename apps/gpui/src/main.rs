use gpui::{
    App, AppContext, Application, Bounds, KeyBinding, Menu, MenuItem, SystemMenuType,
    TitlebarOptions, WindowBounds, WindowOptions, point, px, size,
};
use mdow_gpui::{
    actions::{CloseTab, OpenFile, OpenFolder, Quit, ToggleSidebar, ToggleWideMode},
    app::MdowApp,
    assets::MdowAssets,
};
use std::{borrow::Cow, ffi::OsString, path::PathBuf};

fn launch_path_from_args(args: impl IntoIterator<Item = OsString>) -> Option<PathBuf> {
    args.into_iter()
        .skip(1)
        .find(|argument| !argument.to_string_lossy().starts_with('-'))
        .map(PathBuf::from)
}

fn app_menus() -> Vec<Menu> {
    vec![
        Menu {
            name: "Mdow".into(),
            items: vec![
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action("Quit Mdow", Quit),
            ],
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("Open File…", OpenFile),
                MenuItem::action("Open Folder…", OpenFolder),
                MenuItem::separator(),
                MenuItem::action("Close Tab", CloseTab),
            ],
        },
    ]
}

fn main() {
    let asset_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    let launch_path = launch_path_from_args(std::env::args_os());
    Application::new()
        .with_assets(MdowAssets::new(asset_root.clone()))
        .run(move |cx: &mut App| {
            cx.text_system()
                .add_fonts(vec![
                    Cow::Owned(
                        std::fs::read(asset_root.join("fonts/InterVariable.ttf"))
                            .expect("read bundled Inter font"),
                    ),
                    Cow::Owned(
                        std::fs::read(asset_root.join("fonts/GeistMono-Variable.ttf"))
                            .expect("read bundled Geist Mono font"),
                    ),
                ])
                .expect("register bundled Mdow fonts");

            cx.bind_keys([
                KeyBinding::new("cmd-o", OpenFile, None),
                KeyBinding::new("cmd-shift-o", OpenFolder, None),
                KeyBinding::new("cmd-b", ToggleSidebar, None),
                KeyBinding::new("cmd-w", CloseTab, None),
                KeyBinding::new("cmd-shift-w", ToggleWideMode, None),
                KeyBinding::new("cmd-q", Quit, None),
            ]);
            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.set_menus(app_menus());

            let bounds = Bounds::centered(None, size(px(1120.0), px(760.0)), cx);
            cx.open_window(
                WindowOptions {
                    titlebar: Some(TitlebarOptions {
                        title: Some("Mdow".into()),
                        appears_transparent: true,
                        traffic_light_position: Some(point(px(14.0), px(14.0))),
                    }),
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, cx| {
                    cx.new(|cx| {
                        let mut app = MdowApp::new(window, cx);
                        if let Some(path) = launch_path.as_deref() {
                            app.open_path(path, cx);
                        }
                        app
                    })
                },
            )
            .expect("open Mdow window");
            cx.activate(true);
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::OwnedMenuItem;
    use std::ffi::OsString;

    #[test]
    fn launch_path_is_the_first_non_flag_argument_only() {
        let path = launch_path_from_args([
            OsString::from("mdow"),
            OsString::from("--verify"),
            OsString::from("first.md"),
            OsString::from("second.md"),
        ]);

        assert_eq!(path, Some(PathBuf::from("first.md")));
        assert_eq!(
            launch_path_from_args([OsString::from("mdow"), OsString::from("--verify")]),
            None
        );
    }

    #[test]
    fn native_menus_keep_quit_once_and_file_actions_in_a_distinct_file_menu() {
        let menus = app_menus().into_iter().map(Menu::owned).collect::<Vec<_>>();
        let names = menus
            .iter()
            .map(|menu| menu.name.as_ref())
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["Mdow", "File"]);
        assert!(matches!(
            &menus[0].items[0],
            OwnedMenuItem::SystemMenu(menu) if menu.name.as_ref() == "Services"
        ));
        assert!(matches!(menus[0].items[1], OwnedMenuItem::Separator));
        assert!(matches!(
            &menus[0].items[2],
            OwnedMenuItem::Action { name, action, .. }
                if name == "Quit Mdow" && action.as_any().is::<Quit>()
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
        assert_eq!(file_actions.len(), 3);
        assert_eq!(file_actions[0].0, "Open File…");
        assert!(file_actions[0].1.as_any().is::<OpenFile>());
        assert_eq!(file_actions[1].0, "Open Folder…");
        assert!(file_actions[1].1.as_any().is::<OpenFolder>());
        assert_eq!(file_actions[2].0, "Close Tab");
        assert!(file_actions[2].1.as_any().is::<CloseTab>());

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
    }
}
