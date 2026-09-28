//! The native menu bar. Rebuilt whenever the active window's recents change so
//! File > Open Recent stays current.

#[cfg(target_os = "macos")]
use crate::actions::CheckForUpdates;
use crate::{
    actions::{
        About, BringAllToFront, ClearRecents, CloseTab, FindNext, FindPrevious, Hide, HideOthers,
        Minimize, NewWindow, NextTab, OpenFile, OpenFolder, OpenRecent, OpenWebsite, PreviousTab,
        Quit, Redo, ShowAll, SidebarFolder, SidebarOutline, SidebarRecents, ToggleFind,
        ToggleFullScreen, TogglePalette, ToggleSettings, ToggleShortcuts, ToggleSidebar,
        ToggleWideMode, Undo, Zoom, ZoomIn, ZoomOut, ZoomReset,
    },
    ui::field,
};
use gpui::{Menu, MenuItem, OsAction, SystemMenuType};
use std::path::{Path, PathBuf};

pub const APP_MENU_NAME: &str = "Mdow Native";
pub const WEBSITE_URL: &str = "https://mdow.wania.app";
/// Open Recent mirrors the macOS default of ten entries.
pub const RECENT_MENU_LIMIT: usize = 10;

/// "name.md — parent" so two README.md files stay distinguishable.
pub fn recent_menu_label(path: &Path) -> String {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());
    match path
        .parent()
        .and_then(Path::file_name)
        .map(|parent| parent.to_string_lossy().into_owned())
    {
        Some(parent) => format!("{name} — {parent}"),
        None => name,
    }
}

fn open_recent_menu(recents: &[PathBuf]) -> Menu {
    let mut items = recents
        .iter()
        .take(RECENT_MENU_LIMIT)
        .map(|path| {
            MenuItem::action(
                recent_menu_label(path),
                OpenRecent {
                    path: path.to_owned(),
                },
            )
        })
        .collect::<Vec<_>>();
    if !items.is_empty() {
        items.push(MenuItem::separator());
    }
    items.push(MenuItem::action("Clear Menu", ClearRecents));
    Menu {
        name: "Open Recent".into(),
        items,
    }
}

pub fn app_menus(recents: &[PathBuf]) -> Vec<Menu> {
    let mut app_items = vec![
        MenuItem::action(format!("About {APP_MENU_NAME}"), About),
        MenuItem::separator(),
    ];
    #[cfg(target_os = "macos")]
    {
        app_items.push(MenuItem::action("Check for Updates…", CheckForUpdates));
        app_items.push(MenuItem::separator());
    }
    app_items.extend([
        MenuItem::action("Settings…", ToggleSettings),
        MenuItem::separator(),
        MenuItem::os_submenu("Services", SystemMenuType::Services),
        MenuItem::separator(),
        MenuItem::action(format!("Hide {APP_MENU_NAME}"), Hide),
        MenuItem::action("Hide Others", HideOthers),
        MenuItem::action("Show All", ShowAll),
        MenuItem::separator(),
        MenuItem::action(format!("Quit {APP_MENU_NAME}"), Quit),
    ]);
    vec![
        Menu {
            name: APP_MENU_NAME.into(),
            items: app_items,
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("New Window", NewWindow),
                MenuItem::action("Open File…", OpenFile),
                MenuItem::action("Open Folder…", OpenFolder),
                MenuItem::submenu(open_recent_menu(recents)),
                MenuItem::separator(),
                MenuItem::action("Close Tab", CloseTab),
            ],
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::os_action("Undo", Undo, OsAction::Undo),
                MenuItem::os_action("Redo", Redo, OsAction::Redo),
                MenuItem::separator(),
                MenuItem::os_action("Cut", field::Cut, OsAction::Cut),
                MenuItem::os_action("Copy", field::Copy, OsAction::Copy),
                MenuItem::os_action("Paste", field::Paste, OsAction::Paste),
                MenuItem::os_action("Select All", field::SelectAll, OsAction::SelectAll),
                MenuItem::separator(),
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
                MenuItem::action("Next Tab", NextTab),
                MenuItem::action("Previous Tab", PreviousTab),
                MenuItem::separator(),
                MenuItem::action("Toggle Wide Mode", ToggleWideMode),
                MenuItem::action("Zoom In", ZoomIn),
                MenuItem::action("Zoom Out", ZoomOut),
                MenuItem::action("Actual Size", ZoomReset),
                MenuItem::separator(),
                MenuItem::action("Command Palette", TogglePalette),
                MenuItem::action("Keyboard Shortcuts", ToggleShortcuts),
                MenuItem::separator(),
                MenuItem::action("Toggle Full Screen", ToggleFullScreen),
            ],
        },
        Menu {
            name: "Window".into(),
            items: vec![
                MenuItem::action("Minimize", Minimize),
                MenuItem::action("Zoom", Zoom),
                MenuItem::separator(),
                MenuItem::action("Bring All to Front", BringAllToFront),
            ],
        },
        Menu {
            name: "Help".into(),
            items: vec![
                MenuItem::action("Mdow Website", OpenWebsite),
                MenuItem::action("Keyboard Shortcuts", ToggleShortcuts),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Action, OwnedMenuItem};

    fn action_names(menu: &gpui::OwnedMenu) -> Vec<&str> {
        menu.items
            .iter()
            .filter_map(|item| match item {
                OwnedMenuItem::Action { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect()
    }

    fn find_action<'a>(menu: &'a gpui::OwnedMenu, wanted: &str) -> &'a dyn Action {
        menu.items
            .iter()
            .find_map(|item| match item {
                OwnedMenuItem::Action { name, action, .. } if name == wanted => {
                    Some(action.as_ref())
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("{wanted} should be in {}", menu.name))
    }

    #[test]
    fn menu_bar_has_the_standard_mac_menus() {
        let menus = app_menus(&[])
            .into_iter()
            .map(Menu::owned)
            .collect::<Vec<_>>();
        let names = menus
            .iter()
            .map(|menu| menu.name.as_ref())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![APP_MENU_NAME, "File", "Edit", "View", "Window", "Help"]
        );

        let app = &menus[0];
        assert!(find_action(app, "About Mdow Native").as_any().is::<About>());
        assert!(find_action(app, "Hide Mdow Native").as_any().is::<Hide>());
        assert!(find_action(app, "Hide Others").as_any().is::<HideOthers>());
        assert!(find_action(app, "Show All").as_any().is::<ShowAll>());
        assert!(app.items.iter().any(|item| matches!(
            item,
            OwnedMenuItem::SystemMenu(menu) if menu.name.as_ref() == "Services"
        )));

        let edit = &menus[2];
        assert_eq!(
            action_names(edit),
            vec![
                "Undo",
                "Redo",
                "Cut",
                "Copy",
                "Paste",
                "Select All",
                "Find…",
                "Find Next",
                "Find Previous"
            ]
        );
        assert!(edit.items.iter().any(|item| matches!(
            item,
            OwnedMenuItem::Action { name, os_action: Some(OsAction::Paste), .. } if name == "Paste"
        )));

        assert!(
            find_action(&menus[3], "Toggle Full Screen")
                .as_any()
                .is::<ToggleFullScreen>()
        );
        let window = &menus[4];
        assert!(find_action(window, "Minimize").as_any().is::<Minimize>());
        assert!(find_action(window, "Zoom").as_any().is::<Zoom>());
        assert!(
            find_action(window, "Bring All to Front")
                .as_any()
                .is::<BringAllToFront>()
        );
        assert!(
            find_action(&menus[5], "Mdow Website")
                .as_any()
                .is::<OpenWebsite>()
        );

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

    fn open_recent(menus: &[gpui::OwnedMenu]) -> &gpui::OwnedMenu {
        menus[1]
            .items
            .iter()
            .find_map(|item| match item {
                OwnedMenuItem::Submenu(menu) if menu.name.as_ref() == "Open Recent" => Some(menu),
                _ => None,
            })
            .expect("File > Open Recent")
    }

    #[test]
    fn open_recent_lists_recents_then_clear_menu() {
        let recents = (0..12)
            .map(|index| PathBuf::from(format!("/notes/dir{index}/file{index}.md")))
            .collect::<Vec<_>>();
        let menus = app_menus(&recents)
            .into_iter()
            .map(Menu::owned)
            .collect::<Vec<_>>();
        let submenu = open_recent(&menus);
        let names = action_names(submenu);

        assert_eq!(names.len(), RECENT_MENU_LIMIT + 1);
        assert_eq!(names[0], "file0.md — dir0");
        assert_eq!(*names.last().unwrap(), "Clear Menu");
        let first = find_action(submenu, "file0.md — dir0");
        assert_eq!(
            first.as_any().downcast_ref::<OpenRecent>(),
            Some(&OpenRecent {
                path: recents[0].clone()
            })
        );
        assert!(
            find_action(submenu, "Clear Menu")
                .as_any()
                .is::<ClearRecents>()
        );

        let empty = app_menus(&[])
            .into_iter()
            .map(Menu::owned)
            .collect::<Vec<_>>();
        assert_eq!(action_names(open_recent(&empty)), vec!["Clear Menu"]);
    }
}
