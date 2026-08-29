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
        SidebarOutline
    ]
);
