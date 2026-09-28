use gpui::actions;

actions!(
    mdow,
    [
        NewWindow,
        OpenFile,
        OpenFolder,
        ToggleSidebar,
        CloseTab,
        ToggleWideMode,
        Quit,
        ToggleFind,
        TogglePalette,
        ToggleSettings,
        ToggleShortcuts,
        Dismiss,
        FindNext,
        FindPrevious,
        ZoomIn,
        ZoomOut,
        ZoomReset,
        SidebarRecents,
        SidebarFolder,
        SidebarOutline,
        CheckForUpdates,
        NextTab,
        PreviousTab,
        SelectTab1,
        SelectTab2,
        SelectTab3,
        SelectTab4,
        SelectTab5,
        SelectTab6,
        SelectTab7,
        SelectTab8,
        SelectLastTab,
        ClearRecents,
        About,
        Hide,
        HideOthers,
        ShowAll,
        Minimize,
        Zoom,
        ToggleFullScreen,
        BringAllToFront,
        OpenWebsite,
        Undo,
        Redo
    ]
);

/// File > Open Recent entry.
#[derive(Clone, Debug, PartialEq, gpui::Action)]
#[action(namespace = mdow, no_json)]
pub struct OpenRecent {
    pub path: std::path::PathBuf,
}
